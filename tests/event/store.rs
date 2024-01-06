use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    event::{block::*, db::*, ledger::*, state::*, store::*, *},
    store::IndexerStore,
};
use std::fs::remove_dir_all;

#[tokio::test]
async fn add_and_get_events() {
    let store_dir = setup_new_db_dir("./event-store-test");
    let db = IndexerStore::new(&store_dir).unwrap();

    let event0 = Event::Block(BlockEvent::SawBlock("block0".into()));
    let event1 = Event::Block(BlockEvent::WatchDir("./block0".into()));
    let event2 = Event::Db(DbEvent::Block(DbBlockEvent::AlreadySeenBlock {
        blockchain_length: 0,
        state_hash: "state_hash".into(),
    }));
    let event3 = Event::Db(DbEvent::Block(DbBlockEvent::NewBlock {
        path: "block".into(),
        state_hash: "hash".into(),
        blockchain_length: 0,
    }));
    let event4 = Event::Db(DbEvent::Ledger(DbLedgerEvent::AlreadySeenLedger(
        "hash".into(),
    )));
    let event5 = Event::Db(DbEvent::Ledger(DbLedgerEvent::NewLedger {
        hash: "hash".into(),
        path: "./path".into(),
    }));
    let event6 = Event::Db(DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
        blockchain_length: 0,
        state_hash: "hash".into(),
    }));
    let event7 = Event::Ledger(LedgerEvent::NewLedger {
        hash: "hash".into(),
        path: "./path".into(),
    });
    let event8 = Event::Ledger(LedgerEvent::WatchDir("./path".into()));
    let event9 = Event::State(StateEvent::UpdateCanonicalChain(vec![]));

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
    // state events aren't recorded
    assert_eq!(db.add_event(&event9).unwrap(), 9);

    let next_seq_num = db.get_next_seq_num().unwrap();
    assert_eq!(next_seq_num, 9);

    let event_log = db.get_event_log().unwrap();
    assert_eq!(
        event_log,
        vec![event0, event1, event2, event3, event4, event5, event6, event7, event8]
    );

    remove_dir_all(store_dir).unwrap();
}
