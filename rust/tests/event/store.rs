use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{vrf_output::VrfOutput, Block, BlockHash},
    event::{db::*, store::*, witness_tree::*, *},
    ledger::LedgerHash,
    store::IndexerStore,
};

#[test]
fn add_and_get_events() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-store")?;
    let db = IndexerStore::new(store_dir.path())?;
    let event0 = IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBlock {
        blockchain_length: 23,
        state_hash: BlockHash::default(),
    }));
    let event1 = IndexerEvent::Db(DbEvent::Ledger(DbLedgerEvent::NewLedger {
        ledger_hash: LedgerHash::default(),
        state_hash: BlockHash::default(),
        blockchain_length: 42,
    }));
    let event2 = IndexerEvent::Db(DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
        blockchain_length: 0,
        state_hash: BlockHash::default(),
    }));
    let block = Block {
        parent_hash: "parent_hash".into(),
        state_hash: "state_hash".into(),
        genesis_state_hash: "genesis_hash".into(),
        height: 0,
        blockchain_length: 1,
        global_slot_since_genesis: 0,
        hash_last_vrf_output: VrfOutput::new("last_vrf_output".as_bytes().to_vec()),
    };
    let event3 = IndexerEvent::WitnessTree(WitnessTreeEvent::UpdateBestTip {
        best_tip: block.clone(),
        canonical_blocks: vec![block],
    });

    // block, db, and ledger events are recorded
    assert_eq!(db.add_event(&event0)?, 1);
    assert_eq!(db.add_event(&event1)?, 2);
    assert_eq!(db.add_event(&event2)?, 3);

    // witness tree events aren't recorded
    assert_eq!(db.add_event(&event3)?, 3);

    // expected event log
    let next_seq_num = db.get_next_seq_num()?;
    let event_log = db.get_event_log()?;
    assert_eq!(next_seq_num, event_log.len() as u32);
    assert_eq!(event_log, vec![event0, event1, event2]);
    Ok(())
}
