use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;

/// Extends multiple dangling branches
#[tokio::test]
async fn extensions() {
    // ----- Dangling branches -----------------
    //     Before    |         After
    // ----------- indices ---------------------
    //   0      1    |    0            1
    // -----------------------------------------
    //               => root0        root1
    // root0 child10 =>   |          /   \
    //               => child0  child10  child11

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let root_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // root0_block = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let root0_block = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        root0_block.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // child0_block = mainnet-105492-3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ.json
    let child0_block = block_parser
        .get_precomputed_block("3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ")
        .await
        .unwrap();
    assert_eq!(
        child0_block.state_hash,
        "3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ".to_owned()
    );

    // root1_block = mainnet-105495-3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9.json
    let root1_block = block_parser
        .get_precomputed_block("3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9")
        .await
        .unwrap();
    assert_eq!(
        root1_block.state_hash,
        "3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9".to_owned()
    );

    // child10_block = mainnet-105496-3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL.json
    let child10_block = block_parser
        .get_precomputed_block("3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL")
        .await
        .expect("WTF");
    assert_eq!(
        child10_block.state_hash,
        "3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL".to_owned()
    );

    // child11_block = mainnet-105496-3NKE1aiFviFWrYMN5feKm3L7C4Zqp3czkwAtcXj1tdbaGDZ47L1k.json
    let child11_block = block_parser
        .get_precomputed_block("3NKE1aiFviFWrYMN5feKm3L7C4Zqp3czkwAtcXj1tdbaGDZ47L1k")
        .await
        .unwrap();
    assert_eq!(
        child11_block.state_hash,
        "3NKE1aiFviFWrYMN5feKm3L7C4Zqp3czkwAtcXj1tdbaGDZ47L1k".to_owned()
    );

    // ----------------
    // initialize state
    // ----------------

    // root0_block will the be the root of the 0th dangling_branch
    let mut state = IndexerState::new_testing(&root_block, None, None, None).unwrap();

    // ----------
    // add root 0
    // ----------

    let (extension_type, _) = state.add_block_to_witness_tree(&root0_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    // ------------
    // add child 10
    // ------------

    let (extension_type, _) = state.add_block_to_witness_tree(&child10_block).unwrap();
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
        .for_each(|tree| assert_eq!(tree.height(), 1));
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.leaves().len(), 1));

    // -----------
    // add child 0
    // -----------

    let (extension_type, _) = state.add_block_to_witness_tree(&child0_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingSimpleForward);

    // ----------
    // add root 1
    // ----------

    let (extension_type, _) = state.add_block_to_witness_tree(&root1_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingSimpleReverse);

    // ------------
    // add child 11
    // ------------

    let (extension_type, _) = state.add_block_to_witness_tree(&child11_block).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingSimpleForward);

    // 2 dangling branches
    // - each height = 2
    // - 0: 1 leaf
    // - 1: 2 leaves
    assert_eq!(state.dangling_branches.len(), 2);
    state
        .dangling_branches
        .iter()
        .for_each(|tree| assert_eq!(tree.height(), 2));
    state
        .dangling_branches
        .iter()
        .enumerate()
        .for_each(|(idx, tree)| {
            if idx == 0 {
                assert_eq!(tree.leaves().len(), 1)
            } else if idx == 1 {
                assert_eq!(tree.leaves().len(), 2)
            }
        });

    // after extension quantities
    let root1 = state.dangling_branches.get(1).unwrap().root_block();
    let branches1 = &state.dangling_branches.get(1).unwrap().branches;
    let branch_root1 = &branches1
        .get(branches1.root_node_id().unwrap())
        .unwrap()
        .data();
    let leaves1: Vec<Block> = state.dangling_branches.get(1).unwrap().leaves().to_vec();

    // root1 is not a leaf
    assert_ne!(root1, leaves1.get(0).unwrap());
    println!(
        "\n=== After Branch 0 ===\n{:?}",
        &state.dangling_branches.get(0).unwrap()
    );
    println!(
        "\n=== After Branch 1 ===\n{:?}",
        &state.dangling_branches.get(1).unwrap()
    );

    // branch root should match the tree's root
    assert_eq!(&root1, branch_root1);
}
