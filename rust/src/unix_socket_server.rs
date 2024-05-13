use crate::{
    block::{
        self, precomputed::PrecomputedBlockWithCanonicity, store::BlockStore, BlockHash,
        BlockWithoutHeight,
    },
    canonicity::store::CanonicityStore,
    client::*,
    command::{signed, store::CommandStore, Command},
    ledger::{
        self,
        public_key::{self, PublicKey},
        staking::AggregatedEpochStakeDelegation,
        store::LedgerStore,
        LedgerHash,
    },
    snark_work::store::SnarkStore,
    state::{summary::SummaryShort, IndexerState},
};
use anyhow::bail;
use log::{debug, error, info, trace, warn};
use std::{
    future::Future,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    process,
    sync::Arc,
};
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::RwLock,
};

#[derive(Debug)]
pub struct UnixSocketServer {
    state: Arc<RwLock<IndexerState>>,
    unix_socket: PathBuf,
}

impl UnixSocketServer {
    /// Create a new Unix domain socket server
    pub fn new(state: Arc<RwLock<IndexerState>>, unix_socket: PathBuf) -> Self {
        info!("Creating Unix domain socket server");
        Self { state, unix_socket }
    }
}

impl UnixSocketServer {
    async fn run(&self, listener: &UnixListener) -> io::Result<()> {
        info!("Accepting inbound Unix domain socket connections");
        loop {
            let (socket, _) = listener.accept().await?;
            let state = self.state.clone();
            // Spawn green thread ato handle connection
            tokio::spawn(async move {
                if let Err(e) = handle_conn(socket, &state).await {
                    error!("Unable to process Unix domain socket request: {}", e);
                }
            });
        }
    }
}

/// Start Unix Domain Socket server
pub async fn run(server: UnixSocketServer, shutdown: impl Future) {
    let listener = UnixListener::bind(server.unix_socket.clone())
        .or_else(|e| try_remove_old_socket(e, &server.unix_socket))
        .unwrap_or_else(|e| panic!("Unable to bind to Unix domain socket file: {}", e));

    info!(
        "Unix domain socket server running on: {:?}",
        server.unix_socket
    );

    tokio::select! {
        _ = shutdown => {
            info!("Received shutdown signal");
        }
        res = server.run(&listener) => {
            if let Err(e) = res {
                error!("Unable to process Unix domain socket request: {}", e);
            }
        }
    }
    info!("Unix domain socket server shutdown gracefully");
}

pub async fn parse_conn_to_cli(stream: &UnixStream) -> anyhow::Result<ClientCli> {
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
async fn handle_conn(
    conn: UnixStream,
    state: &Arc<RwLock<IndexerState>>,
) -> Result<(), anyhow::Error> {
    use helpers::*;
    let state = state.read().await;
    let db = if let Some(store) = state.indexer_store.as_ref() {
        store
    } else {
        bail!("Unable to get a handle on indexer store...");
    };
    let local_addr = conn.local_addr()?;

    let command = parse_conn_to_cli(&conn).await?;
    let (_, mut writer) = conn.into_split();

    let response_json = match command {
        ClientCli::Accounts(__) => match __ {
            Accounts::PublicKey { public_key: pk } => {
                info!("Received account command for {pk}");

                if let Some(best_tip) = db.get_best_block()? {
                    if let Some(ledger) = db.get_ledger_state_hash(&best_tip.state_hash(), false)? {
                        if !public_key::is_valid_public_key(&pk) {
                            invalid_public_key(&pk)
                        } else {
                            let pk = pk.into();
                            let account = ledger.accounts.get(&pk);
                            if let Some(account) = account {
                                info!("Writing account {pk} to client");
                                Some(format!("{account}"))
                            } else {
                                warn!("Account {pk} does not exist");
                                Some(format!("Account {pk} does not exist"))
                            }
                        }
                    } else {
                        error!("Best ledger not in database {}", best_tip.summary());
                        Some(format!(
                            "Best ledger not in database {}",
                            best_tip.summary()
                        ))
                    }
                } else {
                    best_tip_missing_from_db()
                }
            }
        },
        ClientCli::Blocks(__) => match __ {
            Blocks::BestTip { verbose, path } => {
                info!("Received best-tip command");

                if let Some(best_tip) = db.get_best_block()? {
                    let block_str = if let Some(canonicity) =
                        db.get_block_canonicity(&best_tip.state_hash())?
                    {
                        if verbose {
                            serde_json::to_string_pretty(&best_tip.with_canonicity(canonicity))?
                        } else {
                            let block = BlockWithoutHeight::with_canonicity(&best_tip, canonicity);
                            serde_json::to_string_pretty(&block)?
                        }
                    } else {
                        block_missing_from_db(&best_tip.state_hash().0)
                    };

                    if path.is_some() {
                        let path = &path.unwrap();
                        info!("Writing best tip block to {:?}", path);
                        std::fs::write(path, block_str)?;
                        Some(format!("Best block written to {}", path.display()))
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
                } else if let Ok(Some(ref block)) = db.get_block(&state_hash.clone().into()) {
                    let block_str =
                        if let Some(canonicity) = db.get_block_canonicity(&block.state_hash())? {
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
                        info!("Writing block {state_hash} to {:?}", path);
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
                    error!("Block at state hash not present in store: {}", state_hash);
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
                info!("Received blocks-at-height command");
                let mut blocks_at_height = db.get_blocks_at_height(height)?;
                blocks_at_height.sort();

                let blocks_str = if verbose {
                    let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_height
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
                    serde_json::to_string(&blocks)?
                } else {
                    let blocks: Vec<BlockWithoutHeight> = blocks_at_height
                        .iter()
                        .flat_map(|block| {
                            if let Ok(Some(canonicity)) =
                                db.get_block_canonicity(&block.state_hash())
                            {
                                Some(BlockWithoutHeight::with_canonicity(block, canonicity))
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
                        info!("Writing blocks at height {height} to {}", path.display());

                        std::fs::write(path.clone(), blocks_str)?;
                        Some(format!(
                            "Blocks at height {height} written to {}",
                            path.display()
                        ))
                    } else {
                        file_must_not_be_a_directory(&path)
                    }
                }
            }
            Blocks::Slot {
                slot,
                verbose,
                path,
            } => {
                info!("Received blocks-at-slot command");
                let slot: u32 = slot.parse()?;

                let mut blocks_at_slot = db.get_blocks_at_slot(slot)?;
                blocks_at_slot.sort();

                let blocks_str = if verbose {
                    let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_slot
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
                    serde_json::to_string(&blocks)?
                } else {
                    let blocks: Vec<BlockWithoutHeight> = blocks_at_slot
                        .iter()
                        .flat_map(|block| {
                            if let Ok(Some(canonicity)) =
                                db.get_block_canonicity(&block.state_hash())
                            {
                                Some(BlockWithoutHeight::with_canonicity(block, canonicity))
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
                        info!("Writing blocks at slot {slot} to {}", path.display());

                        std::fs::write(path.clone(), blocks_str)?;
                        Some(format!(
                            "Blocks at slot {slot} written to {}",
                            path.display()
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
                info!("Received blocks-at-public-key command");

                if !public_key::is_valid_public_key(&pk) {
                    invalid_public_key(&pk)
                } else {
                    let mut blocks_at_pk = db.get_blocks_at_public_key(&pk.clone().into())?;
                    blocks_at_pk.sort();

                    let blocks_str = if verbose {
                        let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks_at_pk
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
                        serde_json::to_string(&blocks)?
                    } else {
                        let blocks: Vec<BlockWithoutHeight> = blocks_at_pk
                            .iter()
                            .flat_map(|block| {
                                if let Ok(Some(canonicity)) =
                                    db.get_block_canonicity(&block.state_hash())
                                {
                                    Some(BlockWithoutHeight::with_canonicity(block, canonicity))
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
                            info!("Writing blocks at public key {pk} to {}", path.display());

                            std::fs::write(path.clone(), blocks_str)?;
                            Some(format!(
                                "Blocks at public key {pk} written to {}",
                                path.display()
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
                info!("Received block-children command for block {}", state_hash);

                let mut children = db.get_block_children(&state_hash.clone().into())?;
                children.sort();

                let blocks_str = if verbose {
                    let blocks: Vec<PrecomputedBlockWithCanonicity> = children
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
                    serde_json::to_string(&blocks)?
                } else {
                    let blocks: Vec<BlockWithoutHeight> = children
                        .iter()
                        .flat_map(|block| {
                            if let Ok(Some(canonicity)) =
                                db.get_block_canonicity(&block.state_hash())
                            {
                                Some(BlockWithoutHeight::with_canonicity(block, canonicity))
                            } else {
                                None
                            }
                        })
                        .collect();
                    format_vec_jq_compatible(&blocks)
                };

                if path.is_none() {
                    info!("Writing children of block {} to stdout", state_hash);
                    Some(blocks_str)
                } else {
                    let path = path.unwrap();
                    if !path.is_dir() {
                        info!(
                            "Writing children of block {} to {}",
                            state_hash,
                            path.display()
                        );

                        std::fs::write(path.clone(), blocks_str)?;
                        Some(format!(
                            "Children of block {} written to {}",
                            state_hash,
                            path.display()
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
                    } else if let (Some(end_block), Some(start_block)) = (
                        db.get_block(&end_state_hash.into())?,
                        db.get_block(&start_state_hash)?,
                    ) {
                        let start_height = start_block.blockchain_length();
                        let end_height = end_block.blockchain_length();
                        let mut parent_hash = end_block.previous_state_hash();
                        let mut best_chain = vec![end_block];

                        // constrain by num and state hash bound
                        for _ in 1..num.min(end_height.saturating_sub(start_height) + 1) {
                            if let Some(parent_pcb) = db.get_block(&parent_hash)? {
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
                                        Some(BlockWithoutHeight::with_canonicity(block, canonicity))
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
                                info!("Writing best chain to {}", path.display());

                                std::fs::write(path.clone(), best_chain_str)?;
                                Some(format!("Best chain written to {}", path.display()))
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
        ClientCli::Checkpoints(__) => match __ {
            Checkpoints::Create { path } => {
                info!("Received checkpoint command");
                if path.exists() {
                    error!("Checkpoint directory already exists at {}", path.display());
                    Some(format!(
                        "Checkpoint directory already exists at {}",
                        path.display()
                    ))
                } else {
                    debug!("Creating checkpoint at {}", path.display());
                    db.create_checkpoint(&path)?;
                    Some(format!(
                        "Checkpoint created and saved to {}",
                        path.display()
                    ))
                }
            }
        },
        ClientCli::Ledgers(__) => match __ {
            Ledgers::Best { path } => {
                info!("Received best-ledger command");

                if let Some(best_tip) = db.get_best_block()? {
                    if let Some(ledger) = db.get_ledger_state_hash(&best_tip.state_hash(), false)? {
                        let ledger = ledger.to_string_pretty();

                        if path.is_none() {
                            debug!("Writing best ledger to stdout");
                            Some(ledger)
                        } else {
                            let path = path.unwrap();
                            if path.is_dir() {
                                file_must_not_be_a_directory(&path)
                            } else {
                                debug!("Writing best ledger to {}", path.display());

                                std::fs::write(path.clone(), ledger)?;
                                Some(format!("Best ledger written to {}", path.display()))
                            }
                        }
                    } else {
                        error!("Best ledger cannot be calculated {}", best_tip.summary());
                        Some(format!(
                            "Best ledger cannot be calculated {}",
                            best_tip.summary()
                        ))
                    }
                } else {
                    best_tip_missing_from_db()
                }
            }
            Ledgers::Hash { hash, path } => {
                info!("Received ledger command for {hash}");

                // check if ledger or state hash and use appropriate getter
                if block::is_valid_state_hash(&hash) {
                    trace!("{hash} is a state hash");

                    if let Some(ledger) = db.get_ledger_state_hash(&hash.clone().into(), true)? {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing ledger at state hash {hash} to stdout");
                            Some(ledger)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                std::fs::write(path.clone(), ledger)?;
                                Some(format!(
                                    "Ledger at state hash {hash} written to {}",
                                    path.display()
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        error!("Ledger at state hash {hash} is not in the store");
                        Some(format!("Ledger at state hash {hash} is not in the store"))
                    }
                } else if ledger::is_valid_ledger_hash(&hash) {
                    trace!("{hash} is a ledger hash");

                    if let Some(ledger) = db.get_ledger(&LedgerHash(hash.clone()))? {
                        let ledger = ledger.to_string_pretty();
                        if path.is_none() {
                            debug!("Writing ledger at hash {hash} to stdout");
                            Some(ledger)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                std::fs::write(path.clone(), ledger)?;
                                Some(format!(
                                    "Ledger at hash {hash} written to {}",
                                    path.display()
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    } else {
                        error!("Ledger at {hash} is not in the store");
                        Some(format!("Ledger at {hash} is not in the store"))
                    }
                } else {
                    error!("Invalid ledger or state hash: {hash}");
                    Some(format!("Invalid ledger or state hash: {hash}"))
                }
            }
            Ledgers::Height { height, path } => {
                info!("Received ledger-at-height {height} command");

                if let Some(best_tip) = db.get_best_block()? {
                    if height > best_tip.blockchain_length() {
                        // ahead of witness tree - cannot compute
                        Some(format!("Invalid query: ledger at height {height} cannot be determined from a chain of length {}", best_tip.blockchain_length()))
                    } else {
                        let ledger_str = if Some(height)
                            > db.get_max_canonical_blockchain_length()?
                        {
                            // follow best chain back from tip to given height block and get the
                            // ledger
                            if let Some(mut curr_block) = db.get_block(&best_tip.state_hash())? {
                                while curr_block.blockchain_length() > height {
                                    if let Some(parent) =
                                        db.get_block(&curr_block.previous_state_hash())?
                                    {
                                        curr_block = parent;
                                    } else {
                                        break;
                                    }
                                }

                                if let Some(ledger) =
                                    db.get_ledger_state_hash(&curr_block.state_hash(), true)?
                                {
                                    ledger.to_string_pretty()
                                } else {
                                    block_missing_from_db(&curr_block.state_hash().0)
                                }
                            } else {
                                best_tip_missing_from_db().unwrap()
                            }
                        } else if let Some(ledger) = db.get_ledger_at_height(height, true)? {
                            ledger.to_string_pretty()
                        } else {
                            error!("Invalid ledger query. Ledger at height {height} not available");
                            format!("Invalid ledger query. Ledger at height {height} not available")
                        };

                        if path.is_none() {
                            debug!("Writing ledger at height {height} to stdout");
                            Some(ledger_str)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at height {height} to {}", path.display());

                                std::fs::write(&path, ledger_str)?;
                                Some(format!(
                                    "Ledger at height {height} written to {}",
                                    path.display()
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

                if ledger::is_valid_ledger_hash(&hash) {
                    trace!("{hash} is a ledger hash");

                    if let Some(staking_ledger) =
                        db.get_staking_ledger_hash(&hash.clone().into())?
                    {
                        let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                        if path.is_none() {
                            debug!("Writing staking ledger at hash {hash} to stdout");
                            Some(ledger_json)
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                std::fs::write(path.clone(), ledger_json)?;
                                Some(format!(
                                    "Staking ledger at hash {hash} written to {}",
                                    path.display()
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
                    db.get_staking_ledger_at_epoch(epoch, &Some(genesis_state_hash.into()))?
                {
                    let ledger_json = serde_json::to_string_pretty(&staking_ledger)?;
                    if path.is_none() {
                        debug!("Writing staking ledger at epoch {epoch} to stdout");
                        Some(ledger_json)
                    } else {
                        let path = path.unwrap();

                        if !path.is_dir() {
                            debug!("Writing ledger at epoch {epoch} to {}", path.display());

                            std::fs::write(path.clone(), ledger_json)?;
                            Some(format!(
                                "Staking ledger at epoch {epoch} written to {}",
                                path.display()
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
                info!(
                    "Received staking-delegations command for pk {} epoch {}",
                    pk, epoch,
                );

                if !block::is_valid_state_hash(&genesis_state_hash) {
                    invalid_state_hash(&genesis_state_hash)
                } else if !public_key::is_valid_public_key(&pk) {
                    invalid_public_key(&pk)
                } else if let Some(aggegated_delegations) =
                    db.get_delegations_epoch(epoch, &Some(genesis_state_hash.into()))?
                {
                    let pk: PublicKey = pk.into();
                    let epoch = aggegated_delegations.epoch;
                    let network = aggegated_delegations.network;
                    let total_delegations = aggegated_delegations.total_delegations;
                    let count_delegates = aggegated_delegations
                        .delegations
                        .get(&pk)
                        .and_then(|agg_del| agg_del.count_delegates);
                    let total_delegated = aggegated_delegations
                        .delegations
                        .get(&pk)
                        .and_then(|agg_del| agg_del.total_delegated);
                    Some(serde_json::to_string_pretty(
                        &AggregatedEpochStakeDelegation {
                            pk,
                            epoch,
                            network,
                            count_delegates,
                            total_delegated,
                            total_stake: total_delegations,
                        },
                    )?)
                } else {
                    error!(
                        "Public key {} is missing from staking ledger epoch {}",
                        pk, epoch
                    );
                    Some(format!(
                        "Public key {} is missing from staking ledger epoch {}",
                        pk, epoch
                    ))
                }
            }
            StakingLedgers::Delegations {
                epoch,
                genesis_state_hash,
                path,
            } => {
                info!("Received staking-delegations command for epoch {}", epoch);

                let aggregated_delegations =
                    db.get_delegations_epoch(epoch, &Some(genesis_state_hash.into()))?;
                if let Some(agg_del_str) = aggregated_delegations
                    .map(|agg_del| serde_json::to_string_pretty(&agg_del).unwrap())
                {
                    if path.is_none() {
                        debug!(
                            "Writing aggregated staking delegations epoch {} to stdout",
                            epoch
                        );
                        Some(agg_del_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            debug!(
                                "Writing aggregated staking delegations epoch {} to {}",
                                epoch,
                                path.display()
                            );

                            std::fs::write(&path, agg_del_str)?;
                            Some(format!(
                                "Aggregated staking delegations epoch {} written to {}",
                                epoch,
                                path.display()
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                } else {
                    error!("Unable to aggregate staking delegations epoch {}", epoch);
                    Some(format!(
                        "Unable to aggregate staking delegations epoch {}",
                        epoch
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
                            debug!(
                                "Writing SNARK work for public key {pk} to {}",
                                path.display()
                            );

                            std::fs::write(&path, snarks_str)?;
                            Some(format!(
                                "SNARK work for public key {pk} written to {}",
                                path.display()
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
                                        "Writing SNARK work for block {state_hash} to {}",
                                        path.display()
                                    );

                                    std::fs::write(&path, snarks_str).unwrap();
                                    Some(format!(
                                        "SNARK work for block {state_hash} written to {}",
                                        path.display()
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
                Some(serde_json::to_string_pretty(&db.get_top_snarkers(num)?)?)
            }
        },
        ClientCli::Shutdown => {
            info!("Received shutdown command");
            writer
                .write_all(b"Shutting down the Mina Indexer daemon...")
                .await?;
            remove_unix_socket(
                local_addr
                    .as_pathname()
                    .expect("Unable to locate Unix domain socket file"),
            )
            .expect("Unix domain socket file deleted");
            process::exit(0);
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
                    info!("Writing summary to {}", path.display());

                    std::fs::write(&path, summary_str)?;
                    Some(format!("Summary written to {}", path.display()))
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
                    let transactions = db.get_commands_for_public_key(&pk.clone().into())?;
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
                            debug!("Writing transactions for {pk} to {}", path.display());

                            std::fs::write(&path, transaction_str)?;
                            Some(format!(
                                "Transactions for {pk} written to {}",
                                path.display()
                            ))
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
                    db.get_command_by_hash(&hash)?.map(|cmd| {
                        if verbose {
                            format!("{cmd:?}")
                        } else {
                            let cmd: Command = cmd.command.into();
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
                    Some(db.get_commands_in_block(&block_hash).map(|cmds| {
                        let transaction_str = if verbose {
                            format_vec_jq_compatible(&cmds)
                        } else {
                            let cmds: Vec<Command> = cmds.into_iter().map(Command::from).collect();
                            format_vec_jq_compatible(&cmds)
                        };
                        if path.is_none() {
                            debug!("Writing transactions for {state_hash} to stdout");
                            transaction_str
                        } else {
                            let path = path.unwrap();
                            if !path.is_dir() {
                                debug!(
                                    "Writing transactions for {state_hash} to {}",
                                    path.display()
                                );

                                std::fs::write(&path, transaction_str).unwrap();
                                format!(
                                    "Transactions for {state_hash} written to {}",
                                    path.display()
                                )
                            } else {
                                file_must_not_be_a_directory(&path).unwrap()
                            }
                        }
                    })?)
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
                    let internal_cmds = db.get_internal_commands_public_key(&pk.clone().into())?;
                    let internal_cmds_str = serde_json::to_string_pretty(&internal_cmds)?;

                    if path.is_none() {
                        debug!("Writing internal commands for {} to stdout", pk);
                        Some(internal_cmds_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            debug!("Writing internal commands for {} to {}", pk, path.display());

                            std::fs::write(&path, internal_cmds_str)?;
                            Some(format!(
                                "Internal commands for {} written to {}",
                                pk,
                                path.display()
                            ))
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
                        debug!(
                            "Writing block internal commands for {} to stdout",
                            state_hash
                        );
                        Some(internal_cmds_str)
                    } else {
                        let path = path.unwrap();
                        if !path.is_dir() {
                            debug!(
                                "Writing block internal commands for {} to {}",
                                state_hash,
                                path.display()
                            );

                            std::fs::write(&path, internal_cmds_str)?;
                            Some(format!(
                                "Block internal commands for {} written to {}",
                                state_hash,
                                path.display()
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
            }
        },
    };

    let response = if let Some(response_json) = response_json {
        response_json
    } else {
        serde_json::to_string("no response 404")?
    };
    writer.write_all(response.as_bytes()).await?;

    Ok(())
}

fn file_must_not_be_a_directory(path: &std::path::Path) -> Option<String> {
    Some(format!(
        "The path provided must not be a directory: {}",
        path.display()
    ))
}

fn try_remove_old_socket(e: io::Error, unix_socket_path: &PathBuf) -> io::Result<UnixListener> {
    if e.kind() == ErrorKind::AddrInUse {
        debug!(
            "Unix domain socket: {:?} already in use. Removing old vestige",
            unix_socket_path
        );
        remove_unix_socket(unix_socket_path)?;
        UnixListener::bind(unix_socket_path)
    } else {
        Err(e)
    }
}

fn remove_unix_socket(unix_socket_path: &Path) -> io::Result<()> {
    std::fs::remove_file(unix_socket_path)?;
    debug!("Unix domain socket removed: {:?}", unix_socket_path);
    Ok(())
}

mod helpers {
    use super::*;

    pub fn invalid_public_key(input: &str) -> Option<String> {
        let msg = format!("Invalid public key: {input}");
        error!("Invalid public key: {}", input);
        Some(msg)
    }

    pub fn invalid_tx_hash(input: &str) -> Option<String> {
        let msg = format!("Invalid transaction hash: {input}");
        error!("Invalid transaction hash: {}", input);
        Some(msg)
    }

    pub fn invalid_state_hash(input: &str) -> Option<String> {
        let msg = format!("Invalid state hash: {input}");
        error!("Invalid state hash: {}", input);
        Some(msg)
    }

    pub fn block_missing_from_db(state_hash: &str) -> String {
        let msg = format!("Block missing from store: {state_hash}");
        error!("Block missing from store: {}", state_hash);
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
