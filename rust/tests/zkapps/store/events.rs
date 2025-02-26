use crate::{generators::*, helpers::store::*};
use mina_indexer::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    mina_blocks::v2::ZkappEvent,
    store::{zkapp::events::ZkappEventStore, IndexerStore},
};
use quickcheck::Arbitrary;

#[test]
fn event_store_test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("zkapp-event-store")?;
    let indexer_store = IndexerStore::new(store_dir.path())?;

    // generate arbitrary events
    let g = &mut gen();
    let events = vec![
        <TestGen<ZkappEvent>>::arbitrary(g).0,
        <TestGen<ZkappEvent>>::arbitrary(g).0,
        <TestGen<ZkappEvent>>::arbitrary(g).0,
    ];
    let events_length = events.len() as u32;

    // set token account
    let pk = PublicKey::default();
    let token = TokenAddress::default();

    /////////////////
    // add events //
    /////////////////

    // before
    assert_eq!(None, indexer_store.get_num_events(&pk, &token)?);

    let events_added = indexer_store.add_events(&pk, &token, &events)?;
    assert_eq!(events_added, events_length);

    // after
    assert_eq!(
        events_added,
        indexer_store.get_num_events(&pk, &token)?.unwrap()
    );

    ////////////////
    // get events //
    ////////////////

    for (idx, event) in events.iter().cloned().enumerate() {
        assert_eq!(
            indexer_store.get_event(&pk, &token, idx as u32)?.unwrap(),
            event
        );
    }

    ///////////////
    // set event //
    ///////////////

    let index = u32::arbitrary(g);
    let index = index % events_length;
    let set_event = <TestGen<ZkappEvent>>::arbitrary(g).0;

    indexer_store.set_event(&pk, &token, &set_event, index)?;
    assert_eq!(
        set_event,
        indexer_store.get_event(&pk, &token, index)?.unwrap()
    );

    ///////////////////
    // remove events //
    ///////////////////

    let num = u32::arbitrary(g);
    let num = num % events_length;

    assert_eq!(
        indexer_store.remove_events(&pk, &token, num)?,
        events_length - num
    );

    // check remaining number
    assert_eq!(
        indexer_store.get_num_events(&pk, &token)?.unwrap(),
        events_length - num
    );

    Ok(())
}
