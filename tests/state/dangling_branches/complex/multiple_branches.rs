use std::{collections::HashSet, path::PathBuf};

use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, State},
};

/// Merges three dangling branches
#[tokio::test]
async fn extension() {
    // ---------- Dangling branches -----------
    //       Before      |          After
    // -------------- indices -----------------
    //   0     1     2   |       0        1  2
    // ----------------------------------------
    //                   =>    root
    //                   =>      |
    //  root leaf0 leaf1 =>    middle     .  .
    //                   =>    /   \
    //                   => leaf0 leaf1

    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

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

    let mut state = State::new(&root_block, None).unwrap();

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

    // 3 dangling branches
    // - each height = 1
    // - each 1 leaf
    assert_eq!(state.dangling_branches.len(), 3);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves.len(), 1));

    println!("=== Before Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    println!("=== Before Branch 1 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(1)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    println!("=== Before Branch 2 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(2)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // ----------------
    // add middle block
    // ----------------

    let extension_type = state.add_block(&middle_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingComplex);

    // 1 dangling branch
    // - height = 3
    // - 2 leaves
    assert_eq!(state.dangling_branches.len(), 1);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.branches.height(), 3));
    state.dangling_branches.iter().for_each(|tree| {
        assert_eq!(tree.leaves.len(), 2);
    });

    // after extension quantities
    let root0 = &state.dangling_branches.get(0).unwrap().root;
    let branches0 = &state.dangling_branches.get(0).unwrap().branches;
    let branch_root0 = &branches0
        .get(&branches0.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let leaves0: HashSet<&Block> = state
        .dangling_branches
        .get(0)
        .unwrap()
        .leaves
        .iter()
        .map(|(_, x)| &x.block)
        .collect();
    let leaf0 = Block::from_precomputed(&leaf0_block, 2);
    let leaf1 = Block::from_precomputed(&leaf1_block, 2);

    assert_eq!(
        leaves0.iter().collect::<HashSet<&&Block>>(),
        HashSet::from([&&leaf0, &&leaf1])
    );

    println!("=== After Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // branch root should match the tree's root
    assert_eq!(root0, branch_root0);
}
