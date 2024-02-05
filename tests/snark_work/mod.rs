use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    constants::{
        CANONICAL_UPDATE_THRESHOLD, LEDGER_CADENCE, MAINNET_CANONICAL_THRESHOLD,
        MAINNET_GENESIS_HASH, MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
    },
    ledger::genesis::parse_file,
    snark_work::{store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash},
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn store() {
    let store_dir = setup_new_db_dir("snark-store").unwrap();
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");

    let indexer_store = Arc::new(IndexerStore::new(store_dir.path()).unwrap());
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path).unwrap();
    let indexer = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_root.into(),
        indexer_store.clone(),
        MAINNET_TRANSITION_FRONTIER_K,
        PRUNE_INTERVAL_DEFAULT,
        CANONICAL_UPDATE_THRESHOLD,
        LEDGER_CADENCE,
    )
    .unwrap();

    let mut bp = BlockParser::new(blocks_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();
    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let block = bp.get_precomputed_block(state_hash).await.unwrap();
    let block_snarks = SnarkWorkSummary::from_precomputed(&block);
    let block_snarks_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(&block);

    // add the block to the block store
    indexer.add_block_to_store(&block).unwrap();

    // check state hash key
    let result_snarks = indexer_store
        .as_ref()
        .get_snark_work_in_block(&state_hash.into())
        .unwrap()
        .unwrap();
    assert_eq!(result_snarks, block_snarks);

    // check each pk key
    for pk in block.prover_keys() {
        let pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = block_snarks_state_hash
            .iter()
            .filter(|x| x.contains_pk(&pk))
            .cloned()
            .collect();
        let result_pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = indexer_store
            .as_ref()
            .get_snark_work_by_public_key(&pk)
            .unwrap()
            .unwrap_or(vec![]);
        assert_eq!(result_pk_snarks, pk_snarks);
    }
}
