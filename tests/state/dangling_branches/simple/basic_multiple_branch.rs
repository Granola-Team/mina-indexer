use mina_indexer::{
    block::parser::BlockParser,
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Creates multiple dangling branches
#[tokio::test]
async fn extensions() {
    // -----------------------
    //       Root branch
    // -----------------------
    //   Before   |   After
    // -----------+-----------
    //      0     =>    0
    // -----------------------

    // -----------------------
    //    Dangling branches
    // ------- indicies ------
    //      .     |     0
    // -----------+-----------
    //      .     =>    1
    // -----------------------

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
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
    let mut state = IndexerState::new_testing(&root0_block, None, None, None).unwrap();

    // Root branch
    // - len = 1
    // - leaf = 1
    // - height = 1
    assert_eq!(state.root_branch.clone().len(), 1);
    assert_eq!(state.root_branch.clone().height(), 1);
    assert_eq!(state.root_branch.clone().leaves.len(), 1);

    // no dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // before extension quantities
    let root0 = state.root_branch.clone().root;
    let root_leaf0 = state
        .root_branch
        .leaves
        .get(state.root_branch.branches.root_node_id().unwrap())
        .unwrap()
        .clone();

    println!("=== Before Branch 0 ===");
    println!("{:?}", state.root_branch.branches);

    // root == leaf
    assert_eq!(root0, root_leaf0.block);

    // ---------
    // add block
    // ---------

    // make a new dangling branch
    let extension_type = state.add_block(&root1_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // Root branch
    // - len = 1
    // - height = 1
    // - leaves = 1
    assert_eq!(state.root_branch.len(), 1);
    assert_eq!(state.root_branch.height(), 1);
    assert_eq!(state.root_branch.leaves.len(), 1);

    // 1 dangling branch
    // - len = 1
    // - height = 1
    // - leaves = 1
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().height(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().leaves.len(), 1);

    // after extension quantities
    let root1 = &state.root_branch.root;
    let branches1 = &state.root_branch.branches;
    let branch_root1 = &branches1
        .get(&branches1.root_node_id().unwrap())
        .unwrap()
        .data();
    let root_leaf1 = state
        .root_branch
        .leaves
        .get(branches1.root_node_id().unwrap())
        .unwrap();

    // root == leaf
    assert_eq!(root1, &root_leaf1.block);

    println!("\n=== After Root Branch ===");
    println!("{:?}", state.root_branch);
    println!("\n=== After Dangling Branch 0 ===");
    println!("{:?}", state.dangling_branches.get(0).unwrap());

    // branch root should match the tree's root
    assert_eq!(root1, &branch_root1.block);
}
