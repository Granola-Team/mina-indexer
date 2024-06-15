use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion},
    constants::*,
    ledger::{
        account::Account,
        genesis::{GenesisLedger, GenesisRoot},
        public_key::PublicKey,
        store::LedgerStore,
    },
    server::IndexerVersion,
    state::IndexerState,
    store::{account::AccountStore, IndexerStore},
};
use std::{path::PathBuf, sync::Arc};

#[test]
fn check_balance() -> anyhow::Result<()> {
    const MAX_BLOCK_LENGTH_FILTER: u32 = 8;

    let store_dir = setup_new_db_dir("balance-sorted-db")?;
    let blocks_dir = &PathBuf::from("../tests/data/initial-blocks");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)
            .unwrap();
    let mut state = IndexerState::new(
        genesis_ledger.clone().into(),
        IndexerVersion::new_testing(),
        indexer_store.clone(),
        10,
    )?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery_filtered(
        blocks_dir,
        PcbVersion::V1,
        MAX_BLOCK_LENGTH_FILTER,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )?;

    // ingest the blocks
    state.add_blocks(&mut bp)?;

    // check best ledger balances equal sorted balances
    for (n, (key, _)) in indexer_store
        .account_balance_iterator(speedb::IteratorMode::End)
        .flatten()
        .enumerate()
    {
        let pk = PublicKey::from_bytes(&key[8..])?;
        let balance = indexer_store.get_account_balance(&pk)?.unwrap();
        println!("(n: {n}) {pk}: {balance}");

        assert_eq!(
            balance,
            indexer_store
                .get_best_ledger()?
                .unwrap()
                .accounts
                .get(&pk)
                .map_or(0, |acct| acct.balance.0)
        );
    }

    // check sorted balances match best ledger balances
    let mut accounts: Vec<(PublicKey, Account)> =
        state.best_ledger()?.unwrap().accounts.into_iter().collect();
    accounts.sort_by(|y, x| {
        let bal_cmp = x.1.balance.0.cmp(&y.1.balance.0);
        if bal_cmp == std::cmp::Ordering::Equal {
            x.0 .0.cmp(&y.0 .0)
        } else {
            bal_cmp
        }
    });

    for (pk, acct) in accounts {
        assert_eq!(
            acct.balance.0,
            indexer_store.get_account_balance(&pk)?.unwrap(),
            "{{ pk: {pk}, diffs: [{}, {}] }}",
            indexer_store
                .get_account_balance(&pk)?
                .unwrap()
                .saturating_sub(acct.balance.0),
            acct.balance
                .0
                .saturating_sub(indexer_store.get_account_balance(&pk)?.unwrap()),
        );
    }

    Ok(())
}
