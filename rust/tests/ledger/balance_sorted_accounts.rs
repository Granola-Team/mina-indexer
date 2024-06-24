use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion},
    constants::*,
    ledger::{
        genesis::{GenesisLedger, GenesisRoot},
        store::LedgerStore,
    },
    server::IndexerVersion,
    state::IndexerState,
    store::{account::AccountStore, balance_of_key, pk_of_key, IndexerStore},
};
use std::{path::PathBuf, sync::Arc};

#[test]
fn check_balance() -> anyhow::Result<()> {
    const MAX_BLOCK_LENGTH_FILTER: u32 = 8;

    let store_dir = setup_new_db_dir("balance-sorted-db")?;
    let blocks_dir = &PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)
            .unwrap();
    let mut state = IndexerState::new(
        genesis_ledger.clone().into(),
        IndexerVersion::new_testing(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
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

    // check sorted store balances equal best ledger balances
    let mut curr_ledger_balance = None;
    let best_ledger = indexer_store.get_best_ledger()?.unwrap();
    for (n, (key, _)) in indexer_store
        .account_balance_iterator(speedb::IteratorMode::End)
        .flatten()
        .enumerate()
    {
        let pk = pk_of_key(&key[8..]);
        let pk_key_balance = balance_of_key(&key);
        let pk_store_balance = indexer_store.get_account_balance(&pk)?.unwrap();
        let pk_ledger_balance = best_ledger.accounts.get(&pk).unwrap().balance.0;

        println!(
            "(n: {n}) {pk}: {pk_store_balance} (store), {pk_ledger_balance} (ledger), {pk_key_balance} (key)"
        );

        // store balance coincides with best ledger balance
        assert_eq!(pk_store_balance, pk_ledger_balance);

        // store balance coincides with key balance
        assert_eq!(pk_store_balance, pk_key_balance);

        // best ledger balances decreasing
        assert!(curr_ledger_balance.unwrap_or(u64::MAX) >= pk_ledger_balance);
        curr_ledger_balance = Some(pk_ledger_balance);
    }

    // check best ledger balances equal sorted store balances
    for (pk, acct) in best_ledger.accounts {
        assert_eq!(
            acct.balance.0,
            indexer_store.get_account_balance(&pk)?.unwrap()
        );
    }

    Ok(())
}
