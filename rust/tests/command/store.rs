use crate::helpers::store::*;
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
    },
    command::{signed::SignedCommand, store::UserCommandStore, UserCommandWithStatusT},
    constants::*,
    ledger::genesis::GenesisLedger,
    server::IndexerVersion,
    state::IndexerState,
    store::*,
    utility::store::command::user::{
        user_commands_iterator_state_hash, user_commands_iterator_txn_hash,
        user_commands_iterator_u32_prefix,
    },
};
use speedb::IteratorMode;
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn add_and_get() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("command-store")?;
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");

    let indexer_store = Arc::new(IndexerStore::new(store_dir.path(), true)?);
    let genesis_ledger = GenesisLedger::new_v1()?;

    let mut indexer = IndexerState::new(
        genesis_ledger,
        IndexerVersion::default(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
        false,
    )?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    // add the first block to the store
    if let Some((block, block_bytes)) = bp.next_block().await? {
        let block: PrecomputedBlock = block.into();
        indexer.add_block_to_store(&block, block_bytes, true)?;
    }

    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let (block, block_bytes) = bp.get_precomputed_block(state_hash).await?;
    let block_cmds = block.commands();
    let pks = block.all_command_public_keys();

    // add another block to the store
    indexer.add_block_to_store(&block, block_bytes, true)?;

    // check state hash key
    let result_cmds = indexer_store
        .as_ref()
        .get_block_user_commands(&state_hash.into())?
        .unwrap_or_default();
    assert_eq!(result_cmds, block_cmds);

    // check each pk key
    for pk in pks {
        let pk_cmds: Vec<SignedCommand> = block_cmds
            .iter()
            .cloned()
            .map(SignedCommand::from)
            .filter(|x| x.contains_public_key(&pk))
            .collect();
        let result_pk_cmds: Vec<SignedCommand> = indexer_store
            .as_ref()
            .get_user_commands_for_public_key(&pk)?
            .unwrap_or_default()
            .into_iter()
            .map(SignedCommand::from)
            .collect();
        assert_eq!(result_pk_cmds, pk_cmds);
    }

    // check transaction hash key
    for cmd in block.commands() {
        let result_cmd: Option<SignedCommand> = indexer_store
            .get_user_command(&cmd.txn_hash()?, 0)?
            .map(|c| c.into());
        let signed: SignedCommand = cmd.into();

        assert_eq!(result_cmd, Some(signed));
    }

    // iterate over transactions via block height
    let mut curr_height = 0;
    for (key, _) in indexer_store
        .user_commands_height_iterator(IteratorMode::End)
        .flatten()
    {
        let txn_hash = user_commands_iterator_txn_hash(&key)?;
        let state_hash = user_commands_iterator_state_hash(&key)?;
        let signed_cmd = indexer_store
            .get_user_command_state_hash(&txn_hash, &state_hash)?
            .unwrap();

        // txn hashes should match
        assert_eq!(txn_hash, signed_cmd.txn_hash);

        // block heights should match
        let cmd_height = user_commands_iterator_u32_prefix(&key);
        assert!(curr_height <= cmd_height);
        assert_eq!(cmd_height, signed_cmd.blockchain_length);

        // blocks should be present
        let state_hash = signed_cmd.state_hash;
        assert!(indexer_store.get_block(&state_hash)?.is_some());

        curr_height = cmd_height;
    }

    Ok(())
}
