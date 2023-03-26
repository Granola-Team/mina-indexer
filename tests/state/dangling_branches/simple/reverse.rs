use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, State},
};

/// Extends a branch backwards with the root's parent
#[tokio::test]
async fn extension() {
    // ----- Dangling Branches -----
    //   Before  |    After
    // -----------------------------
    //           =>     1
    //    0      =>     |
    //           =>     0

    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // *** parent ***
    // new_root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let new_root_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        new_root_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // *** child ***
    // new_root_block = mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let old_root_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        old_root_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // child becomes the root of the 0th dangling branch
    let mut state = State::new(&old_root_block, None).unwrap();

    // before extension quantities
    let before_root = state.dangling_branches.get(0).unwrap().root.clone();
    let before_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
    let before_root_leaf = state
        .dangling_branches
        .get(0)
        .unwrap()
        .leaves
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
        .block
        .clone();

    assert_eq!(before_root, before_root_leaf);
    assert_eq!(before_leaves.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 1);
    println!(
        "=== Before tree ===\n{:?}",
        state.dangling_branches.get(0).unwrap()
    );

    // extend the branch with new_root_block (the parent)
    let extension_type = state.add_block(&new_root_block);
    assert_eq!(extension_type, ExtensionType::DanglingSimpleReverse);

    // after extension quantities
    let after_root = &state.dangling_branches.get(0).unwrap().root;
    let branches1 = &state.dangling_branches.get(0).unwrap().branches;
    let leaves1 = &state.dangling_branches.get(0).unwrap().leaves;
    let after_root_id = branches1.root_node_id().unwrap();
    let after_root_block = branches1.get(&after_root_id).unwrap().data().block.clone();

    assert_eq!(leaves1.len(), 1);
    assert_eq!(branches1.height(), 2);
    println!(
        "=== After tree ===\n{:?}",
        &state.dangling_branches.get(0).unwrap()
    );

    // after root has one child
    let after_children = branches1
        .children_ids(&after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 1);

    let after_child = after_children.get(0).unwrap();
    let after_child_block = Block::from_precomputed(&old_root_block, 1);
    assert_eq!(
        after_child_block,
        leaves1
            .get(after_child)
            .expect("There should be a leaf block")
            .block
    );

    // root checks
    assert_ne!(&before_root, after_root);
    assert_eq!(after_root, &after_root_block);

    // leaf checks
    let leaves: Vec<&Block> = leaves1.iter().map(|(_, x)| &x.block).collect();
    let leaf = leaves.get(0).unwrap();

    // height differs, hashes agree
    assert_eq!(leaf.height, 1 + before_root.height);
    assert_eq!(leaf.state_hash, before_root.state_hash);
    assert_eq!(leaf.parent_hash, before_root.parent_hash);
}
