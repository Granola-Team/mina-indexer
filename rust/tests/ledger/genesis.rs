use crate::helpers::{
    state::{hardfork_genesis_state, mainnet_genesis_state},
    store::setup_new_db_dir,
};
use mina_indexer::ledger::{account::Timing, token::TokenAddress};

#[test]
fn genesis_v1_timing() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("ledger-genesis-timing-v1")?;
    let state = mainnet_genesis_state(store_dir.as_ref())?;

    let account = state
        .ledger
        .get_account(
            &"B62qrBYNNHZSLNZwaY4FZVNkesEPkFbZfq3YUTa4ZyqRkz1aN86BUFN".into(),
            &TokenAddress::default(),
        )
        .unwrap();
    let expect = Timing {
        cliff_time: 36480.into(),
        vesting_period: 1.into(),
        cliff_amount: 187653487488548.into(),
        vesting_increment: 5144010074.into(),
        initial_minimum_balance: 222221235183807.into(),
    };

    assert_eq!(*account.timing.as_ref().unwrap(), expect);
    Ok(())
}

#[test]
fn genesis_v2_timing() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("ledger-genesis-timing-v2")?;
    let state = hardfork_genesis_state(store_dir.as_ref())?;

    let account = state
        .ledger
        .get_account(
            &"B62qrBYNNHZSLNZwaY4FZVNkesEPkFbZfq3YUTa4ZyqRkz1aN86BUFN".into(),
            &TokenAddress::default(),
        )
        .unwrap();
    let expect = Timing {
        cliff_time: 36480.into(),
        vesting_period: 1.into(),
        cliff_amount: 187653487488548.into(),
        vesting_increment: 5144010074.into(),
        initial_minimum_balance: 222221235183807.into(),
    };

    assert_eq!(*account.timing.as_ref().unwrap(), expect);
    Ok(())
}
