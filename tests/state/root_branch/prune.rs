use std::path::PathBuf;

use mina_indexer::{
    block::{parser::FilesystemParser, Block},
    state::branch::Branch,
};

#[tokio::test]
async fn transition_frontier() {
    //   0
    //  / \
    // 1   6
    // |
    // 2         4
    // |     =>  |
    // 3         5
    // |
    // 4
    // |
    // 5

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = FilesystemParser::new_testing(&log_dir).unwrap();

    // root_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let root_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // main_1_block = mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json
    let main_1_block = block_parser
        .get_precomputed_block("3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk")
        .await
        .unwrap();
    assert_eq!(
        main_1_block.state_hash,
        "3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk".to_owned()
    );

    // fork_block = mainnet-105492-3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN.json
    let fork_block = block_parser
        .get_precomputed_block("3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN")
        .await
        .unwrap();
    assert_eq!(
        fork_block.state_hash,
        "3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN".to_owned()
    );

    // main_2_block = mainnet-105493-3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db.json
    let main_2_block = block_parser
        .get_precomputed_block("3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db")
        .await
        .unwrap();
    assert_eq!(
        main_2_block.state_hash,
        "3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db".to_owned()
    );

    // main_3_block = mainnet-105494-3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww.json
    let main_3_block = block_parser
        .get_precomputed_block("3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww")
        .await
        .unwrap();
    assert_eq!(
        main_3_block.state_hash,
        "3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww".to_owned()
    );

    // main_4_block = mainnet-105495-3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9.json
    let main_4_block = block_parser
        .get_precomputed_block("3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9")
        .await
        .unwrap();
    assert_eq!(
        main_4_block.state_hash,
        "3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9".to_owned()
    );

    // main_5_block = mainnet-105496-3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL.json
    let main_5_block = block_parser
        .get_precomputed_block("3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL")
        .await
        .unwrap();
    assert_eq!(
        main_5_block.state_hash,
        "3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL".to_owned()
    );

    // create the tree and add blocks
    let mut branch = Branch::new(&root_block).unwrap();

    branch.simple_extension(&fork_block).unwrap();
    branch.simple_extension(&main_1_block).unwrap();
    branch.simple_extension(&main_2_block).unwrap();
    branch.simple_extension(&main_3_block).unwrap();
    branch.simple_extension(&main_4_block).unwrap();
    let (best_tip_id, _) = branch.simple_extension(&main_5_block).unwrap();

    println!("=== Before prune ===");
    println!("{branch:?}");

    branch.prune_transition_frontier(
        1,
        &branch.branches.get(&best_tip_id).unwrap().data().clone(),
    );

    println!("=== After prune ===");
    println!("{branch:?}");

    assert_eq!(
        Block::from_precomputed(&main_4_block, 0),
        branch.root_block().clone()
    );
}
