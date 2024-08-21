use crate::{
    block::{
        self, precomputed::PrecomputedBlockWithCanonicity, store::BlockStore, BlockHash,
        BlockWithoutHeight,
    },
    canonicity::store::CanonicityStore,
    client::*,
    command::{internal::store::InternalCommandStore, signed, store::UserCommandStore, Command},
    ledger::{
        self,
        public_key::{self, PublicKey},
        staking::AggregatedEpochStakeDelegation,
        store::{best::BestLedgerStore, staged::StagedLedgerStore, staking::StakingLedgerStore},
        Ledger, LedgerHash,
    },
    snark_work::store::SnarkStore,
    state::{summary::SummaryShort, IndexerState},
    store::version::VersionStore,
};
use anyhow::{bail, Context};
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

#[allow(clippy::just_underscores_and_digits)]
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

        let state = state.read().await;
        let db = if let Some(store) = state.indexer_store.as_ref() {
            store
        } else {
            bail!("Unable to get a handle on indexer store...");
        };

        let command = parse_conn_to_cli(&connection).await?;
        let (_, mut writer) = connection.into_split();

        let response_json = match command {
            ClientCli::Accounts(__) => match __ {
                Accounts::PublicKey { public_key: pk } => {
                    info!("Received account command for {pk}");
                    if !public_key::is_valid_public_key(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let pk: PublicKey = pk.into();
                        if let Some(account) = db.get_best_account(&pk)? {
                            info!("Writing account {pk} to client");
                            Some(format!("{account}"))
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
                            Some(format!("Best block written to {path:?}"))
                        } else {
                            info!("Writing best tip block to stdout");
                            Some(block_str)
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
                    if !block::is_valid_state_hash(&state_hash) {
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
                            Some(format!(
                                "Block {} written to {:?}",
                                block.state_hash().0,
                                path
                            ))
                        } else {
                            info!("Writing block to stdout {}", block.summary());
                            Some(block_str)
                        }
                    } else {
                        error!("Block at state hash not present in store: {state_hash}");
                        Some(format!(
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
                        Some(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing blocks at height {height} to {path:?}");
                            std::fs::write(path.clone(), blocks_str)?;
                            Some(format!("Blocks at height {height} written to {path:?}"))
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
                        Some(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing blocks at slot {slot} to {path:?}");

                            std::fs::write(path.clone(), blocks_str)?;
                            Some(format!("Blocks at slot {slot} written to {path:?}"))
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
                    if !public_key::is_valid_public_key(&pk) {
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
                            Some(blocks_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                info!("Writing blocks at public key {pk} to {path:?}");

                                std::fs::write(path.clone(), blocks_str)?;
                                Some(format!("Blocks at public key {pk} written to {path:?}"))
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
                        Some(blocks_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            info!("Writing children of block {state_hash} to {path:?}");
                            std::fs::write(path.clone(), blocks_str)?;
                            Some(format!(
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
                    let start_state_hash: BlockHash = start_state_hash.into();
                    if let Some(best_tip) = db.get_best_block()? {
                        let end_state_hash: String = {
                            if end_state_hash.is_none() {
                                best_tip.state_hash().0
                            } else {
                                let end_state_hash = &end_state_hash.unwrap();
                                if !block::is_valid_state_hash(end_state_hash) {
                                    best_tip.state_hash().0
                                } else {
                                    end_state_hash.into()
                                }
                            }
                        };

                        if !block::is_valid_state_hash(&start_state_hash.0) {
                            invalid_state_hash(&start_state_hash.0)
                        } else if let (Some((end_block, _)), Some((start_block, _))) = (
                            db.get_block(&end_state_hash.into())?,
                            db.get_block(&start_state_hash)?,
                        ) {
                            let start_height = start_block.blockchain_length();
                            let end_height = end_block.blockchain_length();
                            let mut parent_hash = end_block.previous_state_hash();
                            let mut best_chain = vec![end_block];

                            // constrain by num and state hash bound
                            for _ in 1..num.min(end_height.saturating_sub(start_height) + 1) {
                                if let Some((parent_pcb, _)) = db.get_block(&parent_hash)? {
                                    let curr_hash: BlockHash = parent_pcb.state_hash();
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
                                Some(best_chain_str)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    info!("Writing best chain to {path:?}");

                                    std::fs::write(path.clone(), best_chain_str)?;
                                    Some(format!("Best chain written to {path:?}"))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        best_tip_missing_from_db()
                    }
                }
            },
            ClientCli::CreateSnapshot { output_path } => {
                info!("Received create-snapshot command");
                match db.create_snapshot(&output_path) {
                    Err(e) => Some(e.to_string()),
                    Ok(s) => Some(s),
                }
            }
            ClientCli::Ledgers(__) => match __ {
                Ledgers::Best { path, memoize } => {
                    info!("Received best-ledger command");
                    if let Some(ledger) = db.get_best_ledger(memoize)? {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing best ledger to stdout");
                            Some(ledger)
                        } else {
                            let path = path.unwrap();
                            if path.is_dir() {
                                file_must_not_be_a_directory(&path)
                            } else {
                                debug!("Writing best ledger to {path:?}");
                                std::fs::write(path.clone(), ledger)?;
                                Some(format!("Best ledger written to {path:?}"))
                            }
                        }
                    } else {
                        error!("Best ledger cannot be calculated");
                        Some("Best ledger cannot be calculated".into())
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
                    ) -> Option<String> {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing staged ledger at hash {hash} to stdout");
                            Some(ledger)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing staged ledger at {hash} to {path:?}");
                                std::fs::write(path.clone(), ledger).ok();
                                Some(format!("Ledger at hash {hash} written to {path:?}"))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }

                    // check if ledger or state hash and use appropriate getter
                    if block::is_valid_state_hash(&hash) {
                        trace!("{hash} is a state hash");
                        if let Some(ledger) =
                            db.get_staged_ledger_at_state_hash(&hash.clone().into(), memoize)?
                        {
                            write_ledger(path, ledger, &hash)
                        } else {
                            error!("Ledger at state hash {hash} is not in the store");
                            Some(format!("Ledger at state hash {hash} is not in the store"))
                        }
                    } else if ledger::is_valid_ledger_hash(&hash) {
                        trace!("{hash} is a ledger hash");
                        if let Some(ledger) =
                            db.get_staged_ledger_at_ledger_hash(&LedgerHash(hash.clone()), memoize)?
                        {
                            write_ledger(path, ledger, &hash)
                        } else {
                            error!("Ledger at ledger hash {hash} is not in the store");
                            Some(format!("Ledger at ledger hash {hash} is not in the store"))
                        }
                    } else {
                        error!("Invalid ledger or state hash: {hash}");
                        Some(format!("Invalid ledger or state hash: {hash}"))
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
                            Some(format!("Invalid query: ledger at height {height} cannot be determined from a chain of length {best_tip_height}"))
                        } else {
                            let ledger_str = db
                                .get_staged_ledger_at_block_height(height, memoize)?
                                .unwrap()
                                .to_string_pretty();
                            if path.is_none() {
                                debug!("Writing ledger at height {height} to stdout");
                                Some(ledger_str)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    debug!("Writing ledger at height {height} to {path:?}");
                                    std::fs::write(&path, ledger_str)?;
                                    Some(format!("Ledger at height {height} written to {path:?}"))
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
                    if ledger::is_valid_ledger_hash(&hash) {
                        trace!("{hash} is a ledger hash");
                        if let Some(staking_ledger) =
                            db.get_staking_ledger(&hash.clone().into(), None, &None)?
                        {
                            let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                            if path.is_none() {
                                debug!("Writing staking ledger at hash {hash} to stdout");
                                Some(ledger_json)
                            } else {
                                let path = path.unwrap();
                                if !path.is_dir() {
                                    debug!("Writing ledger at {hash} to {path:?}");

                                    std::fs::write(path.clone(), ledger_json)?;
                                    Some(format!(
                                        "Staking ledger at hash {hash} written to {path:?}"
                                    ))
                                } else {
                                    file_must_not_be_a_directory(&path)
                                }
                            }
                        } else {
                            error!("Staking ledger at {hash} is not in the store");
                            Some(format!("Staking ledger at {hash} is not in the store"))
                        }
                    } else {
                        error!("Invalid ledger hash: {hash}");
                        Some(format!("Invalid ledger hash: {hash}"))
                    }
                }
                StakingLedgers::Epoch {
                    epoch,
                    genesis_state_hash,
                    path,
                } => {
                    info!("Received staking-ledgers-epoch {epoch} command");
                    if !block::is_valid_state_hash(&genesis_state_hash) {
                        invalid_state_hash(&genesis_state_hash)
                    } else if let Some(staking_ledger) =
                        db.build_staking_ledger(epoch, &Some(genesis_state_hash.into()))?
                    {
                        let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                        if path.is_none() {
                            debug!("Writing staking ledger at epoch {epoch} to stdout");
                            Some(ledger_json)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at epoch {epoch} to {path:?}");
                                std::fs::write(path.clone(), ledger_json)?;
                                Some(format!(
                                    "Staking ledger at epoch {epoch} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        error!("Staking ledger at epoch {epoch} is not in the store");
                        Some(format!(
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
                    if !block::is_valid_state_hash(&genesis_state_hash) {
                        invalid_state_hash(&genesis_state_hash)
                    } else if !public_key::is_valid_public_key(&pk) {
                        invalid_public_key(&pk)
                    } else if let Some(aggregated_delegations) =
                        db.build_aggregated_delegations(epoch, &Some(genesis_state_hash.into()))?
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
                        Some(serde_json::to_string_pretty(
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
                        error!("Public key {pk} is missing from staking ledger epoch {epoch}");
                        Some(format!(
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
                        db.build_aggregated_delegations(epoch, &Some(genesis_state_hash.into()))?;
                    if let Some(agg_del_str) = aggregated_delegations
                        .map(|agg_del| serde_json::to_string_pretty(&agg_del).unwrap())
                    {
                        if path.is_none() {
                            debug!(
                                "Writing aggregated staking delegations epoch {epoch} to stdout"
                            );
                            Some(agg_del_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing aggregated staking delegations epoch {epoch} to {path:?}");
                                std::fs::write(&path, agg_del_str)?;
                                Some(format!(
                                    "Aggregated staking delegations epoch {epoch} written to {path:?}"))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        error!("Unable to aggregate staking delegations epoch {epoch}");
                        Some(format!(
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

                    if !public_key::is_valid_public_key(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let snarks = db
                            .get_snark_work_by_public_key(&pk.clone().into())?
                            .unwrap_or(vec![]);
                        let snarks_str = format_vec_jq_compatible(&snarks);

                        if path.is_none() {
                            debug!("Writing SNARK work for public key {pk} to stdout");
                            Some(snarks_str)
                        } else {
                            let path = path.unwrap();

                            if !path.is_dir() {
                                debug!("Writing SNARK work for public key {pk} to {path:?}");
                                std::fs::write(&path, snarks_str)?;
                                Some(format!(
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

                    if !block::is_valid_state_hash(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        db.get_snark_work_in_block(&state_hash.clone().into())?
                            .and_then(|snarks| {
                                let snarks_str = format_vec_jq_compatible(&snarks);
                                if path.is_none() {
                                    debug!("Writing SNARK work for block {state_hash} to stdout");
                                    Some(snarks_str)
                                } else {
                                    let path = path.unwrap();
                                    if !path.is_dir() {
                                        debug!(
                                            "Writing SNARK work for block {state_hash} to {path:?}"
                                        );
                                        std::fs::write(&path, snarks_str).unwrap();
                                        Some(format!(
                                            "SNARK work for block {state_hash} written to {path:?}"
                                        ))
                                    } else {
                                        file_must_not_be_a_directory(&path)
                                    }
                                }
                            })
                    }
                }
                Snarks::Top { num } => {
                    info!("Received top {num} SNARKers command");
                    Some(serde_json::to_string_pretty(
                        &db.get_top_snark_workers_by_fees(num)?,
                    )?)
                }
            },
            ClientCli::Shutdown => {
                info!("Received shutdown command");
                writer
                    .write_all(b"Shutting down the Mina Indexer daemon...")
                    .await?;
                subsys.request_shutdown();
                return Ok(());
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
                    Some(summary_str)
                } else {
                    let path = path.unwrap();
                    if !path.is_dir() {
                        info!("Writing summary to {path:?}");
                        std::fs::write(&path, summary_str)?;
                        Some(format!("Summary written to {path:?}"))
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
                } => {
                    let start_state_hash: BlockHash = start_state_hash.into();
                    let end_state_hash: BlockHash = {
                        let raw = end_state_hash.unwrap_or("x".to_string());
                        if &raw == "x" {
                            // dummy value replaced with best block state hash
                            if let Some(best_tip) = db.get_best_block()? {
                                best_tip.state_hash()
                            } else {
                                best_tip_missing_from_db().unwrap().into()
                            }
                        } else {
                            raw.into()
                        }
                    };
                    info!("Received tx-public-key command for {pk}");

                    if !public_key::is_valid_public_key(&pk) {
                        invalid_public_key(&pk)
                    } else if !block::is_valid_state_hash(&start_state_hash.0) {
                        invalid_state_hash(&start_state_hash.0)
                    } else if !block::is_valid_state_hash(&end_state_hash.0) {
                        invalid_state_hash(&end_state_hash.0)
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
                            Some(transaction_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing transactions for {pk} to {path:?}");
                                std::fs::write(&path, transaction_str)?;
                                Some(format!("Transactions for {pk} written to {path:?}"))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
                Transactions::Hash { hash, verbose } => {
                    info!("Received tx-hash command for {hash}");
                    if !signed::is_valid_tx_hash(&hash) {
                        invalid_tx_hash(&hash)
                    } else {
                        db.get_user_command(&hash, 0)?.map(|cmd| {
                            if verbose {
                                format!("{cmd:?}")
                            } else {
                                let cmd: Command = cmd.into();
                                format!("{cmd:?}")
                            }
                        })
                    }
                }
                Transactions::StateHash {
                    state_hash,
                    verbose,
                    path,
                } => {
                    info!("Received tx-state-hash command for {state_hash}");
                    if !block::is_valid_state_hash(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        let block_hash = BlockHash(state_hash.to_owned());
                        db.get_block_user_commands(&block_hash)
                            .unwrap_or_default()
                            .map(|cmds| {
                                let transaction_str = if verbose {
                                    format_vec_jq_compatible(&cmds)
                                } else {
                                    let cmds: Vec<Command> =
                                        cmds.into_iter().map(Command::from).collect();
                                    format_vec_jq_compatible(&cmds)
                                };

                                if path.is_none() {
                                    debug!("Writing transactions for {state_hash} to stdout");
                                    transaction_str
                                } else {
                                    let path = path.unwrap();
                                    if !path.is_dir() {
                                        debug!("Writing transactions for {state_hash} to {path:?}");
                                        std::fs::write(&path, transaction_str).unwrap();
                                        format!("Transactions for {state_hash} written to {path:?}")
                                    } else {
                                        file_must_not_be_a_directory(&path).unwrap()
                                    }
                                }
                            })
                    }
                }
            },
            ClientCli::InternalCommands(__) => match __ {
                InternalCommands::PublicKey {
                    path,
                    public_key: pk,
                } => {
                    if !public_key::is_valid_public_key(&pk) {
                        invalid_public_key(&pk)
                    } else {
                        let internal_cmds =
                            db.get_internal_commands_public_key(&pk.clone().into())?;
                        let internal_cmds_str = serde_json::to_string_pretty(&internal_cmds)?;

                        if path.is_none() {
                            debug!("Writing internal commands for {pk} to stdout");
                            Some(internal_cmds_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing internal commands for {pk} to {path:?}");
                                std::fs::write(&path, internal_cmds_str)?;
                                Some(format!("Internal commands for {pk} written to {path:?}"))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
                InternalCommands::StateHash { path, state_hash } => {
                    info!("Received internal-state-hash command for {}", state_hash);
                    if !block::is_valid_state_hash(&state_hash) {
                        invalid_state_hash(&state_hash)
                    } else {
                        let internal_cmds_str = serde_json::to_string_pretty(
                            &db.get_internal_commands(&state_hash.clone().into())?,
                        )?;

                        if path.is_none() {
                            debug!("Writing block internal commands for {state_hash} to stdout");
                            Some(internal_cmds_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!(
                                    "Writing block internal commands for {state_hash} to {path:?}"
                                );

                                std::fs::write(&path, internal_cmds_str)?;
                                Some(format!(
                                    "Block internal commands for {state_hash} written to {path:?}"
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                }
            },
            ClientCli::DbVersion => {
                Some(format!("mina-indexer database v{}", db.get_db_version()?))
            }
        };

        let response = if let Some(response_json) = response_json {
            response_json
        } else {
            serde_json::to_string("no response 404")?
        };
        writer.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

fn file_must_not_be_a_directory(path: &std::path::Path) -> Option<String> {
    Some(format!(
        "The path provided must not be a directory: {}",
        path.display()
    ))
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

    pub fn invalid_public_key(input: &str) -> Option<String> {
        let msg = format!("Invalid public key: {input}");
        error!("Invalid public key: {input}");
        Some(msg)
    }

    pub fn invalid_tx_hash(input: &str) -> Option<String> {
        let msg = format!("Invalid transaction hash: {input}");
        error!("Invalid transaction hash: {input}");
        Some(msg)
    }

    pub fn invalid_state_hash(input: &str) -> Option<String> {
        let msg = format!("Invalid state hash: {input}");
        error!("Invalid state hash: {input}");
        Some(msg)
    }

    pub fn account_missing_from_db(pk: &PublicKey) -> Option<String> {
        let msg = format!("Account missing from store: {pk}");
        error!("Account missing from store: {pk}");
        Some(msg)
    }

    pub fn block_missing_from_db(state_hash: &str) -> String {
        let msg = format!("Block missing from store: {state_hash}");
        error!("Block missing from store: {state_hash}");
        msg
    }

    /// Always returns `Some`, safe to `.unwrap()`
    pub fn best_tip_missing_from_db() -> Option<String> {
        let msg = "Best tip block missing from store";
        error!("Best tip block missing from store");
        Some(msg.to_string())
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
