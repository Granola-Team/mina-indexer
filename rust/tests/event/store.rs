use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{vrf_output::VrfOutput, Block, BlockHash},
    chain::Network,
    event::{block::*, db::*, ledger::*, store::*, witness_tree::*, *},
    ledger::LedgerHash,
    store::IndexerStore,
};

#[test]
fn add_and_get_events() {
    let store_dir = setup_new_db_dir("event-store").unwrap();
    let db = IndexerStore::new(store_dir.path()).unwrap();

    let event0 = IndexerEvent::BlockWatcher(BlockWatcherEvent::SawBlock {
        blockchain_length: 19,
        network: Network::Testworld,
        state_hash: BlockHash::default(),
    });
    let event1 = IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBlock {
        blockchain_length: 23,
        state_hash: BlockHash::default(),
    }));
    let event2 = IndexerEvent::Db(DbEvent::Ledger(DbLedgerEvent::NewLedger {
        ledger_hash: LedgerHash::default(),
        state_hash: BlockHash::default(),
        blockchain_length: 42,
    }));
    let event3 = IndexerEvent::Db(DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
        blockchain_length: 0,
        state_hash: BlockHash::default(),
    }));
    let event4 = IndexerEvent::StakingLedgerWatcher(StakingLedgerWatcherEvent::NewStakingLedger {
        epoch: 0,
        ledger_hash: LedgerHash("jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".into()),
    });
    let block = Block {
        parent_hash: "parent_hash".into(),
        state_hash: "state_hash".into(),
        height: 0,
        blockchain_length: 1,
        global_slot_since_genesis: 0,
        hash_last_vrf_output: VrfOutput::new("last_vrf_output".as_bytes().to_vec()),
    };
    let event5 = IndexerEvent::WitnessTree(WitnessTreeEvent::UpdateBestTip {
        best_tip: block.clone(),
        canonical_blocks: vec![block],
    });

    // block, db, and ledger events are recorded
    assert_eq!(db.add_event(&event0).unwrap(), 1);
    assert_eq!(db.add_event(&event1).unwrap(), 2);
    assert_eq!(db.add_event(&event2).unwrap(), 3);
    assert_eq!(db.add_event(&event3).unwrap(), 4);
    assert_eq!(db.add_event(&event4).unwrap(), 5);
    // witness tree events aren't recorded
    assert_eq!(db.add_event(&event5).unwrap(), 5);

    let next_seq_num = db.get_next_seq_num().unwrap();
    assert_eq!(next_seq_num, 5);

    let event_log = db.get_event_log().unwrap();
    assert_eq!(event_log, vec![event0, event1, event2, event3, event4]);
}
