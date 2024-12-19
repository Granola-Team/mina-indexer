use crate::helpers::setup_new_db_dir;
use anyhow::Context;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion},
    constants::*,
    ledger::{
        account::Account,
        genesis::{GenesisLedger, GenesisRoot},
        store::best::BestLedgerStore,
    },
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
    utility::store::ledger::best::split_best_account_sort_key,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn check_best_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("best-ledger-balance-sorted-db")?;
    let blocks_dir = &PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)?;
    let genesis_ledger: GenesisLedger = genesis_ledger.into();
    let mut state = IndexerState::new(
        genesis_ledger.clone(),
        IndexerVersion::default(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
        false,
    )?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    // check sorted store balances equal best ledger balances
    let mut curr_ledger_balance = None;
    let best_ledger = indexer_store.build_best_ledger()?.unwrap();
    for (n, (key, value)) in indexer_store
        .best_ledger_account_balance_iterator(speedb::IteratorMode::End)
        .flatten()
        .enumerate()
    {
        let (token, balance, pk) = split_best_account_sort_key(&key).unwrap();

        let pk_value_account = serde_json::from_slice::<Account>(&value)?;
        let pk_store_account = indexer_store.get_best_account(&pk, &token)?.unwrap();
        let pk_best_account = best_ledger
            .get_account(&pk, &token)
            .with_context(|| format!("pk: {pk}"))
            .unwrap()
            .clone();

        if pk_store_account != pk_best_account
            || pk_best_account != pk_value_account
            || pk_store_account.balance.0 != balance
        {
            println!(
                "(n: {n}) {pk}: {} (store), {} (ledger), {} (value), {balance} (key)",
                pk_store_account.balance.0, pk_best_account.balance.0, pk_value_account.balance.0
            );
        }

        // store balance coincides with best ledger balance
        assert_eq!(pk_store_account, pk_best_account);

        // store balance coincides with key balance
        assert_eq!(pk_store_account.balance.0, balance);

        // best ledger balances decreasing
        assert!(curr_ledger_balance.unwrap_or(u64::MAX) >= pk_best_account.balance.0);
        curr_ledger_balance = Some(pk_best_account.balance.0);
    }

    // check best ledger balances equal sorted store balances
    for (token, token_ledger) in best_ledger.tokens.iter() {
        for (pk, best_acct) in token_ledger.accounts.iter() {
            assert_eq!(
                *best_acct,
                indexer_store.get_best_account(pk, token)?.unwrap()
            );
        }
    }

    Ok(())
}
