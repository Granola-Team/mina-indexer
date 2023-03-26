use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, State},
};

/// Adds a new dangling branch and extends it with a new leaf
#[tokio::test]
async fn extension() {
    //      0
    // 0 => |
    //      1

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

    // ----------------
    // initialize state
    // ----------------

    let mut state = State::new(&root_block, None).unwrap();

    // root_block is added as the root of the 0th dangling branch
    assert!(state.root_branch.is_none());
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 1);

    // before extension quantities
    let before_root = state.dangling_branches.get(0).unwrap().root.clone();
    let before_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
    let before_leaf = &before_leaves
        .get(
            state
                .dangling_branches
                .get(0)
                .unwrap()
                .branches
                .root_node_id()
                .unwrap(),
        )
        .unwrap()
        .block;

    // before_root is the only leaf
    assert_eq!(before_leaves.len(), 1);
    assert_eq!(&before_root, before_leaf);
    assert_eq!(before_root, Block::from_precomputed(&root_block, 0));

    // extend the branch with a child of the root
    // child_block = mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let child_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        child_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );

    // before root has no children
    assert!(state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .children_ids(
            state
                .dangling_branches
                .get(0)
                .unwrap()
                .branches
                .root_node_id()
                .unwrap()
        )
        .expect("before branch child")
        .next()
        .is_none());

    println!(
        "Before tree:\n{:?}",
        state.dangling_branches.get(0).unwrap().branches
    );

    // ---------------
    // add child block
    // ---------------
    assert_eq!(
        state.add_block(&child_block),
        ExtensionType::DanglingSimpleForward
    );

    // after extension quantities
    let after_root = &state.dangling_branches.get(0).unwrap().root;
    let branches1 = &state.dangling_branches.get(0).unwrap().branches;
    let leaves1 = &state.dangling_branches.get(0).unwrap().leaves;
    let after_root_id = branches1.root_node_id().unwrap();
    let after_root_leaf = {
        let child_ids: Vec<&NodeId> = branches1.children_ids(&after_root_id).unwrap().collect();
        assert_eq!(child_ids.len(), 1);
        branches1.get(child_ids.get(0).unwrap()).unwrap().data()
    };

    // branch root should still match the root of the dangling branch
    assert_eq!(
        after_root,
        &state
            .dangling_branches
            .get(0)
            .unwrap()
            .branches
            .get(
                state
                    .dangling_branches
                    .get(0)
                    .unwrap()
                    .branches
                    .root_node_id()
                    .unwrap()
            )
            .unwrap()
            .data()
            .block
    );

    println!("After tree:\n{:?}", branches1);

    assert_eq!(
        after_root_leaf.block,
        Block::from_precomputed(&child_block, 1)
    );

    // root shouldn't change
    assert_eq!(&before_root, after_root);

    // after tree has 2 blocks
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 2);

    // after root isn't a leaf
    assert!(!leaves1.contains_key(after_root_id));
}
