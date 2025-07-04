use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Extends a branch with a new leaf
#[tokio::test]
async fn extension() {
    // --------------------------------
    //           Root branch
    // --------------------------------
    //      =>  root  =>     root
    // root =>   |    =>    /    \
    //      => child0 => child0 child1
    // --------------------------------

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // ----------------
    // initialize state
    // ----------------

    // root_block =
    // mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let (root_block, root_block_bytes) = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash().0,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    let mut state =
        IndexerState::new_testing(&root_block, root_block_bytes, None, None, None, false).unwrap();

    // root branch
    // - len = 1
    // - height = 1
    // - leaves = 1
    assert_eq!(state.root_branch.len(), 1);
    assert_eq!(state.root_branch.height(), 1);
    assert_eq!(state.root_branch.leaves().len(), 1);

    // no dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // -----------
    // add child 1
    // -----------

    // child1 = mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.
    // json
    let (child1, _) = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert!(matches!(
        state
            .add_block_to_witness_tree(&child1, true, true)
            .unwrap()
            .0,
        ExtensionType::RootSimple(_)
    ));
    assert_eq!(
        child1.state_hash().0,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    println!("=== Before Root Branch ===");
    println!("{:?}", state.root_branch.branches);

    let before_root = state.root_branch.root_block().clone();

    // root branch
    // - len = 2
    // - height = 2
    // - leaves = 1
    assert_eq!(state.root_branch.len(), 2);
    assert_eq!(state.root_branch.height(), 2);
    assert_eq!(state.root_branch.leaves().len(), 1);

    // no dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // -----------
    // add child 2
    // -----------

    // child2 = mainnet-105492-3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN.
    // json
    let (child2, _) = block_parser
        .get_precomputed_block("3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN")
        .await
        .unwrap();
    let (extension_type, _) = state
        .add_block_to_witness_tree(&child2, true, true)
        .unwrap();

    assert!(matches!(extension_type, ExtensionType::RootSimple(_)));
    assert_eq!(
        child2.state_hash().0,
        "3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN".to_owned()
    );

    // root branch
    // - len = 3
    // - height = 2
    // - leaves = 2
    assert_eq!(state.root_branch.len(), 3);
    assert_eq!(state.root_branch.height(), 2);
    assert_eq!(state.root_branch.leaves().len(), 2);

    // no dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // after extension quantities
    let after_root = state.root_branch.root_block().clone();
    let branches1 = &state.root_branch.branches;
    let leaves1 = state.root_branch.leaves();
    let after_root_id = branches1.root_node_id().unwrap();

    // branch root should match the tree's root
    assert_eq!(
        &after_root,
        state
            .root_branch
            .branches
            .get(after_root_id)
            .unwrap()
            .data()
    );

    println!("=== After Root Branch ===");
    println!("{:?}", state.root_branch.branches);

    // after root has one child
    let after_children = branches1
        .children_ids(after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 2);
    println!("After children:\n  {:?}", after_children);

    let after_child1_block = Block::from_precomputed(&child1, 1);
    let after_child2_block = Block::from_precomputed(&child2, 1);

    // child1 is a leaf
    assert!(leaves1.to_vec().contains(&after_child1_block));

    // child2 is a leaf
    assert!(leaves1.to_vec().contains(&after_child2_block));

    println!("=== After Root Branch Leaves ===");
    println!("{:?}", leaves1.to_vec());

    // root doesn't change
    assert_eq!(before_root, after_root);

    // after root isn't a leaf
    assert!(!leaves1.to_vec().contains(&after_root));
}
