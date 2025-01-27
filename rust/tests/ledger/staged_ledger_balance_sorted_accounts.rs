use crate::helpers::{state::*, store::*};
use anyhow::Context;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion, BlockHash},
    constants::*,
    ledger::store::{best::BestLedgerStore, staged::StagedLedgerStore},
};
use std::path::PathBuf;

#[tokio::test]
async fn check_staged_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("staged-ledger-balance-sorted-db")?;
    let block_dir = &PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let mut state = mainnet_genesis_state(store_dir.as_ref())?;

    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        block_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;
    let state_hash = BlockHash::from("3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ");

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // check "best" staged ledger equals best ledger
    let best_ledger = store.build_best_ledger()?.unwrap();
    let staged_ledger = store
        .get_staged_ledger_at_state_hash(&state_hash, false)?
        .unwrap();

    if best_ledger != staged_ledger {
        for (token, token_ledger) in staged_ledger.tokens.iter() {
            for (pk, staged_acct) in token_ledger.accounts.iter() {
                let staged_ledger_acct = &store
                    .get_staged_account(pk, token, &state_hash)?
                    .with_context(|| format!("\npk: {pk}\ntoken: {token}\nblock: {state_hash}"))
                    .unwrap();

                assert_eq!(staged_acct, staged_ledger_acct);
                assert_eq!(staged_acct, best_ledger.get_account(pk, token).unwrap());
            }
        }

        panic!("Ledgers do not match")
    }

    Ok(())
}
