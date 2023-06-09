use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{branch::Branch, ledger::Ledger},
};

mod simple_improper;
mod simple_proper;

#[tokio::test]
async fn prune_transition_frontier() {
    // 0
    // |    1
    // 1 => |
    // |    2
    // 2

    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let root_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    let mut root_tree = Branch::new(&root_block, Ledger::new()).unwrap();

    let middle_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        middle_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );
    root_tree
        .simple_extension(&middle_block)
        .expect("new leaf should be inserted");

    let child_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        child_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );
    root_tree
        .simple_extension(&child_block)
        .expect("new leaf should be inserted");

    root_tree.prune_transition_frontier(1);

    let pruned_root_block = root_tree.root;

    assert_eq!(Block::from_precomputed(&middle_block, 1), pruned_root_block);
}
