use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    command::{signed::SignedCommand, store::CommandStore},
    constants::*,
    ledger::genesis::parse_file,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn add_and_get() {
    let store_dir = setup_new_db_dir("command-store").unwrap();
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");

    let indexer_store = Arc::new(IndexerStore::new(store_dir.path()).unwrap());
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path).unwrap();
    let indexer = IndexerState::new(
        genesis_root.into(),
        indexer_store.clone(),
        MAINNET_TRANSITION_FRONTIER_K,
    )
    .unwrap();

    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .unwrap();
    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let (block, _) = bp.get_precomputed_block(state_hash).await.unwrap();
    let block_cmds = block.commands();
    let pks = block.all_command_public_keys();

    // add the block to the block store
    indexer.add_block_to_store(&block).unwrap();

    // check state hash key
    let result_cmds = indexer_store
        .as_ref()
        .get_commands_in_block(&state_hash.into())
        .unwrap();
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
            .get_commands_for_public_key(&pk)
            .unwrap()
            .into_iter()
            .map(SignedCommand::from)
            .collect();
        assert_eq!(result_pk_cmds, pk_cmds);
    }

    // check transaction hash key
    for cmd in SignedCommand::from_precomputed(&block) {
        let result_cmd: SignedCommand = indexer_store
            .get_command_by_hash(&cmd.hash_signed_command().unwrap())
            .unwrap()
            .unwrap()
            .into();
        assert_eq!(result_cmd, cmd);
    }
}
