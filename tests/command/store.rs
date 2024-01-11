use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    command::{signed::SignedCommand, store::CommandStore},
    ledger::genesis::parse_file,
    state::IndexerState,
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH,
    MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs::remove_dir_all, path::PathBuf, sync::Arc};

#[tokio::test]
async fn add_and_get() {
    let store_dir = setup_new_db_dir("./command-store-test");
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");

    let indexer_store = Arc::new(IndexerStore::new(&store_dir).unwrap());
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path).unwrap();
    let indexer = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_root.ledger,
        indexer_store.clone(),
        MAINNET_TRANSITION_FRONTIER_K,
        PRUNE_INTERVAL_DEFAULT,
        CANONICAL_UPDATE_THRESHOLD,
    )
    .unwrap();

    let mut bp = BlockParser::new(blocks_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    // TODO check command store for transactions at EACH key

    let state_hash = "3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw";
    let block = bp.get_precomputed_block(state_hash).await.unwrap();
    let block_cmds = SignedCommand::from_precomputed(&block);
    let pks = block.block_public_keys();

    // add the block to the block store
    indexer.add_block_to_store(&block).unwrap();

    // check state hash key
    let result_cmds = indexer_store
        .as_ref()
        .get_commands_in_block(&state_hash.into())
        .unwrap()
        .unwrap();
    assert_eq!(result_cmds, block_cmds);

    // check each pk key
    for pk in pks {
        let pk_cmds: Vec<SignedCommand> = block_cmds
            .iter()
            .cloned()
            .filter(|x| x.contains_public_key(&pk))
            .collect();
        let result_pk_cmds = indexer_store
            .as_ref()
            .get_commands_public_key(&pk)
            .unwrap()
            .unwrap();
        assert_eq!(result_pk_cmds, pk_cmds);
    }

    // check transaction hash key
    for cmd in block_cmds {
        let result_cmd = indexer_store
            .get_command_by_hash(&cmd.hash_signed_command().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(result_cmd, cmd);
    }

    remove_dir_all(store_dir).unwrap();
}
