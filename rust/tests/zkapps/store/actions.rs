use crate::{generators::TestGen, helpers::store::*};
use mina_indexer::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    mina_blocks::v2::ActionState,
    store::{zkapp::actions::ZkappActionStore, IndexerStore},
};
use quickcheck::{Arbitrary, Gen};

#[test]
fn action_store_test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("zkapp-action-store")?;
    let indexer_store = IndexerStore::new(store_dir.path())?;

    // generate arbitrary actions
    let mut gen = Gen::new(100);
    let actions = vec![
        <TestGen<ActionState>>::arbitrary(&mut gen).0,
        <TestGen<ActionState>>::arbitrary(&mut gen).0,
        <TestGen<ActionState>>::arbitrary(&mut gen).0,
    ];
    let actions_length = actions.len() as u32;

    // set token account
    let pk = PublicKey::default();
    let token = TokenAddress::default();

    /////////////////
    // add actions //
    /////////////////

    // before
    assert_eq!(None, indexer_store.get_num_actions(&pk, &token)?);

    let actions_added = indexer_store.add_actions(&pk, &token, &actions)?;
    assert_eq!(actions_added, actions_length);

    // after
    assert_eq!(
        actions_added,
        indexer_store.get_num_actions(&pk, &token)?.unwrap()
    );

    /////////////////
    // get actions //
    /////////////////

    for (idx, action) in actions.iter().cloned().enumerate() {
        assert_eq!(
            indexer_store.get_action(&pk, &token, idx as u32)?.unwrap(),
            action
        );
    }

    ////////////////
    // set action //
    ////////////////

    let index: u32 = Arbitrary::arbitrary(&mut gen);
    let index = index % actions_length;
    let set_action = <TestGen<ActionState>>::arbitrary(&mut gen).0;

    indexer_store.set_action(&pk, &token, &set_action, index)?;
    assert_eq!(
        set_action,
        indexer_store.get_action(&pk, &token, index)?.unwrap()
    );

    ////////////////////
    // remove actions //
    ////////////////////

    let num: u32 = Arbitrary::arbitrary(&mut gen);
    let num = num % actions_length;

    assert_eq!(
        indexer_store.remove_actions(&pk, &token, num)?,
        actions_length - num
    );

    // check remaining number
    assert_eq!(
        indexer_store.get_num_actions(&pk, &token)?.unwrap(),
        actions_length - num
    );

    Ok(())
}
