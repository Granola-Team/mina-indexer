use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, State},
};

/// Extends multiple dangling branches
#[tokio::test]
async fn test() {
    // ----- Dangling branches -----
    //     Before    |         After
    // ----------- indices ---------------------
    //   0      1    |    0            1
    // -----------------------------------------
    //               => root0        root1
    // root0 child10 =>   |          /   \
    //               => child0  child10  child11

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

    // child0_block = mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let child0_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        child0_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );

    // root1_block = mainnet-105493-3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db.json
    let root1_block = block_parser
        .get_precomputed_block("3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db")
        .await
        .unwrap();
    assert_eq!(
        root1_block.state_hash,
        "3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db".to_owned()
    );

    // child10_block = mainnet-105494-3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy.json
    let child10_block = block_parser
        .get_precomputed_block("3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy")
        .await
        .expect("WTF");
    assert_eq!(
        child10_block.state_hash,
        "3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy".to_owned()
    );

    // child11_block = mainnet-105494-3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww.json
    let child11_block = block_parser
        .get_precomputed_block("3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww")
        .await
        .unwrap();
    assert_eq!(
        child11_block.state_hash,
        "3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root0_block will the be the root of the 0th dangling_branch
    let mut state = State::new(&root0_block, None).unwrap();

    // ------------
    // add child 10
    // ------------

    let extension_type = state.add_block(&child10_block);
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!(
        "=== Before Branch 0 ===\n{:?}",
        state.dangling_branches.get(0).unwrap()
    );
    println!(
        "=== Before Branch 1 ===\n{:?}",
        state.dangling_branches.get(1).unwrap()
    );

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

    // -----------
    // add child 0
    // -----------

    let extension_type = state.add_block(&child0_block);
    assert_eq!(extension_type, ExtensionType::DanglingSimpleForward);

    // ----------
    // add root 1
    // ----------

    let extension_type = state.add_block(&root1_block);
    assert_eq!(extension_type, ExtensionType::DanglingSimpleReverse);

    // ------------
    // add child 11
    // ------------

    let extension_type = state.add_block(&child11_block);
    assert_eq!(extension_type, ExtensionType::DanglingSimpleForward);

    // 2 dangling branches
    // - each height = 2
    // - 0: 1 leaf
    // - 1: 2 leaves
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 2));
    state
        .dangling_branches
        .iter()
        .enumerate()
        .for_each(|(idx, tree)| {
            if idx == 0 {
                assert_eq!(tree.leaves.len(), 1)
            } else if idx == 1 {
                assert_eq!(tree.leaves.len(), 2)
            }
        });

    // after extension quantities
    let root1 = &state.dangling_branches.get(1).unwrap().root;
    let branches1 = &state.dangling_branches.get(1).unwrap().branches;
    let branch_root1 = &branches1
        .get(&branches1.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let leaves1: Vec<&Block> = state
        .dangling_branches
        .get(1)
        .unwrap()
        .leaves
        .iter()
        .map(|(_, x)| &x.block)
        .collect();

    // root1 is not a leaf
    assert_ne!(&root1, leaves1.get(0).unwrap());
    println!(
        "\n=== After Branch 0 ===\n{:?}",
        &state.dangling_branches.get(0).unwrap()
    );
    println!(
        "\n=== After Branch 1 ===\n{:?}",
        &state.dangling_branches.get(1).unwrap()
    );

    // branch root should match the tree's root
    assert_eq!(root1, branch_root1);
}
