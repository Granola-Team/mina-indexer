use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block, BlockHash},
    state::{branch::Leaf, ledger::diff::LedgerDiff, ExtensionType, IndexerState},
};

/// Extends a branch with a new leaf
#[tokio::test]
async fn extension() {
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

    let mut state = IndexerState::new(BlockHash(root_block.state_hash), None, None).unwrap();

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
        state.add_block(&child1).unwrap(),
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
        state.add_block(&child2).unwrap(),
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
