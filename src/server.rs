use crate::{
    block::{parser::BlockParser, Block, BlockHash, BlockWithoutHeight},
    constants::{MAINNET_TRANSITION_FRONTIER_K, SOCKET_NAME},
    ledger::{genesis::GenesisRoot, Ledger},
    state::IndexerState,
    store::IndexerStore,
};

use std::{
    fs, path::{Path, PathBuf}, process, sync::Arc
};

use tracing::{debug, info};

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub ledger: GenesisRoot,
    pub root_hash: BlockHash,
    pub startup_dir: PathBuf,
    pub watch_dir: PathBuf,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub initialization_mode: InitializationMode,
    pub ledger_cadence: u32,
    pub database_dir: PathBuf,
}

pub struct MinaIndexer {
    config: IndexerConfiguration,
}

#[derive(Debug, Clone)]
pub enum InitializationMode {
    New,
    Replay,
    Sync,
}

impl MinaIndexer {
    pub fn new(config: IndexerConfiguration) -> Self {
        Self { config }
    }
}

pub async fn start(indexer: MinaIndexer) -> anyhow::Result<()> {
    let config = indexer.config.clone();
    let watch_dir = config.watch_dir.clone();
    let database_dir = config.database_dir.clone();

    //TODO: This doesn't need to be an Arc but it's easier to make it so for now
    let store = Arc::new(IndexerStore::new(&database_dir)?);
    let state = initialize(config, store).await?;
        
    let _ = tokio::spawn(async move {
        run(watch_dir, state).await
    });
    
    Ok(())
}

async fn run(block_watch_dir: impl AsRef<Path>, state: IndexerState) {
    todo!()
}

async fn initialize(
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
) -> anyhow::Result<IndexerState> {
    debug!("Setting Ctrl-C handler");
    ctrlc::set_handler(move || {
        info!("SIGINT received. Exiting.");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    info!("Starting mina-indexer server");
    let db_path = store.db_path.clone();
    let IndexerConfiguration {
        ledger,
        root_hash,
        startup_dir,
        watch_dir: _,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode,
        ledger_cadence,
        database_dir,
    } = config;

    fs::create_dir_all(startup_dir.clone()).expect("startup_dir created");

    let state = {
        let mut state = match initialization_mode {
            InitializationMode::New => {
                info!(
                    "Initializing indexer state from blocks in {}",
                    startup_dir.display()
                );
                IndexerState::new(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                    ledger_cadence,
                )?
            }
            InitializationMode::Replay => {
                info!("Replaying indexer events from db at {}", db_path.display());
                IndexerState::new_without_genesis_events(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                    ledger_cadence,
                )?
            }
            InitializationMode::Sync => {
                info!("Syncing indexer state from db at {}", db_path.display());
                IndexerState::new_without_genesis_events(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                    ledger_cadence,
                )?
            }
        };

        match initialization_mode {
            InitializationMode::New => {
                info!("Parsing blocks");
                let mut block_parser = BlockParser::new(&startup_dir, canonical_threshold)?;
                state
                    .initialize_with_canonical_chain_discovery(&mut block_parser)
                    .await?;
            }
            InitializationMode::Replay => {
                state.replay_events()?;
            }
            InitializationMode::Sync => {
                state.sync_from_db()?;
            }
        }
        state
    };

    Ok(state)
}

// block_fut = filesystem_receiver.recv_block() => {
//     if let Some(precomputed_block) = block_fut? {
//         let block = BlockWithoutHeight::from_precomputed(&precomputed_block);
//         debug!("Receiving block {block:?}");

//         state.add_block_to_witness_tree(&precomputed_block)?;
//         info!("Added {block:?}");
//     } else {
//         info!("Block receiver shutdown, system exit");
//         return Ok(())
//     }
// }
