use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, Block, BlockHash},
    state::{ledger::genesis::GenesisLedger, ExtensionType, IndexerState},
};

/// Merges two dangling branches
#[tokio::test]
async fn extension() {
    // -----------------------------
    //          Root branch
    // -----------------------------
    //    Before    |    After
    // -----------------------------
    //              =>   root
    //              =>     |
    //     root     =>   middle
    //              =>     |
    //              =>    leaf
    // -----------------------------
    //       Dangling branches
    // -----------------------------
    //    Before    |    After
    // --------- indices -----------
    //       0      |      0
    // -----------------------------
    //      leaf    =>     .
    // -----------------------------

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
    let mut state = IndexerState::new(
        BlockHash(root_block.state_hash),
        GenesisLedger {
            name: "testing".to_string(),
            accounts: Vec::new(),
        },
        None,
    )
    .unwrap();

    // --------
    // add leaf
    // --------

    let extension_type = state.add_block(&leaf_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!("=== Before Root Branch ===");
    let mut tree = String::new();
    state
        .root_branch
        .clone()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    println!("=== Before Dangling Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // 1 dangling branch
    // - each height = 1
    // - each 1 leaf
    assert_eq!(state.dangling_branches.len(), 1);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves.len(), 1));

    // ----------------
    // add middle block
    // ----------------

    // dangling branch rebases on top of root_branch
    let extension_type = state.add_block(&middle_block).unwrap();
    assert_eq!(extension_type, ExtensionType::RootComplex);

    println!("=== After Root Branch ===");
    let mut tree = String::new();
    state
        .root_branch
        .clone()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // no more dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // Root branch
    // - len = 3
    // - leaf = 1
    // - height = 3
    assert_eq!(state.root_branch.clone().len(), 3);
    assert_eq!(state.root_branch.clone().height(), 3);
    assert_eq!(state.root_branch.clone().leaves.len(), 1);

    // after extension quantities
    let root = &state.root_branch.clone().root;
    let branches = &state.root_branch.clone().branches;
    let branch_root = &branches
        .get(&branches.root_node_id().unwrap())
        .unwrap()
        .data();
    let root_branch = state.root_branch;
    let leaves: Vec<&Block> = root_branch.leaves.iter().map(|(_, x)| &x.block).collect();

    assert_eq!(leaves.get(0).unwrap().state_hash.0, leaf_block.state_hash);

    // branch root should match the tree's root
    assert_eq!(root, &branch_root.block);
}
