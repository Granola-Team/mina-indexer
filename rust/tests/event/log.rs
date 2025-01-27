use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
    },
    canonicity::store::CanonicityStore,
    constants::*,
    event::{store::EventStore, witness_tree::*},
};
use std::path::PathBuf;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let block_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let store_dir0 = setup_new_db_dir("event-log-store0")?;
    let mut block_parser0 = BlockParser::new_with_canonical_chain_discovery(
        &block_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    let store_dir1 = setup_new_db_dir("event-log-store1")?;
    let mut block_parser1 = BlockParser::new_with_canonical_chain_discovery(
        &block_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    let mut state0 = mainnet_genesis_state(store_dir0.path())?;
    let mut state1 = mainnet_genesis_state(store_dir1.path())?;

    // add parser0 blocks to state0
    state0.add_blocks(&mut block_parser0).await?;

    // add parser1 blocks to state1
    // - add block to db
    // - add block to witness tree
    // - update best tip
    // - update canonicities
    while let Some((block, block_bytes)) = block_parser1.next_block().await? {
        let block: PrecomputedBlock = block.into();
        if let Some(db_event) = state1
            .indexer_store
            .as_ref()
            .map(|store| store.add_block(&block, block_bytes).unwrap())
        {
            if db_event.map(|db| db.is_new_block_event()).unwrap_or(false) {
                if let Some(wt_event) = state1.add_block_to_witness_tree(&block, false, true)?.1 {
                    let (best_tip, new_canonical_blocks) = match wt_event {
                        WitnessTreeEvent::UpdateBestTip {
                            best_tip,
                            canonical_blocks,
                        } => (best_tip, canonical_blocks),
                    };

                    state1.update_best_block_in_store(&best_tip.state_hash)?;
                    new_canonical_blocks.iter().for_each(|block| {
                        if let Some(store) = state1.indexer_store.as_ref() {
                            store
                                .add_canonical_block(
                                    block.blockchain_length,
                                    block.global_slot_since_genesis,
                                    &block.state_hash,
                                    &block.genesis_state_hash,
                                    None,
                                )
                                .unwrap()
                        }
                    });
                    state1.add_block_to_witness_tree(&block, true, true)?;
                }
            }
        }
    }

    let store0 = state0.indexer_store.as_ref().unwrap();
    let store1 = state1.indexer_store.as_ref().unwrap();

    // check event logs match
    let event_log0 = store0
        .event_log_iterator(speedb::IteratorMode::Start)
        .flatten();
    let event_log1 = store1
        .event_log_iterator(speedb::IteratorMode::Start)
        .flatten();

    for ((key0, value0), (key1, value1)) in event_log0.zip(event_log1) {
        assert_eq!(key0, key1);
        assert_eq!(value0, value1);
    }

    Ok(())
}
