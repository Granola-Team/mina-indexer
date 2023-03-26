use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, State},
};

/// Extends multiple dangling branches
#[tokio::test]
async fn test() {
    // ----- Dangling branches -----
    //    Before    |      After
    // ---------- indices ------------
    //   0     1    |    0       1
    // -------------------------------
    //              => root
    //              =>   |
    //  root  leaf  => middle    .
    //              =>   |
    //              => leaf

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

    // middle_block = mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let middle_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        middle_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );

    // leaf_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let leaf_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        leaf_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root0_block will the be the root of the 0th dangling_branch
    let mut state = State::new(&root_block, None).unwrap();

    // ------------
    // add child 10
    // ------------

    let extension_type = state.add_block(&leaf_block);
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!("=== Before Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree:?}");

    println!("=== Before Branch 1 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(1)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree:?}");

    // 2 dangling branches
    // - each height = 1
    // - each 1 leaf
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves.len(), 1));

    // ----------------
    // add middle block
    // ----------------

    let extension_type = state.add_block(&middle_block);
    assert_eq!(extension_type, ExtensionType::DanglingComplex);

    // 1 dangling branch
    // - height = 3
    // - 1 leaf
    assert_eq!(state.dangling_branches.len(), 1);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 3));
    state
        .dangling_branches
        .iter()
        .enumerate()
        .for_each(|(_, tree)| {
            assert_eq!(tree.leaves.len(), 1);
        });

    // after extension quantities
    let root0 = &state.dangling_branches.get(0).unwrap().root;
    let branches0 = &state.dangling_branches.get(0).unwrap().branches;
    let branch_root0 = &branches0
        .get(&branches0.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let leaves0: Vec<&Block> = state
        .dangling_branches
        .get(0)
        .unwrap()
        .leaves
        .iter()
        .map(|(_, x)| &x.block)
        .collect();

    assert_eq!(leaves0.get(0).unwrap().state_hash.0, leaf_block.state_hash);

    println!("=== Before Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree:?}");

    // branch root should match the tree's root
    assert_eq!(root0, branch_root0);
}
