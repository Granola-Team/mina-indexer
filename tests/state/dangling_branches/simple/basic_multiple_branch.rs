use std::path::PathBuf;

use mina_indexer::{
    block::parser::BlockParser,
    state::{ExtensionType, IndexerState},
};

/// Creates multiple dangling branches
#[tokio::test]
async fn extensions() {
    // ----- Dangling branches -----
    //    Before   |    After
    // -----------------------------
    //      0      =>   0   1  (two separate branches)

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
    let mut state = IndexerState::new(&root0_block, None).unwrap();

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
    println!("=== Before Branch 0 ===");
    println!("{:?}", state.dangling_branches.get(0).unwrap().branches);

    // ---------
    // add block
    // ---------

    // make a new dangling branch
    let extension_type = state.add_block(&root1_block).unwrap();
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

    println!("\n=== After Branch 0 ===");
    println!("{:?}", &state.dangling_branches.get(0).unwrap());
    println!("\n=== After Branch 1 ===");
    println!("{:?}", &state.dangling_branches.get(1).unwrap());

    // branch root should match the tree's root
    assert_eq!(root1, branch_root1);
}
