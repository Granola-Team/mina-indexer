use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore},
    canonicity::store::CanonicityStore,
    ledger::{
        diff::LedgerDiff, public_key::PublicKey, store::staged::StagedLedgerStore,
        token::TokenAddress,
    },
};
use std::path::PathBuf;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("./test_canonical_ledgers_store")?;
    let block_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let mut state = mainnet_genesis_state(store_dir.as_ref())?;
    let mut block_parser = BlockParser::new_testing(&block_dir)?;

    state.add_blocks(&mut block_parser).await?;

    let indexer_store = state.indexer_store.as_ref().unwrap();
    let mut ledger_diff = indexer_store
        .get_staged_ledger_at_block_height(1, false)?
        .unwrap();

    for n in 1..=3 {
        let state_hash = indexer_store.get_canonical_hash_at_height(n)?.unwrap();
        let block = indexer_store.get_block(&state_hash)?.unwrap().0;
        let ledger = indexer_store
            .get_staged_ledger_at_state_hash(&state_hash, false)?
            .unwrap();

        ledger_diff._apply_diff(&LedgerDiff::from_precomputed(&block))?;

        if ledger != ledger_diff {
            let mut keys: Vec<&PublicKey> = ledger
                .tokens
                .get(&TokenAddress::default())
                .map(|token_ledger| token_ledger.accounts.keys().collect())
                .expect("MINA token ledger");
            let mut keys_diff: Vec<&PublicKey> = ledger_diff
                .tokens
                .get(&TokenAddress::default())
                .map(|token_ledger| token_ledger.accounts.keys().collect())
                .expect("MINA token ledger");

            keys.sort();
            keys_diff.sort();

            for (m, k) in keys_diff.iter().enumerate() {
                let key = keys[m];
                if key != *k {
                    println!("{n}: {k}");
                    break;
                }
            }
            assert_eq!(keys.len(), keys_diff.len(), "Different number of keys!");

            for (n, pk) in keys.iter().enumerate() {
                let pk_diff = keys_diff[n];
                let ledger_balance = |pk: &PublicKey| {
                    ledger
                        .tokens
                        .get(&TokenAddress::default())
                        .map(|token_ledger| {
                            token_ledger
                                .accounts
                                .get(pk)
                                .map(|acct| (acct.balance.0, acct.nonce.map_or(0, |n| n.0)))
                        })
                };
                let ledger_diff_balance = |pk: &PublicKey| {
                    ledger_diff
                        .tokens
                        .get(&TokenAddress::default())
                        .map(|token_ledger| {
                            token_ledger
                                .accounts
                                .get(pk)
                                .map(|acct| (acct.balance.0, acct.nonce.map_or(0, |n| n.0)))
                        })
                };

                if *pk != pk_diff {
                    if ledger_balance(pk) != ledger_diff_balance(pk) {
                        println!(
                            "pk:      {pk:?} -> {:?} =/= {:?}",
                            ledger_balance(pk),
                            ledger_diff_balance(pk)
                        );
                    }
                    if ledger_balance(pk_diff) != ledger_diff_balance(pk_diff) {
                        println!(
                            "pk_diff: {pk_diff:?} -> {:?} =/= {:?}",
                            ledger_balance(pk_diff),
                            ledger_diff_balance(pk_diff)
                        );
                    }
                }

                assert_eq!(
                    ledger_balance(pk),
                    ledger_diff_balance(pk),
                    "Different balances (diff): {pk}"
                );
                assert_eq!(*pk, pk_diff, "Different keys!");
            }
        }

        assert!(ledger == ledger_diff, "Different ledgers!");
    }

    Ok(())
}
