use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Merges two dangling branches, ignore others
#[tokio::test]
async fn extension() -> anyhow::Result<()> {
    // ---------------- Branches ------------------
    //        Before       |         After
    // ------+-------------+-----------+-----------
    //  Root |   Dangling  |    Root   |  Dangling
    // ------+-------------+-----------+-----------
    //       |   0     1   |           |   0    1
    // ------+-------------+-----------+-----------
    //       |             =>   root   |
    //       |             =>    |     |
    //  root | other  leaf =>  middle  | other  .
    //       |             =>    |     |
    //       |             =>   leaf   |

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir)?;

    // root_block =
    // mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let (root_block, root_block_bytes) = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await?;
    assert_eq!(
        root_block.state_hash().0,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    // middle_block =
    // mainnet-105493-3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db.json
    let (middle_block, _) = block_parser
        .get_precomputed_block("3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db")
        .await?;
    assert_eq!(
        middle_block.state_hash().0,
        "3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db".to_owned()
    );

    // leaf_block =
    // mainnet-105494-3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy.json
    let (leaf_block, _) = block_parser
        .get_precomputed_block("3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy")
        .await?;
    assert_eq!(
        leaf_block.state_hash().0,
        "3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy".to_owned()
    );

    // other_block =
    // mainnet-105496-3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL.json
    let (other_block, _) = block_parser
        .get_precomputed_block("3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL")
        .await?;
    assert_eq!(
        other_block.state_hash().0,
        "3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL".to_owned()
    );

    // ----------
    // add blocks
    // ----------

    // root in branch branch
    let mut state =
        IndexerState::new_testing(&root_block, root_block_bytes, None, None, None, None, None)?;

    // other in dangling branch 0
    let (extension_type, _) = state.add_block_to_witness_tree(&other_block)?;
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // leaf in dangling branch 1
    let (extension_type, _) = state.add_block_to_witness_tree(&leaf_block)?;
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // 2 dangling branches
    // - each height = 1
    // - each 1 leaf
    assert_eq!(state.dangling_branches.len(), 2);
    state.dangling_branches.iter().for_each(|tree| {
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.leaves().len(), 1);
    });

    for (idx, branch) in state.dangling_branches.iter().enumerate() {
        println!("=== Before Branch {idx} ===");
        let mut tree = String::new();
        branch.branches.write_formatted(&mut tree)?;
        println!("{tree}");
    }

    // ----------------
    // add middle block
    // ----------------

    // merges branch 2 into 0
    let (extension_type, _) = state.add_block_to_witness_tree(&middle_block)?;
    assert!(matches!(extension_type, ExtensionType::RootComplex(_)));

    for (idx, branch) in state.dangling_branches.iter().enumerate() {
        println!("=== After Branch {idx} ===");
        let mut tree = String::new();
        branch.branches.write_formatted(&mut tree)?;
        println!("{tree}");
    }

    // root branch
    assert_eq!(state.root_branch.clone().len(), 3);
    assert_eq!(state.root_branch.clone().leaves().len(), 1);

    // 1 dangling branch
    // - height = 1
    // - 1 leaf
    assert_eq!(state.dangling_branches.len(), 1);
    state
        .dangling_branches
        .iter()
        .enumerate()
        .for_each(|(_, tree)| assert_eq!(tree.height(), 1));
    state.dangling_branches.iter().for_each(|tree| {
        assert_eq!(tree.leaves().len(), 1);
    });

    // after extension quantities
    let root = state.root_branch.root_block();
    let branches = state.root_branch.clone().branches;
    let branch_root = branches
        .get(branches.root_node_id().unwrap())
        .unwrap()
        .data();
    let leaves: Vec<Block> = state.root_branch.leaves().to_vec();
    let leaf = Block::from_precomputed(&leaf_block, 2);

    println!("Root Leaves: {:?}", leaves);
    assert_eq!(leaves.first().unwrap(), &leaf);

    // branch root should match the tree's root
    assert_eq!(root, branch_root);

    Ok(())
}
