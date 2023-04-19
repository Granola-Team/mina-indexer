use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, Block, BlockHash},
    state::{ExtensionType, IndexerState},
};

/// Merges two dangling branches, ignore others
#[tokio::test]
async fn extension() {
    // ---------- Dangling branches ----------
    //        Before      |          After
    // --------------- indices ---------------
    //   0      1     2   |     0      1     2
    // ---------------------------------------
    //                    =>          root
    //                    =>           |
    //  root  other  leaf =>  other  middle  .
    //                    =>           |
    //                    =>          leaf

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

    // leaf_block = mainnet-105494-3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy.json
    let leaf_block = block_parser
        .get_precomputed_block("3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy")
        .await
        .unwrap();
    assert_eq!(
        leaf_block.state_hash,
        "3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy".to_owned()
    );

    // other_block = mainnet-105496-3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL.json
    let other_block = block_parser
        .get_precomputed_block("3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL")
        .await
        .unwrap();
    assert_eq!(
        other_block.state_hash,
        "3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL".to_owned()
    );

    // ----------
    // add blocks
    // ----------

    // root in branch 0
    let mut state = IndexerState::new(BlockHash(root_block.state_hash), None, None).unwrap();

    // other in branch 1
    let extension_type = state.add_block(&other_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // leaf in brach 2
    let extension_type = state.add_block(&leaf_block).unwrap();
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

    // merges branch 2 into 0
    let extension_type = state.add_block(&middle_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingComplex);

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

    println!("=== After Branch 1 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(1)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // 2 dangling branches
    // - 0: height = 3
    // - 1: height = 1
    // - each 1 leaf
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .enumerate()
        .for_each(|(index, tree)| match index {
            0 => assert_eq!(tree.branches.height(), 1),
            1 => assert_eq!(tree.branches.height(), 3),
            _ => unreachable!(),
        });
    state.dangling_branches.iter().for_each(|tree| {
        assert_eq!(tree.leaves.len(), 1);
    });

    // after extension quantities
    let root0 = &state.dangling_branches.get(0).unwrap().root;
    let root1 = &state.dangling_branches.get(1).unwrap().root;
    let branches0 = &state.dangling_branches.get(0).unwrap().branches;
    let branches1 = &state.dangling_branches.get(1).unwrap().branches;
    let branch_root0 = &branches0
        .get(&branches0.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let branch_root1 = &branches1
        .get(&branches1.root_node_id().unwrap())
        .unwrap()
        .data()
        .block;
    let leaves0: Vec<Block> = state
        .dangling_branches
        .get(0)
        .unwrap()
        .leaves
        .iter()
        .map(|(_, x)| x.block.clone())
        .collect();
    let leaves1: Vec<Block> = state
        .dangling_branches
        .get(1)
        .unwrap()
        .leaves
        .iter()
        .map(|(_, x)| x.block.clone())
        .collect();
    let leaf = Block::from_precomputed(&leaf_block, 2);
    let other = Block::from_precomputed(&other_block, 0);

    println!("Leaves 0: {:?}", leaves0);
    println!("Leaves 1: {:?}", leaves1);

    assert_eq!(leaves0.get(0).unwrap(), &other);
    assert_eq!(leaves1.get(0).unwrap(), &leaf);

    // branch root should match the tree's root
    assert_eq!(root0, branch_root0);
    assert_eq!(root1, branch_root1);
}
