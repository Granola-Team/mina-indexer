use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Extends a branch backwards with the root's parent
#[tokio::test]
async fn extension() {
    // --- Dangling branch 0 ---
    //   Before   |    After
    // -----------+-------------
    //            =>   new
    //     old    =>    |
    //            =>   old

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // root_block =
    // mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let (root_block, root_block_bytes) = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash().0,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // *** parent ***
    // new_dangling_root_block =
    // mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let (new_dangling_root_block, _) = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        new_dangling_root_block.state_hash().0,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // *** child ***
    // new_dangling_root_block =
    // mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let (old_dangling_root_block, _) = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert_eq!(
        old_dangling_root_block.state_hash().0,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root_block is the root of the root branch
    let mut state =
        IndexerState::new_testing(&root_block, root_block_bytes, None, None, None, None, None)
            .unwrap();

    // old_dangling_root_block is originally the root of the 0th dangling branch
    let (extension_type, _) = state
        .add_block_to_witness_tree(&old_dangling_root_block, true)
        .unwrap();

    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // before extension quantities
    let before_root = state
        .dangling_branches
        .first()
        .unwrap()
        .root_block()
        .clone();
    let before_leaves = state.dangling_branches.first().unwrap().leaves();
    let before_root_leaf = state
        .dangling_branches
        .first()
        .unwrap()
        .leaves()
        .first()
        .unwrap()
        .clone();

    assert_eq!(before_leaves.len(), 1);
    assert_eq!(state.dangling_branches.first().unwrap().len(), 1);
    assert_eq!(state.dangling_branches.first().unwrap().height(), 1);
    assert_eq!(before_root, before_root_leaf);

    println!("=== Before state ===");
    println!("{state}");

    // extend the branch with new_dangling_root_block (the parent)
    let (extension_type, _) = state
        .add_block_to_witness_tree(&new_dangling_root_block, true)
        .unwrap();

    println!("=== After state ===");
    println!("{}", &state);

    assert_eq!(extension_type, ExtensionType::DanglingSimpleReverse);

    // after extension quantities
    let after_branch = state.dangling_branches.first().unwrap();
    let after_root = after_branch.root_block().clone();
    let branches1 = &after_branch.branches;
    let leaves1 = &after_branch.leaves();
    let after_root_id = branches1.root_node_id().unwrap();
    let after_root_block = branches1.get(after_root_id).unwrap().data().clone();

    assert_eq!(leaves1.len(), 1);
    assert_eq!(branches1.height(), 2);
    println!("=== After state ===");
    println!("{}", &state);

    // after root has one child
    let after_children = branches1
        .children_ids(after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 1);

    let after_child_block = Block::from_precomputed(&old_dangling_root_block, 1);
    assert_eq!(
        &after_child_block,
        after_branch
            .leaves()
            .first()
            .expect("There should be a leaf block")
    );

    // root checks
    assert_ne!(before_root, after_root);
    assert_eq!(after_root, after_root_block);

    // leaf checks
    let leaves: Vec<Block> = leaves1.to_vec();
    let leaf = leaves.first().unwrap();

    // height differs, hashes agree
    assert_eq!(leaf.height, 1 + before_root.height);
    assert_eq!(leaf.state_hash, before_root.state_hash);
    assert_eq!(leaf.parent_hash, before_root.parent_hash);
}
