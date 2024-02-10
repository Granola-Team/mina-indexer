use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::Block,
    event::{block::*, db::*, ledger::*, store::*, witness_tree::*, *},
    store::IndexerStore,
};

#[test]
fn add_and_get_events() {
    let store_dir = setup_new_db_dir("event-store").unwrap();
    let db = IndexerStore::new(store_dir.path()).unwrap();

    let event0 = IndexerEvent::BlockWatcher(BlockWatcherEvent::SawBlock {
        state_hash: "block0".into(),
        path: ".".into(),
    });
    let event1 = IndexerEvent::BlockWatcher(BlockWatcherEvent::WatchDir("./block0".into()));
    let event2 = IndexerEvent::Db(DbEvent::Block(DbBlockEvent::AlreadySeenBlock {
        blockchain_length: 0,
        state_hash: "state_hash".into(),
    }));
    let event3 = IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBlock {
        state_hash: "hash".into(),
        blockchain_length: 0,
    }));
    let event4 = IndexerEvent::Db(DbEvent::Ledger(DbLedgerEvent::AlreadySeenLedger(
        "hash".into(),
    )));
    let event5 = IndexerEvent::Db(DbEvent::Ledger(DbLedgerEvent::NewLedger {
        hash: "hash".into(),
    }));
    let event6 = IndexerEvent::Db(DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
        blockchain_length: 0,
        state_hash: "hash".into(),
    }));
    let event7 = IndexerEvent::LedgerWatcher(LedgerWatcherEvent::NewLedger {
        hash: "hash".into(),
        path: "./path".into(),
    });
    let event8 = IndexerEvent::LedgerWatcher(LedgerWatcherEvent::WatchDir("./path".into()));
    let block = Block {
        parent_hash: "parent_hash".into(),
        state_hash: "state_hash".into(),
        height: 0,
        blockchain_length: 1,
        global_slot_since_genesis: 0,
        last_vrf_output: "last_vrf_output".into(),
    };
    let event9 = IndexerEvent::WitnessTree(WitnessTreeEvent::UpdateBestTip(block.clone()));
    let event10 = IndexerEvent::WitnessTree(WitnessTreeEvent::UpdateCanonicalChain {
        best_tip: block,
        canonical_blocks: CanonicalBlocksEvent::CanonicalBlocks(vec![]),
    });

    // block, db, and ledger events are recorded
    assert_eq!(db.add_event(&event0).unwrap(), 1);
    assert_eq!(db.add_event(&event1).unwrap(), 2);
    assert_eq!(db.add_event(&event2).unwrap(), 3);
    assert_eq!(db.add_event(&event3).unwrap(), 4);
    assert_eq!(db.add_event(&event4).unwrap(), 5);
    assert_eq!(db.add_event(&event5).unwrap(), 6);
    assert_eq!(db.add_event(&event6).unwrap(), 7);
    assert_eq!(db.add_event(&event7).unwrap(), 8);
    assert_eq!(db.add_event(&event8).unwrap(), 9);
    // witness tree events aren't recorded
    assert_eq!(db.add_event(&event9).unwrap(), 9);
    assert_eq!(db.add_event(&event10).unwrap(), 9);

    let next_seq_num = db.get_next_seq_num().unwrap();
    assert_eq!(next_seq_num, 9);

    let event_log = db.get_event_log().unwrap();
    assert_eq!(
        event_log,
        vec![event0, event1, event2, event3, event4, event5, event6, event7, event8]
    );
}
