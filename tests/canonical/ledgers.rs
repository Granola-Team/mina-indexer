use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore, BlockHash},
    canonical::store::CanonicityStore,
    state::{
        ledger::{
            diff::LedgerDiff, genesis::GenesisRoot, public_key::PublicKey, store::LedgerStore,
        },
        IndexerState,
    },
    store::IndexerStore,
    MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs, path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let db_path = PathBuf::from("./test_canonical_ledgers_store");
    let indexer_store = Arc::new(IndexerStore::new(&db_path).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)
        .unwrap()
        .ledger;
    let mut state = IndexerState::new(
        BlockHash(MAINNET_GENESIS_HASH.to_string()),
        genesis_ledger,
        indexer_store,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    while let Some(precomputed_block) = block_parser.next_block().unwrap() {
        state.add_block(&precomputed_block).unwrap();
    }

    let indexer_store = state.indexer_store.as_ref().unwrap();
    let mut ledger_diff = indexer_store.get_ledger_at_height(1).unwrap().unwrap();
    let mut ledger_post = indexer_store.get_ledger_at_height(1).unwrap().unwrap();

    for n in 2..=3 {
        let state_hash = indexer_store
            .get_canonical_hash_at_height(n)
            .unwrap()
            .unwrap();
        let block = indexer_store.get_block(&state_hash).unwrap().unwrap();
        let ledger = indexer_store.get_ledger(&state_hash).unwrap().unwrap();

        ledger_post.apply_post_balances(&block);
        ledger_diff
            .apply_diff(&LedgerDiff::from_precomputed_block(&block))
            .unwrap();

        if ledger != ledger_post || ledger != ledger_diff {
            let mut keys: Vec<&PublicKey> = ledger.accounts.keys().collect();
            let mut keys_post: Vec<&PublicKey> = ledger_post.accounts.keys().collect();
            let mut keys_diff: Vec<&PublicKey> = ledger_diff.accounts.keys().collect();

            keys.sort();
            keys_post.sort();
            keys_diff.sort();

            assert_eq!(
                keys.len(),
                keys_post.len(),
                "Different number of keys (post)!"
            );
            assert_eq!(
                keys.len(),
                keys_diff.len(),
                "Different number of keys (diff)!"
            );

            for (n, pk) in keys.iter().enumerate() {
                let pk_post = keys_post.get(n).unwrap();
                let pk_diff = keys_diff.get(n).unwrap();
                let ledger_balance =
                    |pk: &PublicKey| ledger.accounts.get(pk).map(|acct| acct.balance.0);
                let ledger_diff_balance =
                    |pk: &PublicKey| ledger_diff.accounts.get(pk).map(|acct| acct.balance.0);
                let ledger_post_balance =
                    |pk: &PublicKey| ledger_post.accounts.get(pk).map(|acct| acct.balance.0);

                if pk != pk_diff {
                    println!("*********************");
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
                    "Different balances (diff)"
                );
                assert_eq!(
                    ledger_balance(pk),
                    ledger_post_balance(pk),
                    "Different balances (post)"
                );
                assert_eq!(pk, pk_diff, "Different keys (diff)");
                assert_eq!(pk, pk_post, "Different keys (post)");
            }
        }

        assert!(ledger == ledger_diff, "Different ledgers (diff)");
        assert!(ledger == ledger_post, "Different ledgers (post)");
    }

    fs::remove_dir_all(db_path).unwrap();
}
