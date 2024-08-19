use crate::helpers::setup_new_db_dir;
use anyhow::Context;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion},
    constants::*,
    ledger::{
        genesis::{GenesisLedger, GenesisRoot},
        store::best::BestLedgerStore,
    },
    server::IndexerVersion,
    state::IndexerState,
    store::{balance_key_prefix, pk_key_prefix, IndexerStore},
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn check_balance() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("balance-sorted-db")?;
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
    )?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    // check sorted store balances equal best ledger balances
    let mut curr_ledger_balance = None;
    let best_ledger = indexer_store.get_best_ledger()?.unwrap();
    for (n, (key, _)) in indexer_store
        .account_balance_iterator(speedb::IteratorMode::End)
        .flatten()
        .enumerate()
    {
        let pk = pk_key_prefix(&key[8..]);

        // this account does not exist on the ledger, but is stored in the
        // indexer. We skip any assertions on this account for now
        if pk.0 == *MINA_ACCOUNT_CREATION_FEE_ACCOUNTING_ADDRESS.to_string() {
            continue;
        }
        let pk_key_balance = balance_key_prefix(&key);
        let pk_store_account = indexer_store.get_best_account(&pk)?.unwrap();
        let pk_best_account = best_ledger
            .accounts
            .get(&pk)
            .with_context(|| format!("pk: {pk}"))
            .unwrap();

        if pk_store_account != *pk_best_account || pk_store_account.balance.0 != pk_key_balance {
            println!(
                "(n: {n}) {pk}: {} (store), {} (ledger), {pk_key_balance} (key)",
                pk_store_account.balance.0, pk_best_account.balance.0
            );
        }

        // store balance coincides with best ledger balance
        assert_eq!(pk_store_account, *pk_best_account);

        // store balance coincides with key balance
        assert_eq!(pk_store_account.balance.0, pk_key_balance);

        // best ledger balances decreasing
        assert!(curr_ledger_balance.unwrap_or(u64::MAX) >= pk_best_account.balance.0);
        curr_ledger_balance = Some(pk_best_account.balance.0);
    }

    // check best ledger balances equal sorted store balances
    for (pk, acct) in best_ledger.accounts {
        let best_account = indexer_store.get_best_account(&pk)?.unwrap();
        if pk.0 == *MINA_ACCOUNT_CREATION_FEE_ACCOUNTING_ADDRESS.to_string() {
            // This virtual accounting address is not in the ledger, but exists in our
            // indexer We assert that it is accumulating 1 MINA accounting fees
            // for each new account created
            assert_eq!(best_account.balance.0, 8000000000);
        } else {
            assert_eq!(acct, best_account);
        }
    }
    Ok(())
}
