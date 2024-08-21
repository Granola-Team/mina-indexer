use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion, BlockHash},
    constants::*,
    ledger::{
        genesis::{GenesisLedger, GenesisRoot},
        store::{best::BestLedgerStore, staged::StagedLedgerStore},
    },
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn check_staged_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("staged-ledger-balance-sorted-db")?;
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
        None,
    )?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;
    let state_hash = BlockHash::from("3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ");

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    // check "best" staged ledger equals best ledger
    let best_ledger = indexer_store.build_best_ledger()?.unwrap();
    let staged_ledger = indexer_store
        .get_staged_ledger_at_state_hash(&state_hash, false)?
        .unwrap();

    if best_ledger != staged_ledger {
        for (pk, staged_acct) in staged_ledger.accounts.iter() {
            let staged_ledger_acct = indexer_store
                .get_staged_account(pk.clone(), state_hash.clone())?
                .unwrap();
            assert_eq!(*staged_acct, staged_ledger_acct);
            assert_eq!(staged_acct, best_ledger.accounts.get(pk).unwrap());
        }
        panic!("Ledgers do not match")
    }
    Ok(())
}
