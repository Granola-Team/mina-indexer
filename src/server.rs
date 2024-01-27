use crate::constants::MAINNET_TRANSITION_FRONTIER_K;
use crate::{
    block::{
        is_valid_block_file, parser::BlockParser, precomputed::PrecomputedBlock, BlockHash,
        BlockWithoutHeight,
    },
    ledger::genesis::GenesisRoot,
    state::IndexerState,
    store::IndexerStore,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
    thread,
};

use crossbeam_channel::{bounded, Receiver};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use tracing::{debug, error, info, instrument, warn};

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
#[instrument(skip_all)]
pub async fn start(indexer: MinaIndexer) -> anyhow::Result<()> {
    info!("Starting Mina Indexer...");
    let config = indexer.config.clone();
    let watch_dir = config.watch_dir.clone();
    let database_dir = config.database_dir.clone();

    //TODO: This doesn't need to be an Arc but it's easier to make it so for now
    let store = Arc::new(IndexerStore::new(&database_dir)?);
    let state = initialize(config, store).await?;
    run(watch_dir, Arc::new(Mutex::new(state)));
    Ok(())
}

fn run(block_watch_dir: PathBuf, state: Arc<Mutex<IndexerState>>) {
    let (ingestion_tx, ingestion_rx) = bounded(16384);
    // Launch watch block directory thread
    let _ = thread::spawn(move || {
        let _ = watch_directory_for_blocks(block_watch_dir, ingestion_tx);
    });

    let foobar = state.clone();
    // Launch precomputed block deserializer and persistence thread
    let _ = thread::spawn(move || {
        let _ = foobar_thread(foobar, ingestion_rx);
    });
}

fn foobar_thread(foobar: Arc<Mutex<IndexerState>>, ingestion_rx: Receiver<PathBuf>) {
    info!("Starting block persisting thread..");
    for path_buf in ingestion_rx {
        let precomputed_block = PrecomputedBlock::parse_file(&path_buf.as_path()).unwrap();
        let block = BlockWithoutHeight::from_precomputed(&precomputed_block);
        debug!("Deserialized precomputed block {block:?}");
        foobar
            .lock()
            .unwrap()
            .add_block_to_witness_tree(&precomputed_block)
            .unwrap();
    }
}
/// Watches a directory listening for when valid precomputed blocks are created and signals downstream
fn watch_directory_for_blocks<P: AsRef<Path>>(
    watch_dir: P,
    sender: crossbeam_channel::Sender<PathBuf>,
) -> notify::Result<()> {
    let (tx, rx) = bounded(4096);
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    watcher.watch(watch_dir.as_ref(), RecursiveMode::NonRecursive)?;
    info!("Starting block watcher thread..");
    info!(
        "Listening for precomputed blocks in directory: {:?}",
        watch_dir.as_ref()
    );
    for res in rx {
        match res {
            Ok(event) => {
                if let EventKind::Create(notify::event::CreateKind::File) = event.kind {
                    for path in event.paths {
                        if is_valid_block_file(&path) {
                            debug!("Valid precomputed block file");
                            if let Err(e) = sender.send(path) {
                                error!("Unable to send path downstream. {}", e);
                            }
                        } else {
                            warn!("Invalid precomputed block file: {}", path.display());
                        }
                    }
                }
            }
            Err(error) => error!("Error: {error:?}"),
        }
    }
    Ok(())
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
                state.initialize_with_canonical_chain_discovery(&mut block_parser)?;
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
