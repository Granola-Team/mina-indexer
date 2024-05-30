use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
    },
    command::{signed::SignedCommand, store::UserCommandStore},
    constants::*,
    ledger::genesis::parse_file,
    server::IndexerVersion,
    state::IndexerState,
    store::*,
};
use speedb::IteratorMode;
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn add_and_get() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("command-store")?;
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path)?;
    let indexer = IndexerState::new(
        genesis_root.into(),
        IndexerVersion::new_testing(),
        indexer_store.clone(),
        MAINNET_TRANSITION_FRONTIER_K,
    )?;

    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )?;

    // add the first block to the store
    if let Some((block, _)) = bp.next_block()? {
        let block: PrecomputedBlock = block.into();
        indexer.add_block_to_store(&block)?;
    }

    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let (block, _) = bp.get_precomputed_block(state_hash).await?;
    let block_cmds = block.commands();
    let pks = block.all_command_public_keys();

    // add another block to the store
    indexer.add_block_to_store(&block)?;

    // check state hash key
    let result_cmds = indexer_store
        .as_ref()
        .get_user_commands_in_block(&state_hash.into())?;
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
            .into_iter()
            .map(SignedCommand::from)
            .collect();
        assert_eq!(result_pk_cmds, pk_cmds);
    }

    // check transaction hash key
    for cmd in SignedCommand::from_precomputed(&block) {
        let result_cmd: SignedCommand = indexer_store
            .get_user_command_by_hash(&cmd.hash_signed_command()?)?
            .unwrap()
            .into();
        assert_eq!(result_cmd, cmd);
    }

    // iterate over transactions
    let mut curr_slot = 0;
    for (key, value) in user_commands_iterator(&indexer_store, IteratorMode::End).flatten() {
        let signed_cmd = user_commands_iterator_signed_command(&value)?;

        // txn hashes should match
        assert_eq!(user_commands_iterator_txn_hash(&key)?, signed_cmd.tx_hash);

        // global slot numbers should match
        let cmd_slot = user_commands_iterator_global_slot(&key);
        assert!(curr_slot <= cmd_slot);
        assert_eq!(cmd_slot, signed_cmd.global_slot_since_genesis,);

        // blocks should be present
        let state_hash = signed_cmd.state_hash;
        assert!(indexer_store.get_block(&state_hash)?.is_some());

        curr_slot = cmd_slot;
    }
    Ok(())
}
