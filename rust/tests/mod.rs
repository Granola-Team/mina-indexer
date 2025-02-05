//////////////////
// Test modules //
//////////////////

mod block;
mod canonicity;
mod command;
mod event;
mod ledger;
#[cfg(all(test, feature = "tier2"))]
mod protocol;
mod snark_work;
mod state;
mod usernames;
mod zkapps;

//////////////////
// Test helpers //
//////////////////

pub mod generators;

pub mod helpers {
    pub mod store {
        /// Sets up a new temp dir, deleted when it goes out of scope
        pub fn setup_new_db_dir(prefix: &str) -> anyhow::Result<tempfile::TempDir> {
            let store_dir = tempfile::TempDir::with_prefix(prefix)?;

            if store_dir.path().exists() {
                std::fs::remove_dir_all(store_dir.path())?;
            }

            Ok(store_dir)
        }
    }

    pub mod state {
        use mina_indexer::{
            constants::{MAINNET_CANONICAL_THRESHOLD, MAINNET_TRANSITION_FRONTIER_K},
            state::IndexerState,
            store::IndexerStore,
        };
        use std::path::Path;

        /// Creates an indexer from the original mainnet genesis ledger & block
        pub fn mainnet_genesis_state<P>(path: P) -> anyhow::Result<IndexerState>
        where
            P: AsRef<Path>,
        {
            let indexer_store = std::sync::Arc::new(IndexerStore::new(path.as_ref())?);
            IndexerState::new_v1(
                indexer_store,
                MAINNET_CANONICAL_THRESHOLD,
                MAINNET_TRANSITION_FRONTIER_K,
                false,
            )
        }

        /// Creates an indexer from the hardfork genesis ledger & block
        pub fn hardfork_genesis_state<P>(path: P) -> anyhow::Result<IndexerState>
        where
            P: AsRef<Path>,
        {
            let indexer_store = std::sync::Arc::new(IndexerStore::new(path.as_ref())?);
            IndexerState::new_v2(
                indexer_store,
                MAINNET_CANONICAL_THRESHOLD,
                MAINNET_TRANSITION_FRONTIER_K,
                false,
            )
        }
    }
}
