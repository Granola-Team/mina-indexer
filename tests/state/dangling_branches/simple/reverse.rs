use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block, BlockHash},
    state::{ledger::genesis::GenesisLedger, ExtensionType, IndexerState},
};

/// Extends a branch backwards with the root's parent
#[tokio::test]
async fn extension() {
    // --- Dangling branch 0 ---
    //   Before   |    After
    // -----------+-------------
    //            =>   new
    //     old    =>    |
    //            =>   old

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

    // *** parent ***
    // new_dangling_root_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let new_dangling_root_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        new_dangling_root_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // *** child ***
    // new_dangling_root_block = mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let old_dangling_root_block = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert_eq!(
        old_dangling_root_block.state_hash,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root_block is the root of the root branch
    let mut state = IndexerState::new(
        BlockHash(root_block.state_hash.clone()),
        GenesisLedger {
            name: "testing".to_string(),
            accounts: Vec::new(),
        },
        None,
    )
    .unwrap();

    // old_dangling_root_block is originally the root of the 0th dangling branch
    let extension_type = state.add_block(&old_dangling_root_block).unwrap();

    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // before extension quantities
    let before_branch = state.dangling_branches.get(0).unwrap();
    let before_root = before_branch.root.clone();
    let before_leaves = before_branch.leaves.clone();
    let before_root_leaf = before_branch
        .leaves
        .get(before_branch.branches.root_node_id().unwrap())
        .unwrap()
        .clone();

    assert_eq!(before_leaves.len(), 1);
    assert_eq!(before_branch.len(), 1);
    assert_eq!(before_branch.height(), 1);
    assert_eq!(before_root, before_root_leaf.block);

    println!("=== Before state ===");
    println!("{:?}", &state);

    // extend the branch with new_dangling_root_block (the parent)
    let extension_type = state.add_block(&new_dangling_root_block).unwrap();

    println!("=== After state ===");
    println!("{:?}", &state);

    assert_eq!(extension_type, ExtensionType::DanglingSimpleReverse);

    // after extension quantities
    let after_branch = state.dangling_branches.get(0).unwrap();
    let after_root = &after_branch.root;
    let branches1 = &after_branch.branches;
    let leaves1 = &after_branch.leaves;
    let after_root_id = branches1.root_node_id().unwrap();
    let after_root_block = branches1.get(&after_root_id).unwrap().data().clone();

    assert_eq!(leaves1.len(), 1);
    assert_eq!(branches1.height(), 2);
    println!("=== After state ===");
    println!("{:?}", &state);

    // after root has one child
    let after_children = branches1
        .children_ids(&after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 1);

    let after_child = after_children.get(0).unwrap();
    let after_child_block = Block::from_precomputed(&old_dangling_root_block, 1);
    assert_eq!(
        after_child_block,
        leaves1
            .get(after_child)
            .expect("There should be a leaf block")
            .block
    );

    // root checks
    assert_ne!(&before_root, after_root);
    assert_eq!(after_root, &after_root_block.block);

    // leaf checks
    let leaves: Vec<&Block> = leaves1.iter().map(|(_, x)| &x.block).collect();
    let leaf = leaves.get(0).unwrap();

    // height differs, hashes agree
    assert_eq!(leaf.height, 1 + before_root.height);
    assert_eq!(leaf.state_hash, before_root.state_hash);
    assert_eq!(leaf.parent_hash, before_root.parent_hash);
}
