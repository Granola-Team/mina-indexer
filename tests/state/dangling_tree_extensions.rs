use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{branch::Leaf, ledger::diff::LedgerDiff, ExtensionType, State},
};

/// Adds a new dangling branch and extends it with a new leaf
#[tokio::test]
async fn simple_proper_forward_extension() {
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

/// Extends a branch with a new leaf
#[tokio::test]
async fn simple_improper_forward_extension() {
    // 0      0
    // | =>  / \
    // 1    1   2

    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // ----------------
    // initialize state
    // ----------------

    // root_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let root_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    let mut state = State::new(&root_block, None).unwrap();

    // blocks added to 0th dangling branch
    assert_eq!(state.dangling_branches.len(), 1);

    // only one leaf
    assert_eq!(state.dangling_branches.get(0).unwrap().leaves.len(), 1);

    // -----------
    // add child 1
    // -----------

    // child1 = mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let child1 = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert_eq!(
        state.add_block(&child1),
        ExtensionType::DanglingSimpleForward
    );
    assert_eq!(
        child1.state_hash,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    println!(
        "Before tree:\n{:?}",
        state.dangling_branches.get(0).unwrap().branches
    );

    let before_root = state.dangling_branches.get(0).unwrap().root.clone();

    // blocks added to 0th dangling branch
    assert_eq!(state.dangling_branches.len(), 1);

    // two blocks in the dangling branch
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 2);

    // only one leaf
    assert_eq!(state.dangling_branches.get(0).unwrap().leaves.len(), 1);

    // -----------
    // add child 2
    // -----------

    // child2 = mainnet-105492-3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN.json
    let child2 = block_parser
        .get_precomputed_block("3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN")
        .await
        .unwrap();
    assert_eq!(
        state.add_block(&child2),
        ExtensionType::DanglingSimpleForward
    );
    assert_eq!(
        child2.state_hash,
        "3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN".to_owned()
    );

    // blocks are added to 0th dangling branch
    assert_eq!(state.dangling_branches.len(), 1);

    // three blocks but height = 2
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 2);

    // two leaves
    assert_eq!(state.dangling_branches.get(0).unwrap().leaves.len(), 2);

    // after extension quantities
    let after_root = &state.dangling_branches.get(0).unwrap().root;
    let branches1 = &state.dangling_branches.get(0).unwrap().branches;
    let leaves1 = &state.dangling_branches.get(0).unwrap().leaves;
    let after_root_id = branches1.root_node_id().unwrap();

    // branch root should match the tree's root
    assert_eq!(
        after_root,
        &state
            .dangling_branches
            .get(0)
            .unwrap()
            .branches
            .get(after_root_id)
            .unwrap()
            .data()
            .block
    );

    println!(
        "After tree:\n{:?}",
        state.dangling_branches.get(0).unwrap().branches
    );

    // after root has one child
    let after_children = branches1
        .children_ids(&after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 2);
    println!("After children:\n  {:?}", after_children);

    let after_child1 = after_children.get(0).unwrap();
    let after_child2 = after_children.get(1).unwrap();
    let after_child1_block = Block::from_precomputed(&child1, 1);
    let after_child2_block = Block::from_precomputed(&child2, 1);

    // child1 is a leaf
    assert_eq!(after_child1_block, leaves1.get(after_child1).unwrap().block);

    // child2 is a leaf
    assert_eq!(after_child2_block, leaves1.get(after_child2).unwrap().block);

    println!(
        "After leaves: {:?}",
        leaves1.values().collect::<Vec<&Leaf<LedgerDiff>>>()
    );

    // root shouldn't change
    assert_eq!(&before_root, after_root);

    // after root isn't a leaf
    assert!(!leaves1.contains_key(after_root_id));
}

/// extend a branch backwards with the root's parent
#[tokio::test]
async fn simple_backward_extension() {
    //      1
    // 0 => |
    //      0

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

/// create multiple dangling branches
#[tokio::test]
async fn multiple_dangling_branches() {
    // 0 => 0 1 (two separate branches)

    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // root0_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let root0_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root0_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // root1_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let root1_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        root1_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root0_block will the be the root of the 0th dangling_branch
    let mut state = State::new(&root0_block, None).unwrap();

    // 1 dangling branch
    // - height = 1
    // - 1 leaf
    assert_eq!(state.dangling_branches.len(), 1);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves.len(), 1));

    // before extension quantities
    let root0 = state.dangling_branches.get(0).unwrap().root.clone();
    let root_leaf0 = state
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

    // root == leaf
    assert_eq!(root0, root_leaf0);
    println!(
        "=== Dangling Branch 0 ===\n{:?}",
        state.dangling_branches.get(0).unwrap()
    );

    // ---------
    // add block
    // ---------

    // make a new dangling branch
    let extension_type = state.add_block(&root1_block);
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // 2 dangling branches
    // - each height = 1
    // - each has 1 leaf
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves.len(), 1));

    // after extension quantities
    let root1 = &state.dangling_branches.get(1).unwrap().root;
    let branches1 = &state.dangling_branches.get(1).unwrap().branches;
    let branch_root1 = &branches1
        .get(&branches1.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let root_leaf1 = &state
        .dangling_branches
        .get(1)
        .unwrap()
        .leaves
        .get(branches1.root_node_id().unwrap())
        .unwrap()
        .block
        .clone();

    // root == leaf
    assert_eq!(root1, root_leaf1);
    println!(
        "\n=== Dangling Branch 1 ===\n{:?}",
        &state.dangling_branches.get(1).unwrap()
    );

    // branch root should match the tree's root
    assert_eq!(root1, branch_root1);
    assert!(false);
}

// TODO more complex test scenarios
