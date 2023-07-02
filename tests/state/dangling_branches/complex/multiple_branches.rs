use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::{collections::HashSet, path::PathBuf};

/// Merges two dangling branches onto the root branch
#[tokio::test]
async fn merge() {
    // --------------------------
    //       Root branch
    // ----------+---------------
    //   Before  |      After
    // ----------+---------------
    //           =>     root
    //           =>       |
    //    root   =>     middle
    //           =>     /   \
    //           =>  leaf0 leaf1
    // --------------------------

    // --------------------------
    //       Dangling branches
    // --------------+-----------
    //     Before    |   After
    // ---------- indices -------
    //    0      1   |     .
    // --------------+-----------
    //  leaf0  leaf1 =>    .
    // --------------------------

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // root_block = mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let root_block = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    // middle_block = mainnet-105493-3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db.json
    let middle_block = block_parser
        .get_precomputed_block("3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db")
        .await
        .unwrap();
    assert_eq!(
        middle_block.state_hash,
        "3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db".to_owned()
    );

    // leaf0_block = mainnet-105494-3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy.json
    let leaf0_block = block_parser
        .get_precomputed_block("3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy")
        .await
        .unwrap();
    assert_eq!(
        leaf0_block.state_hash,
        "3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy".to_owned()
    );

    // leaf1_block = mainnet-105494-3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww.json
    let leaf1_block = block_parser
        .get_precomputed_block("3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww")
        .await
        .unwrap();
    assert_eq!(
        leaf1_block.state_hash,
        "3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    let mut state = IndexerState::new_testing(&root_block, None, None, None).unwrap();

    // ---------
    // add leaf0
    // ---------

    let extension_type = state.add_block(&leaf0_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // ---------
    // add leaf1
    // ---------

    let extension_type = state.add_block(&leaf1_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // Root branch
    // - len = 1
    // - height = 1
    // - leaves = 1
    assert_eq!(state.root_branch.clone().len(), 1);
    assert_eq!(state.root_branch.clone().height(), 1);
    assert_eq!(state.root_branch.clone().leaves().len(), 1);

    // 2 dangling branches
    // - len = 1
    // - height = 1
    // - leaves = 1
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.len(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves().len(), 1));

    println!("=== Before state ===");
    println!("{state:?}");

    // ----------------
    // add middle block
    // ----------------

    let extension_type = state.add_block(&middle_block).unwrap();
    assert_eq!(extension_type, ExtensionType::RootComplex);

    println!("=== After state ===");
    println!("{state:?}");

    // Root branch
    // - len = 4
    // - height = 3
    // - leaves = 2
    assert_eq!(state.root_branch.clone().len(), 4);
    assert_eq!(state.root_branch.clone().height(), 3);
    assert_eq!(state.root_branch.clone().leaves().len(), 2);

    // no dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // after extension quantities
    let root_branch = state.root_branch.clone();
    let root0 = state.root_branch.root_block();
    let branches0 = state.root_branch.clone().branches;
    let branch_root0 = branches0
        .get(&branches0.root_node_id().unwrap())
        .unwrap()
        .data();
    let leaf0 = Block::from_precomputed(&leaf0_block, 2);
    let leaf1 = Block::from_precomputed(&leaf1_block, 2);
    let leaves0: HashSet<Block> = root_branch.leaves().iter().map(|x| x.clone()).collect();

    assert_eq!(
        leaves0
            .iter()
            .map(|x| x.clone())
            .collect::<HashSet<Block>>(),
        HashSet::from([leaf0, leaf1])
    );

    // branch root should match the tree's root
    assert_eq!(root0, branch_root0);
}
