use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

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

    // dangling_root_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let dangling_root_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        dangling_root_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    let mut state = IndexerState::new_testing(&root_block, None, None, None, None).unwrap();

    // add dangling_root_block
    let extension = state.add_block(&dangling_root_block).unwrap();
    assert_eq!(extension, ExtensionType::DanglingNew);

    // danlging_root_block is added as the root of the 0th dangling branch
    assert_eq!(state.root_branch.clone().height(), 1);
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().height(), 1);

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
        .unwrap();

    // before_root is the only leaf
    assert_eq!(before_leaves.len(), 1);
    assert_eq!(before_root, before_leaf.block);
    assert_eq!(
        before_root,
        Block::from_precomputed(&dangling_root_block, 0)
    );

    // dangling_child_block = mainnet-105492-3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ.json
    let dangling_child_block = block_parser
        .get_precomputed_block("3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ")
        .await
        .unwrap();
    assert_eq!(
        dangling_child_block.state_hash,
        "3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ".to_owned()
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

    println!("Before state:\n{state:?}");

    // ---------------
    // add child block
    // ---------------
    let extension = state.add_block(&dangling_child_block).unwrap();

    // after extension quantities
    let after_root = &state.dangling_branches.get(0).unwrap().root;
    let branches1 = &state.dangling_branches.get(0).unwrap();
    let leaves1 = &branches1.leaves;
    let after_root_id = branches1.branches.root_node_id().unwrap();
    let after_root_leaf = {
        let child_ids: Vec<&NodeId> = branches1
            .branches
            .children_ids(&after_root_id)
            .unwrap()
            .collect();
        assert_eq!(child_ids.len(), 1);
        branches1
            .branches
            .get(child_ids.get(0).unwrap())
            .unwrap()
            .data()
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

    println!("After state:\n{state:?}");

    assert_eq!(extension, ExtensionType::DanglingSimpleForward);

    assert_eq!(
        Block::from_precomputed(&dangling_child_block, 1),
        after_root_leaf.block
    );

    // root shouldn't change
    assert_eq!(&before_root, after_root);

    // after tree has 2 blocks
    assert_eq!(state.dangling_branches.get(0).unwrap().height(), 2);

    // after root isn't a leaf
    assert!(!leaves1.contains_key(after_root_id));
}
