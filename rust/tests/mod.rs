//////////////////
// Test modules //
//////////////////

mod block;
mod canonicity;
mod command;
mod event;
mod ledger;
#[cfg(all(test, feature = "mina_rs"))]
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
    /// Sets up a new temp dir, deleted when it goes out of scope
    pub fn setup_new_db_dir(prefix: &str) -> anyhow::Result<tempfile::TempDir> {
        let store_dir = tempfile::TempDir::with_prefix(prefix)?;
        if store_dir.path().exists() {
            std::fs::remove_dir_all(store_dir.path())?;
        }
        Ok(store_dir)
    }

    // for building the hardfork ledger in tests

    use mina_indexer::{
        constants::MAINNET_CANONICAL_THRESHOLD,
        ledger::{account::Account, genesis::GenesisLedger, public_key::PublicKey, TokenLedger},
        mina_blocks::common::*,
        server::IndexerVersion,
        state::IndexerState,
        store::IndexerStore,
    };
    use serde::Deserialize;
    use std::{collections::HashMap, path::Path};

    /// Creates an indexer from the hardfork genesis ledger
    pub(crate) fn hardfork_genesis_state<P>(path: P) -> anyhow::Result<IndexerState>
    where
        P: AsRef<Path>,
    {
        let indexer_store = std::sync::Arc::new(IndexerStore::new(path.as_ref())?);
        let hardfork_genesis_ledger =
            parse_simple_ledger("./tests/data/genesis_ledgers/hardfork.json")?;

        let state = IndexerState::new(
            hardfork_genesis_ledger,
            IndexerVersion::v2(),
            indexer_store,
            MAINNET_CANONICAL_THRESHOLD,
            10,
            false,
            true,
        )?;

        Ok(state)
    }

    #[derive(Deserialize)]
    struct SimpleGenesisAccount {
        #[serde(deserialize_with = "from_nanomina_str")]
        balance: u64,

        nonce: u32,
        delegate: PublicKey,
        genesis_account: bool,
    }

    struct SimpleAccount {
        public_key: PublicKey,
        account: SimpleGenesisAccount,
    }

    impl From<SimpleAccount> for Account {
        fn from(value: SimpleAccount) -> Self {
            Self {
                public_key: value.public_key,
                nonce: Some(value.account.nonce.into()),
                balance: value.account.balance.into(),
                delegate: value.account.delegate,
                genesis_account: value.account.genesis_account,
                ..Default::default()
            }
        }
    }

    fn parse_simple_ledger<P>(path: P) -> anyhow::Result<GenesisLedger>
    where
        P: AsRef<Path>,
    {
        let data = std::fs::read(path)?;
        let simple_ledger: HashMap<String, SimpleGenesisAccount> = serde_json::from_slice(&data)?;

        let mut ledger = TokenLedger::new();
        for (public_key, account) in simple_ledger.into_iter() {
            let account = SimpleAccount {
                public_key: public_key.into(),
                account,
            };

            ledger
                .accounts
                .insert(account.public_key.clone(), account.into());
        }

        Ok(GenesisLedger { ledger })
    }
}
