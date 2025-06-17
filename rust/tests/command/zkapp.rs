use crate::helpers::{state::hardfork_genesis_state, store::*};
use mina_indexer::{
    base::state_hash::StateHash,
    block::parser::BlockParser,
    command::{signed::SignedCommandWithData, store::UserCommandStore, TxnHash},
    utility::store::common::U32_LEN,
};
use speedb::IteratorMode;
use std::path::PathBuf;

#[tokio::test]
async fn zkapp_command_iterator() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("zkapp-command-store")?;
    let blocks_dir = &PathBuf::from("./tests/data/hardfork");

    // start with the hardfork genesis ledger
    let mut state = hardfork_genesis_state(store_dir.path())?;
    let mut bp = BlockParser::new_testing(blocks_dir)?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    let store = state.indexer_store.unwrap();

    // block height sorted
    let mut height_count = 0;
    for (key, value) in store
        .zkapp_commands_height_iterator(IteratorMode::End)
        .flatten()
    {
        // from value
        let expect: SignedCommandWithData = serde_json::from_slice(&value)?;

        // from user commands
        let txn_hash = TxnHash::from_bytes(key[U32_LEN..][..TxnHash::V1_LEN].to_vec())?;
        let state_hash = StateHash::from_bytes(&key[U32_LEN..][TxnHash::V1_LEN..])?;
        let zkapp_command = store
            .get_user_command_state_hash(&txn_hash, &state_hash)?
            .unwrap();

        height_count += 1;

        assert!(zkapp_command.is_zkapp_command());
        assert_eq!(zkapp_command, expect);
    }

    assert!(height_count > 0);
    Ok(())
}
