use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::{genesis::GenesisBlock, store::BlockStore},
    constants::*,
};

#[test]
fn genesis_v1() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v1")?;
    let state = mainnet_genesis_state(store_dir.as_ref())?;

    // check v1 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&MAINNET_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(GenesisBlock::new_v1().unwrap().to_precomputed())
    );

    Ok(())
}

#[test]
fn genesis_v2_add() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v2-add")?;
    let mut state = mainnet_genesis_state(store_dir.as_ref())?;

    // add v2 genesis block
    let GenesisBlock(block, block_bytes) = GenesisBlock::new_v2()?;

    assert!(state.block_pipeline(&block, block_bytes)?);

    // check v2 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&HARDFORK_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(block)
    );

    Ok(())
}

#[test]
fn genesis_v2_start() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v2-start")?;
    let state = hardfork_genesis_state(store_dir.path())?;

    // check v2 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&HARDFORK_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(GenesisBlock::new_v2().unwrap().to_precomputed())
    );

    Ok(())
}
