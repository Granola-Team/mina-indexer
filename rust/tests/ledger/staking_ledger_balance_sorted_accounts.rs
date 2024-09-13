use crate::helpers::setup_new_db_dir;
use anyhow::Context;
use mina_indexer::{
    constants::*,
    ledger::{
        genesis::{GenesisLedger, GenesisRoot},
        store::staking::StakingLedgerStore,
    },
    server::IndexerVersion,
    state::IndexerState,
    store::{staking_ledger_store_impl::split_staking_ledger_sort_key, IndexerStore},
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn check_staking_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("staking-ledger-balance-sorted-db")?;
    let ledgers_dir = PathBuf::from("./tests/data/staking_ledgers");
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
    let epoch = 0;

    // ingest the blocks
    state
        .add_startup_staking_ledgers_to_store(&ledgers_dir)
        .await?;

    // check sorted store balances equal best ledger balances
    let mut curr_ledger_balance = None;
    let staking_ledger = indexer_store.build_staking_ledger(epoch, None)?.unwrap();
    for (n, (key, _)) in indexer_store
        .staking_ledger_account_balance_iterator(epoch, speedb::Direction::Reverse)
        .flatten()
        .enumerate()
    {
        let (key_epoch, balance, pk) = split_staking_ledger_sort_key(&key)?;
        if key_epoch != epoch {
            panic!("Only epoch 0 staking ledger present");
        }

        let pk_store_account = indexer_store
            .get_staking_account(&pk, epoch, None)?
            .unwrap();
        let pk_staking_account = staking_ledger
            .staking_ledger
            .get(&pk)
            .with_context(|| format!("pk: {pk}"))
            .unwrap();

        if pk_store_account != *pk_staking_account || pk_store_account.balance != balance {
            println!(
                "(n: {n}) {pk}: {} (store), {} (ledger), {balance} (key)",
                pk_store_account.balance, pk_staking_account.balance
            );
        }

        // store balance coincides with best ledger balance
        assert_eq!(pk_store_account, *pk_staking_account);

        // store balance coincides with key balance
        assert_eq!(pk_store_account.balance, balance);

        // staking ledger balances decreasing
        assert!(curr_ledger_balance.unwrap_or(u64::MAX) >= pk_staking_account.balance);
        curr_ledger_balance = Some(pk_staking_account.balance);
    }

    // check staking ledger accounts equal balance-sorted store accounts
    for (pk, acct) in staking_ledger.staking_ledger {
        assert_eq!(
            acct,
            indexer_store
                .get_staking_account(&pk, epoch, None)?
                .unwrap()
        );
    }
    Ok(())
}
