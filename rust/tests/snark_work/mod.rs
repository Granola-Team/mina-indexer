use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PcbVersion},
    constants::*,
    ledger::genesis::parse_file,
    server::IndexerVersion,
    snark_work::{store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash},
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn store() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("snark-store")?;
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger_path = &PathBuf::from("./data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path)?;
    let mut indexer = IndexerState::new(
        genesis_root.into(),
        IndexerVersion::default(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
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
    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let (block, block_bytes) = bp.get_precomputed_block(state_hash).await?;
    let block_snarks = SnarkWorkSummary::from_precomputed(&block);
    let block_snarks_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(&block);

    // add the block to the block store
    indexer.add_block_to_store(&block, block_bytes, true)?;

    // check state hash key
    let result_snarks = indexer_store
        .as_ref()
        .get_snark_work_in_block(&state_hash.into())?
        .unwrap();
    assert_eq!(result_snarks, block_snarks);

    // check each pk key
    for pk in block.prover_keys() {
        let pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = block_snarks_state_hash
            .iter()
            .filter(|x| x.contains_pk(&pk))
            .cloned()
            .collect();
        let result_pk_snarks: Vec<SnarkWorkSummaryWithStateHash> =
            indexer_store.as_ref().get_snark_work_by_public_key(&pk)?;
        assert_eq!(result_pk_snarks, pk_snarks);
    }
    Ok(())
}
