use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Merges two dangling branches
#[tokio::test]
async fn extension() -> anyhow::Result<()> {
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

    let blocks_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&blocks_dir)?;

    // root_block =
    // mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let (root_block, root_block_bytes) = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await?;
    assert_eq!(
        root_block.state_hash().0,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // middle_block =
    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let (middle_block, _) = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await?;
    assert_eq!(
        middle_block.state_hash().0,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );

    // leaf_block =
    // mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let (leaf_block, _) = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await?;
    assert_eq!(
        leaf_block.state_hash().0,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root0_block will the be the root of the 0th dangling_branch
    let mut state =
        IndexerState::new_testing(&root_block, root_block_bytes, None, None, None, None, None)?;

    // --------
    // add leaf
    // --------

    let (extension_type, _) = state.add_block_to_witness_tree(&leaf_block)?;
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!("=== Before Root Branch ===");
    let mut tree = String::new();
    state
        .root_branch
        .clone()
        .branches
        .write_formatted(&mut tree)?;
    println!("{tree}");

    println!("=== Before Dangling Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .first()
        .unwrap()
        .branches
        .write_formatted(&mut tree)?;
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
        .for_each(|tree| assert_eq!(tree.leaves().len(), 1));

    // ----------------
    // add middle block
    // ----------------

    println!(
        "Block added: {:?}\n",
        Block::from_precomputed(&middle_block, 1)
    );
    println!("=== After Root Branch ===");
    let mut tree = String::new();
    state
        .root_branch
        .clone()
        .branches
        .write_formatted(&mut tree)?;
    println!("{tree}");

    // dangling branch rebases on top of root_branch
    let (extension_type, _) = state.add_block_to_witness_tree(&middle_block)?;
    assert!(matches!(extension_type, ExtensionType::RootComplex(_)));

    // no more dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // Root branch
    // - len = 3
    // - leaf = 1
    // - height = 3
    assert_eq!(state.root_branch.clone().len(), 3);
    assert_eq!(state.root_branch.clone().height(), 3);
    assert_eq!(state.root_branch.clone().leaves().len(), 1);

    // after extension quantities
    let root_branch = state.root_branch;
    let root = root_branch.root_block();
    let branches = root_branch.clone().branches;
    let branch_root = branches
        .get(branches.root_node_id().unwrap())
        .unwrap()
        .data();
    let leaves: Vec<Block> = root_branch.leaves().to_vec();

    assert_eq!(leaves.first().unwrap().state_hash, leaf_block.state_hash());

    // branch root should match the tree's root
    assert_eq!(root, branch_root);

    Ok(())
}
