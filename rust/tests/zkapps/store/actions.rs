use crate::{generators::*, helpers::store::*};
use mina_indexer::{
    base::{public_key::PublicKey, state_hash::StateHash},
    command::TxnHash,
    ledger::token::TokenAddress,
    mina_blocks::v2::zkapp::action_state::{ActionState, ActionStateWithMeta},
    store::{zkapp::actions::ZkappActionStore, IndexerStore},
};
use quickcheck::Arbitrary;

#[test]
fn action_store_test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("zkapp-action-store")?;
    let indexer_store = IndexerStore::new(store_dir.path(), true)?;

    // generate arbitrary actions
    let g = &mut gen();
    let actions = vec![
        <TestGen<ActionState>>::arbitrary(g).0,
        <TestGen<ActionState>>::arbitrary(g).0,
        <TestGen<ActionState>>::arbitrary(g).0,
    ];
    let actions_length = actions.len() as u32;

    // set block/txn
    let state_hash = StateHash::default();
    let block_height = u32::arbitrary(g);
    let txn_hash = TxnHash::default();

    // set token account
    let pk = PublicKey::default();
    let token = TokenAddress::default();

    /////////////////
    // add actions //
    /////////////////

    // before
    assert_eq!(None, indexer_store.get_num_actions(&pk, &token)?);

    let actions_added =
        indexer_store.add_actions(&pk, &token, &actions, &state_hash, block_height, &txn_hash)?;
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
            ActionStateWithMeta {
                action,
                block_height,
                txn_hash: txn_hash.clone(),
                state_hash: state_hash.clone(),
            }
        );
    }

    ////////////////
    // set action //
    ////////////////

    let index = u32::arbitrary(g);
    let index = index % actions_length;

    let set_action = <TestGen<ActionState>>::arbitrary(g).0;
    let set_action = ActionStateWithMeta {
        action: set_action,
        txn_hash,
        state_hash,
        block_height,
    };

    indexer_store.set_action(&pk, &token, &set_action, index)?;
    assert_eq!(
        set_action,
        indexer_store.get_action(&pk, &token, index)?.unwrap()
    );

    ////////////////////
    // remove actions //
    ////////////////////

    let num = u32::arbitrary(g);
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
