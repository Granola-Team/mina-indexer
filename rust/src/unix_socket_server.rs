use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{
        precomputed::{PcbVersion, PrecomputedBlockWithCanonicity},
        store::BlockStore,
        BlockWithoutHeight,
    },
    canonicity::store::CanonicityStore,
    client::*,
    command::{
        internal::store::InternalCommandStore, signed::TxnHash, store::UserCommandStore, Command,
    },
    constants::{HARDFORK_GENESIS_HASH, MAINNET_GENESIS_HASH},
    ledger::{
        staking::AggregatedEpochStakeDelegation,
        store::{best::BestLedgerStore, staged::StagedLedgerStore, staking::StakingLedgerStore},
        token::TokenAddress,
        Ledger, LedgerHash,
    },
    snark_work::store::SnarkStore,
    state::{summary::SummaryShort, IndexerState},
    store::version::VersionStore,
};
use anyhow::{bail, Context};
use bincode::{Decode, Encode};
use log::{debug, error, info, trace, warn};
use std::{
    io::{self, ErrorKind},
    path::Path,
    sync::Arc,
};
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::RwLock,
};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

/// Create Unix Domain Socket listener
pub fn create_socket_listener(domain_socket_path: &Path) -> UnixListener {
    let listener = UnixListener::bind(domain_socket_path)
        .or_else(|e| try_replace_old_socket(e, domain_socket_path))
        .unwrap_or_else(|e| {
            panic!("Unable to bind to Unix domain socket file {domain_socket_path:#?} due to {e}")
        });
    info!("Created Unix domain socket server at {domain_socket_path:#?}");
    listener
}

async fn parse_conn_to_cli(stream: &UnixStream) -> anyhow::Result<ClientCli> {
    loop {
        stream.readable().await?;

        let mut buffer = Vec::with_capacity(BUFFER_SIZE);
        match stream.try_read_buf(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                buffer.truncate(n);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
        let (command, _): (ClientCli, usize) =
            bincode::decode_from_slice(&buffer, BIN_CODE_CONFIG)?;
        return Ok(command);
    }
    bail!("Unexpected Unix domain socket read error");
}

#[derive(Debug, Encode, Decode)]
pub enum ServerCliResponse {
    Success(String),
    Error(String),
}

#[allow(clippy::just_underscores_and_digits)]
#[allow(clippy::too_many_lines)]
pub async fn handle_connection(
    listener: UnixListener,
    state: Arc<RwLock<IndexerState>>,
    subsys: SubsystemHandle,
) -> anyhow::Result<()> {
    use helpers::*;

    loop {
        let (connection, _) = match listener.accept().cancel_on_shutdown(&subsys).await {
            Ok(connection) => connection,
            Err(_) => break,
        }?;

        let command = parse_conn_to_cli(&connection).await?;
        let (_, mut writer) = connection.into_split();

        let state = state.read().await;
        let db = if let Some(store) = state.indexer_store.as_ref() {
            store
        } else {
            let response =
                ServerCliResponse::Error("Unable to get a handle on indexer store...".to_string());
            let encoded = bincode::encode_to_vec(&response, BIN_CODE_CONFIG)?;
            writer.write_all(&encoded).await?;
            continue;
        };

        let response = match command {
            ClientCli::Accounts(__) => match __ {
                Accounts::PublicKey { public_key: pk } => {
                    info!("Received account command for {pk}");
                    if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let pk: PublicKey = pk.into();
                        if let Some(account) = db.get_best_account(&pk, &TokenAddress::default())? {
                            info!("Writing account {pk} to client");
                            ServerCliResponse::Success(format!("{account}"))
                        } else {
                            account_missing_from_db(&pk)
                        }
                    }
                }
            },
            ClientCli::Blocks(__) => match __ {
                Blocks::Best { verbose, path } => {
                    info!("Received best block command");
                    if let Some(best_tip) = db.get_best_block()? {
                        let block_str = if let Some(canonicity) =
                            db.get_block_canonicity(&best_tip.state_hash())?
                        {
                            if verbose {
                                serde_json::to_string_pretty(&best_tip.with_canonicity(canonicity))?
                            } else {
                                let block =
                                    BlockWithoutHeight::with_canonicity(&best_tip, canonicity);
                                serde_json::to_string_pretty(&block)?
                            }
                        } else {
                            block_missing_from_db(&best_tip.state_hash().0)
                        };

                        if path.is_some() {
                            let path = &path.unwrap();
                            info!("Writing best tip block to {path:?}");
                            std::fs::write(path, block_str)?;
                            ServerCliResponse::Success(format!("Best block written to {path:?}"))
                        } else {
                            info!("Writing best tip block to stdout");
                            ServerCliResponse::Success(block_str)
                        }
                    } else {
                        best_tip_missing_from_db()
                    }
                }
                Blocks::StateHash {
                    state_hash,
                    verbose,
                    path,
                } => {
                    info!("Received block-state-hash command");
                    if !StateHash::is_valid(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else if let Ok(Some((ref block, _))) =
                        db.get_block(&state_hash.clone().into())
                    {
                        let block_str = if let Some(canonicity) =
                            db.get_block_canonicity(&block.state_hash())?
                        {
                            if verbose {
                                serde_json::to_string_pretty(&block.with_canonicity(canonicity))?
                            } else {
                                let block = BlockWithoutHeight::with_canonicity(block, canonicity);
                                serde_json::to_string_pretty(&block)?
                            }
                        } else {
                            block_missing_from_db(&block.state_hash().0)
                        };
                        if path.is_some() {
                            let path = &path.unwrap();
                            info!("Writing block {state_hash} to {path:?}");
                            std::fs::write(path, block_str)?;
                            ServerCliResponse::Success(format!(
                                "Block {} written to {:?}",
                                block.state_hash().0,
                                path
                            ))
                        } else {
                            info!("Writing block to stdout {}", block.summary());
                            ServerCliResponse::Success(block_str)
                        }
                    } else {
                        ServerCliResponse::Error(format!(
                            "Block at state hash not present in store: {state_hash}"
                        ))
                    }
                }
                Blocks::Height {
                    height,
                    verbose,
                    path,
                } => {
                    info!("Received blocks-at-height {height} command");
                    let blocks_at_height = db.get_blocks_at_height(height)?;
                    let blocks_str = if verbose {
                        let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_height
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(block.with_canonicity(canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        serde_json::to_string(&blocks)?
                    } else {
                        let blocks: Vec<BlockWithoutHeight> = blocks_at_height
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(BlockWithoutHeight::with_canonicity(&block, canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        format_vec_jq_compatible(&blocks)
                    };

                    if path.is_none() {
                        info!("Writing blocks at height {height} to stdout");
                        ServerCliResponse::Success(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing blocks at height {height} to {path:?}");
                            std::fs::write(path.clone(), blocks_str)?;
                            ServerCliResponse::Success(format!(
                                "Blocks at height {height} written to {path:?}"
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
                Blocks::GlobalSlot {
                    slot,
                    verbose,
                    path,
                } => {
                    info!("Received blocks-at-slot {slot} command");
                    let slot: u32 = slot.parse()?;
                    let blocks_at_slot = db.get_blocks_at_slot(slot)?;
                    let blocks_str = if verbose {
                        let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_slot
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(block.with_canonicity(canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        serde_json::to_string(&blocks)?
                    } else {
                        let blocks: Vec<BlockWithoutHeight> = blocks_at_slot
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(BlockWithoutHeight::with_canonicity(&block, canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        format_vec_jq_compatible(&blocks)
                    };

                    if path.is_none() {
                        info!("Writing blocks at slot {slot} to stdout");
                        ServerCliResponse::Success(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing blocks at slot {slot} to {path:?}");

                            std::fs::write(path.clone(), blocks_str)?;
                            ServerCliResponse::Success(format!(
                                "Blocks at slot {slot} written to {path:?}"
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
                Blocks::PublicKey {
                    public_key: pk,
                    verbose,
                    path,
                } => {
                    info!("Received blocks-at-public-key command {pk}");
                    if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let blocks_at_pk = db.get_blocks_at_public_key(&pk.clone().into())?;
                        let blocks_str = if verbose {
                            let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_pk
                                .iter()
                                .flat_map(|state_hash| {
                                    if let Ok(Some(canonicity)) =
                                        db.get_block_canonicity(state_hash)
                                    {
                                        let block = db
                                            .get_block(state_hash)
                                            .with_context(|| {
                                                format!("block missing from store {state_hash}")
                                            })
                                            .unwrap()
                                            .unwrap()
                                            .0;
                                        Some(block.with_canonicity(canonicity))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            serde_json::to_string(&blocks)?
                        } else {
                            let blocks: Vec<BlockWithoutHeight> = blocks_at_pk
                                .iter()
                                .flat_map(|state_hash| {
                                    if let Ok(Some(canonicity)) =
                                        db.get_block_canonicity(state_hash)
                                    {
                                        let block = db
                                            .get_block(state_hash)
                                            .with_context(|| {
                                                format!("block missing from store {state_hash}")
                                            })
                                            .unwrap()
                                            .unwrap()
                                            .0;
                                        Some(BlockWithoutHeight::with_canonicity(
                                            &block, canonicity,
                                        ))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            format_vec_jq_compatible(&blocks)
                        };

                        if path.is_none() {
                            info!("Writing blocks at public key {pk} to stdout");
                            ServerCliResponse::Success(blocks_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                info!("Writing blocks at public key {pk} to {path:?}");

                                std::fs::write(path.clone(), blocks_str)?;
                                ServerCliResponse::Success(format!(
                                    "Blocks at public key {pk} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
                Blocks::Children {
                    state_hash,
                    verbose,
                    path,
                } => {
                    info!("Received block-children command for block {state_hash}");
                    let children = db.get_block_children(&state_hash.clone().into())?;
                    let blocks_str = if verbose {
                        let blocks: Vec<PrecomputedBlockWithCanonicity> = children
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(block.with_canonicity(canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        serde_json::to_string(&blocks)?
                    } else {
                        let blocks: Vec<BlockWithoutHeight> = children
                            .iter()
                            .flat_map(|state_hash| {
                                if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                                    let block = db
                                        .get_block(state_hash)
                                        .with_context(|| {
                                            format!("block missing from store {state_hash}")
                                        })
                                        .unwrap()
                                        .unwrap()
                                        .0;
                                    Some(BlockWithoutHeight::with_canonicity(&block, canonicity))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        format_vec_jq_compatible(&blocks)
                    };

                    if path.is_none() {
                        info!("Writing children of block {state_hash} to stdout");
                        ServerCliResponse::Success(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing children of block {state_hash} to {path:?}");
                            std::fs::write(path.clone(), blocks_str)?;
                            ServerCliResponse::Success(format!(
                                "Children of block {state_hash} written to {path:?}"
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
            },
            ClientCli::Chain(__) => match __ {
                Chain::Best {
                    num,
                    verbose,
                    start_state_hash,
                    end_state_hash,
                    path,
                } => {
                    info!("Received best-chain command");
                    let start_state_hash: StateHash = match start_state_hash {
                        None => {
                            if let Ok(Some(PcbVersion::V2)) = db.get_best_block_version() {
                                HARDFORK_GENESIS_HASH.into()
                            } else {
                                MAINNET_GENESIS_HASH.into()
                            }
                        }
                        Some(start_state_hash) => start_state_hash.into(),
                    };

                    if let Some(best_tip) = db.get_best_block()? {
                        let end_state_hash = {
                            match end_state_hash {
                                None => best_tip.state_hash(),
                                Some(end_state_hash) => {
                                    if !StateHash::is_valid(&end_state_hash) {
                                        best_tip.state_hash()
                                    } else {
                                        end_state_hash.into()
                                    }
                                }
                            }
                        };

                        if !StateHash::is_valid(&start_state_hash.0) {
                            invalid_state_hash(&start_state_hash.0)
                        } else if let (Some((end_block, _)), Some((start_block, _))) = (
                            db.get_block(&end_state_hash)?,
                            db.get_block(&start_state_hash)?,
                        ) {
                            let start_height = start_block.blockchain_length();
                            let end_height = end_block.blockchain_length();
                            let mut parent_hash = end_block.previous_state_hash();
                            let mut best_chain = vec![end_block];

                            // constrain by num and state hash bound
                            for _ in 1..num.min(end_height.saturating_sub(start_height) + 1) {
                                if let Some((parent_pcb, _)) = db.get_block(&parent_hash)? {
                                    let curr_hash: StateHash = parent_pcb.state_hash();
                                    parent_hash = parent_pcb.previous_state_hash();
                                    best_chain.push(parent_pcb);

                                    if curr_hash == start_state_hash {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }

                            let best_chain_str = if verbose {
                                let best_chain: Vec<PrecomputedBlockWithCanonicity> = best_chain
                                    .iter()
                                    .flat_map(|block| {
                                        if let Ok(Some(canonicity)) =
                                            db.get_block_canonicity(&block.state_hash())
                                        {
                                            Some(block.with_canonicity(canonicity))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                serde_json::to_string(&best_chain)?
                            } else {
                                let best_chain: Vec<BlockWithoutHeight> = best_chain
                                    .iter()
                                    .flat_map(|block| {
                                        if let Ok(Some(canonicity)) =
                                            db.get_block_canonicity(&block.state_hash())
                                        {
                                            Some(BlockWithoutHeight::with_canonicity(
                                                block, canonicity,
                                            ))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                format_vec_jq_compatible(&best_chain)
                            };

                            if path.is_none() {
                                info!("Writing best chain to stdout");
                                ServerCliResponse::Success(best_chain_str)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    info!("Writing best chain to {path:?}");

                                    std::fs::write(path.clone(), best_chain_str)?;
                                    ServerCliResponse::Success(format!(
                                        "Best chain written to {path:?}"
                                    ))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        } else {
                            ServerCliResponse::Success("No results".to_string())
                        }
                    } else {
                        best_tip_missing_from_db()
                    }
                }
            },
            ClientCli::CreateSnapshot { output_path } => {
                info!("Received create-snapshot command");
                match db.create_snapshot(&output_path) {
                    Err(e) => ServerCliResponse::Error(e.to_string()),
                    Ok(s) => ServerCliResponse::Success(s),
                }
            }
            ClientCli::Ledgers(__) => match __ {
                Ledgers::Best { path, memoize } => {
                    info!("Received best-ledger command");
                    if let Some(ledger) = db.get_best_ledger(memoize)? {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing best ledger to stdout");
                            ServerCliResponse::Success(ledger)
                        } else {
                            let path = path.unwrap();
                            if path.is_dir() {
                                file_must_not_be_a_directory(&path)
                            } else {
                                debug!("Writing best ledger to {path:?}");
                                std::fs::write(path.clone(), ledger)?;
                                ServerCliResponse::Success(format!(
                                    "Best ledger written to {path:?}"
                                ))
                            }
                        }
                    } else {
                        ServerCliResponse::Error("Best ledger cannot be calculated".to_string())
                    }
                }
                Ledgers::Hash {
                    hash,
                    path,
                    memoize,
                } => {
                    info!("Received staged ledger command for {hash}");
                    fn write_ledger(
                        path: Option<std::path::PathBuf>,
                        ledger: Ledger,
                        hash: &str,
                    ) -> ServerCliResponse {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing staged ledger at hash {hash} to stdout");
                            ServerCliResponse::Success(ledger)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing staged ledger at {hash} to {path:?}");
                                std::fs::write(path.clone(), ledger).ok();
                                ServerCliResponse::Success(format!(
                                    "Ledger at hash {hash} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }

                    // check if ledger or state hash and use appropriate getter
                    if StateHash::is_valid(&hash) {
                        trace!("{hash} is a state hash");
                        if let Some(ledger) =
                            db.get_staged_ledger_at_state_hash(&hash.clone().into(), memoize)?
                        {
                            write_ledger(path, ledger, &hash)
                        } else {
                            ServerCliResponse::Error(format!(
                                "Ledger at state hash {hash} is not in the store"
                            ))
                        }
                    } else if LedgerHash::is_valid(&hash) {
                        trace!("{hash} is a ledger hash");
                        if let Some(ledger) = db.get_staged_ledger_at_ledger_hash(
                            &LedgerHash::new_or_panic(hash.clone()),
                            memoize,
                        )? {
                            write_ledger(path, ledger, &hash)
                        } else {
                            ServerCliResponse::Error(format!(
                                "Ledger at ledger hash {hash} is not in the store"
                            ))
                        }
                    } else {
                        ServerCliResponse::Error(format!("Invalid ledger or state hash: {hash}"))
                    }
                }
                Ledgers::Height {
                    height,
                    path,
                    memoize,
                } => {
                    info!("Received staged ledger at height {height} command");
                    if let Ok(Some(best_tip_height)) = db.get_best_block_height() {
                        if height > best_tip_height {
                            // ahead of witness tree - cannot compute
                            ServerCliResponse::Error(format!("Invalid query: ledger at height {height} cannot be determined from a chain of length {best_tip_height}"))
                        } else {
                            let ledger_str = db
                                .get_staged_ledger_at_block_height(height, memoize)?
                                .unwrap()
                                .to_string_pretty();
                            if path.is_none() {
                                debug!("Writing ledger at height {height} to stdout");
                                ServerCliResponse::Success(ledger_str)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    debug!("Writing ledger at height {height} to {path:?}");
                                    std::fs::write(&path, ledger_str)?;
                                    ServerCliResponse::Success(format!(
                                        "Ledger at height {height} written to {path:?}"
                                    ))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        }
                    } else {
                        best_tip_missing_from_db()
                    }
                }
            },
            ClientCli::StakingLedgers(__) => match __ {
                StakingLedgers::Hash { hash, path } => {
                    info!("Received staking-ledgers-hash command for {hash}");
                    if LedgerHash::is_valid(&hash) {
                        trace!("{hash} is a ledger hash");
                        if let Some(staking_ledger) =
                            db.get_staking_ledger(&hash.clone().into(), None, None)?
                        {
                            let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                            if path.is_none() {
                                debug!("Writing staking ledger at hash {hash} to stdout");
                                ServerCliResponse::Success(ledger_json)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    debug!("Writing ledger at {hash} to {path:?}");

                                    std::fs::write(path.clone(), ledger_json)?;
                                    ServerCliResponse::Success(format!(
                                        "Staking ledger at hash {hash} written to {path:?}"
                                    ))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        } else {
                            ServerCliResponse::Error(format!(
                                "Staking ledger at {hash} is not in the store"
                            ))
                        }
                    } else {
                        ServerCliResponse::Error(format!("Invalid ledger hash: {hash}"))
                    }
                }
                StakingLedgers::Epoch {
                    epoch,
                    genesis_state_hash,
                    path,
                } => {
                    info!("Received staking-ledgers-epoch {epoch} command");
                    if !StateHash::is_valid(&genesis_state_hash) {
                        invalid_state_hash(&genesis_state_hash)
                    } else if let Some(staking_ledger) =
                        db.build_staking_ledger(epoch, Some(&genesis_state_hash.into()))?
                    {
                        let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                        if path.is_none() {
                            debug!("Writing staking ledger at epoch {epoch} to stdout");
                            ServerCliResponse::Success(ledger_json)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at epoch {epoch} to {path:?}");
                                std::fs::write(path.clone(), ledger_json)?;
                                ServerCliResponse::Success(format!(
                                    "Staking ledger at epoch {epoch} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        ServerCliResponse::Error(format!(
                            "Staking ledger at epoch {epoch} is not in the store"
                        ))
                    }
                }
                StakingLedgers::PublicKey {
                    epoch,
                    genesis_state_hash,
                    public_key: pk,
                } => {
                    info!("Received staking ledger account command for pk {pk} epoch {epoch}");
                    if !StateHash::is_valid(&genesis_state_hash) {
                        invalid_state_hash(&genesis_state_hash)
                    } else if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else if let Some(aggregated_delegations) =
                        db.build_aggregated_delegations(epoch, Some(&genesis_state_hash.into()))?
                    {
                        let pk: PublicKey = pk.into();
                        let epoch = aggregated_delegations.epoch;
                        let network = aggregated_delegations.network;
                        let total_delegations = aggregated_delegations.total_delegations;
                        let count_delegates = aggregated_delegations
                            .delegations
                            .get(&pk)
                            .and_then(|agg_del| agg_del.count_delegates);
                        let total_delegated = aggregated_delegations
                            .delegations
                            .get(&pk)
                            .and_then(|agg_del| agg_del.total_delegated);
                        let delegates = aggregated_delegations
                            .delegations
                            .get(&pk)
                            .map_or(vec![], |agg_del| {
                                agg_del.delegates.iter().cloned().collect()
                            });
                        ServerCliResponse::Success(serde_json::to_string_pretty(
                            &AggregatedEpochStakeDelegation {
                                pk,
                                epoch,
                                network,
                                count_delegates,
                                total_delegated,
                                total_stake: total_delegations,
                                delegates,
                            },
                        )?)
                    } else {
                        ServerCliResponse::Error(format!(
                            "Public key {pk} is missing from staking ledger epoch {epoch}"
                        ))
                    }
                }
                StakingLedgers::Delegations {
                    epoch,
                    genesis_state_hash,
                    path,
                } => {
                    info!("Received staking-delegations command for epoch {epoch}");
                    let aggregated_delegations =
                        db.build_aggregated_delegations(epoch, Some(&genesis_state_hash.into()))?;
                    if let Some(agg_del_str) = aggregated_delegations
                        .map(|agg_del| serde_json::to_string_pretty(&agg_del).unwrap())
                    {
                        if path.is_none() {
                            debug!(
                                "Writing aggregated staking delegations epoch {epoch} to stdout"
                            );
                            ServerCliResponse::Success(agg_del_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing aggregated staking delegations epoch {epoch} to {path:?}");
                                std::fs::write(&path, agg_del_str)?;
                                ServerCliResponse::Success(format!(
                                    "Aggregated staking delegations epoch {epoch} written to {path:?}"))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        ServerCliResponse::Error(format!(
                            "Unable to aggregate staking delegations epoch {epoch}"
                        ))
                    }
                }
            },
            ClientCli::Snarks(__) => match __ {
                Snarks::PublicKey {
                    public_key: pk,
                    path,
                } => {
                    info!("Received SNARK work command for public key {pk}");

                    if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let snarks = db.get_snark_work_by_public_key(&pk.clone().into())?;
                        let snarks_str = format_vec_jq_compatible(&snarks);

                        if path.is_none() {
                            debug!("Writing SNARK work for public key {pk} to stdout");
                            ServerCliResponse::Success(snarks_str)
                        } else {
                            let path = path.unwrap();

                            if !path.is_dir() {
                                debug!("Writing SNARK work for public key {pk} to {path:?}");
                                std::fs::write(&path, snarks_str)?;
                                ServerCliResponse::Success(format!(
                                    "SNARK work for public key {pk} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
                Snarks::StateHash { state_hash, path } => {
                    info!("Received SNARK work command for state hash {state_hash}");

                    if !StateHash::is_valid(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        match db.get_block_snark_work(&state_hash.clone().into())? {
                            Some(snarks) => {
                                let snarks_str = format_vec_jq_compatible(&snarks);
                                if path.is_none() {
                                    debug!("Writing SNARK work for block {state_hash} to stdout");
                                    ServerCliResponse::Success(snarks_str)
                                } else {
                                    let path = path.unwrap();
                                    if !path.is_dir() {
                                        debug!(
                                            "Writing SNARK work for block {state_hash} to {path:?}"
                                        );
                                        std::fs::write(&path, snarks_str).unwrap();
                                        ServerCliResponse::Success(format!(
                                            "SNARK work for block {state_hash} written to {path:?}"
                                        ))
                                    } else {
                                        file_must_not_be_a_directory(&path)
                                    }
                                }
                            }
                            None => ServerCliResponse::Success(format!(
                                "No SNARK work found for block {state_hash}"
                            )),
                        }
                    }
                }

                Snarks::Top { num } => {
                    info!("Received top {num} SNARKers command");
                    ServerCliResponse::Success(serde_json::to_string_pretty(
                        &db.get_top_snark_provers_by_total_fees(num)?,
                    )?)
                }
            },
            ClientCli::Shutdown => {
                info!("Received shutdown command");
                // First respond success to client before shutting down
                let response =
                    ServerCliResponse::Success("Shutting down Mina Indexer...".to_string());
                let encoded = bincode::encode_to_vec(&response, BIN_CODE_CONFIG)?;
                writer.write_all(&encoded).await?;
                writer.flush().await?; // Ensure response is sent

                // Then initiate clean shutdown
                subsys.request_shutdown();
                return Ok(()); // Exit the function
            }
            ClientCli::Summary {
                verbose,
                json,
                path,
            } => {
                info!("Received summary command");

                let summary = state.summary_verbose();
                let summary_str = if verbose {
                    format_json(&summary, json)
                } else {
                    let summary: SummaryShort = summary.clone().into();
                    format_json(&summary, json)
                };

                if path.is_none() {
                    info!("Writing summary to stdout");
                    ServerCliResponse::Success(summary_str)
                } else {
                    let path = path.unwrap();
                    if !path.is_dir() {
                        info!("Writing summary to {path:?}");
                        std::fs::write(&path, summary_str)?;
                        ServerCliResponse::Success(format!("Summary written to {path:?}"))
                    } else {
                        file_must_not_be_a_directory(&path)
                    }
                }
            }
            ClientCli::Transactions(__) => match __ {
                Transactions::PublicKey {
                    public_key: pk,
                    verbose,
                    start_state_hash,
                    end_state_hash,
                    path,
                    csv,
                } => {
                    let start_state_hash: StateHash = start_state_hash.into();
                    let end_state_hash_result = match end_state_hash {
                        Some(hash) => Ok(hash.into()),
                        None => match db.get_best_block()? {
                            Some(best_tip) => Ok(best_tip.state_hash()),
                            None => Err(()),
                        },
                    };

                    info!("Received tx-public-key command for {pk}");

                    if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else if !StateHash::is_valid(&start_state_hash.0) {
                        invalid_state_hash(&start_state_hash.0)
                    } else if end_state_hash_result.is_err() {
                        best_tip_missing_from_db()
                    } else {
                        let end_state_hash = end_state_hash_result.unwrap();
                        if !StateHash::is_valid(&end_state_hash.0) {
                            invalid_state_hash(&end_state_hash.0)
                        } else if csv {
                            match db.write_user_commands_csv(&pk.clone().into(), path) {
                                Ok(path) => ServerCliResponse::Success(format!(
                                    "Successfully wrote user commands CSV for {pk} to {path:?}"
                                )),
                                Err(e) => ServerCliResponse::Error(format!(
                                    "Error writing user commands CSV for {pk}: {e}"
                                )),
                            }
                        } else {
                            let transactions = db
                                .get_user_commands_for_public_key(&pk.clone().into())?
                                .unwrap_or_default();
                            let transaction_str = if verbose {
                                format_vec_jq_compatible(&transactions)
                            } else {
                                let txs: Vec<Command> =
                                    transactions.into_iter().map(Command::from).collect();
                                format_vec_jq_compatible(&txs)
                            };

                            if path.is_none() {
                                debug!("Writing transactions for {pk} to stdout");
                                ServerCliResponse::Success(transaction_str)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    debug!("Writing transactions for {pk} to {path:?}");
                                    std::fs::write(&path, transaction_str)?;
                                    ServerCliResponse::Success(format!(
                                        "Transactions for {pk} written to {path:?}"
                                    ))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        }
                    }
                }
                Transactions::Hash { hash, verbose } => {
                    info!("Received tx-hash command for {hash}");
                    let hash = TxnHash::new(hash)?;
                    let response = db.get_user_command(&hash, 0)?.map(|cmd| {
                        if verbose {
                            format!("{cmd:?}")
                        } else {
                            let cmd: Command = cmd.into();
                            format!("{cmd:?}")
                        }
                    });
                    if let Some(msg) = response {
                        ServerCliResponse::Success(msg)
                    } else {
                        ServerCliResponse::Error("Failed to retrieve tx-hash".to_string())
                    }
                }
                Transactions::StateHash {
                    state_hash,
                    verbose,
                    path,
                } => {
                    info!("Received tx-state-hash command for {state_hash}");
                    if !StateHash::is_valid(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        let block_hash = StateHash(state_hash.to_owned());
                        match db.get_block_user_commands(&block_hash).unwrap_or_default() {
                            Some(cmds) => {
                                let transaction_str = if verbose {
                                    format_vec_jq_compatible(&cmds)
                                } else {
                                    let cmds: Vec<Command> =
                                        cmds.into_iter().map(Command::from).collect();
                                    format_vec_jq_compatible(&cmds)
                                };

                                if path.is_none() {
                                    debug!("Writing transactions for {state_hash} to stdout");
                                    ServerCliResponse::Success(transaction_str)
                                } else {
                                    let path = path.unwrap();
                                    if !path.is_dir() {
                                        debug!("Writing transactions for {state_hash} to {path:?}");
                                        std::fs::write(&path, transaction_str).unwrap();
                                        ServerCliResponse::Success(format!(
                                            "Transactions for {state_hash} written to {path:?}"
                                        ))
                                    } else {
                                        file_must_not_be_a_directory(&path)
                                    }
                                }
                            }
                            None => ServerCliResponse::Success(format!(
                                "No transactions found for block {state_hash}"
                            )),
                        }
                    }
                }
            },
            ClientCli::InternalCommands(__) => match __ {
                InternalCommands::PublicKey {
                    path,
                    public_key: pk,
                    csv,
                } => {
                    if !PublicKey::is_valid(&pk) {
                        invalid_public_key(&pk)
                    } else if csv {
                        match db.write_internal_commands_csv(pk.clone().into(), path) {
                            Ok(path) => ServerCliResponse::Success(format!(
                                "Successfully wrote internal commands CSV for {pk} to {path:?}"
                            )),
                            Err(e) => ServerCliResponse::Error(format!(
                                "Error writing internal commands CSV for {pk}: {e}"
                            )),
                        }
                    } else {
                        let internal_cmds =
                            db.get_internal_commands_public_key(&pk.clone().into(), 0, usize::MAX)?;
                        let internal_cmds_str = serde_json::to_string_pretty(&internal_cmds)?;

                        if path.is_none() {
                            debug!("Writing internal commands for {pk} to stdout");
                            ServerCliResponse::Success(internal_cmds_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing internal commands for {pk} to {path:?}");
                                std::fs::write(&path, internal_cmds_str)?;
                                ServerCliResponse::Success(format!(
                                    "Internal commands for {pk} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
                InternalCommands::StateHash { path, state_hash } => {
                    info!("Received internal-state-hash command for {}", state_hash);
                    if !StateHash::is_valid(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        let state_hash = StateHash(state_hash);
                        let internal_cmds_str =
                            serde_json::to_string_pretty(&db.get_internal_commands(&state_hash)?)?;

                        if path.is_none() {
                            debug!("Writing block internal commands for {state_hash} to stdout");
                            ServerCliResponse::Success(internal_cmds_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!(
                                    "Writing block internal commands for {state_hash} to {path:?}"
                                );

                                std::fs::write(&path, internal_cmds_str)?;
                                ServerCliResponse::Success(format!(
                                    "Block internal commands for {state_hash} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
            },
            ClientCli::DbVersion => ServerCliResponse::Success(format!(
                "mina-indexer database v{}",
                db.get_db_version()?
            )),
        };

        match response {
            ServerCliResponse::Success(_) => {}
            ServerCliResponse::Error(ref msg) => {
                error!("{}", msg);
            }
        };

        let encoded = bincode::encode_to_vec(&response, BIN_CODE_CONFIG)?;
        writer.write_all(&encoded).await?;
    }
    Ok(())
}

fn try_replace_old_socket(e: io::Error, unix_socket_path: &Path) -> io::Result<UnixListener> {
    if e.kind() == ErrorKind::AddrInUse {
        warn!("Unix domain socket: {unix_socket_path:?} already in use. Replacing old vestige");
        remove_unix_socket(unix_socket_path)?;
        UnixListener::bind(unix_socket_path)
    } else {
        Err(e)
    }
}

pub fn remove_unix_socket(unix_socket_path: &Path) -> io::Result<()> {
    std::fs::remove_file(unix_socket_path)?;
    debug!("Removed Unix domain socket: {unix_socket_path:?}");
    Ok(())
}

mod helpers {
    use super::*;

    pub fn invalid_public_key(input: &str) -> ServerCliResponse {
        ServerCliResponse::Success(format!("Invalid public key: {input}"))
    }

    pub fn invalid_state_hash(input: &str) -> ServerCliResponse {
        ServerCliResponse::Success(format!("Invalid state hash: {input}"))
    }

    pub fn account_missing_from_db(pk: &PublicKey) -> ServerCliResponse {
        ServerCliResponse::Success(format!("Account missing from store: {pk}"))
    }

    pub fn block_missing_from_db(state_hash: &str) -> String {
        format!("Block missing from store: {state_hash}")
    }

    pub fn best_tip_missing_from_db() -> ServerCliResponse {
        ServerCliResponse::Error("Best tip block missing from store".to_string())
    }

    pub fn file_must_not_be_a_directory(path: &std::path::Path) -> ServerCliResponse {
        ServerCliResponse::Success(format!(
            "The path provided must not be a directory: {}",
            path.display()
        ))
    }

    pub fn format_vec_jq_compatible<T>(vec: &Vec<T>) -> String
    where
        T: std::fmt::Debug,
    {
        let pp = format!("{vec:#?}");
        pp.replace(",\n]", "\n]")
    }

    pub fn format_json<T>(input: &T, json: bool) -> String
    where
        T: ?Sized + serde::Serialize + std::fmt::Display,
    {
        if json {
            serde_json::to_string(input).unwrap()
        } else {
            format!("{input}")
        }
    }
}
