use mina_indexer::{block::parser::BlockParser, state::IndexerState};
use std::path::PathBuf;

/// Parses all blocks in ./tests/data/sequential_blocks
/// Adds them to a fresh state
/// Verifies the faithfullness of the correspondence between dangling branch `leaves` and the underlying tree's leaf blocks
/// Verifies the length of the longest chain
#[tokio::test]
async fn extension() {
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    let mut n = 0;
    if let Some(precomputed_block) = block_parser.next().await.unwrap() {
        let mut state = IndexerState::new_testing(&precomputed_block, None, None, None).unwrap();
        n += 1;
        while let Some(precomputed_block) = block_parser.next().await.unwrap() {
            state.add_block(&precomputed_block).unwrap();
            n += 1;
        }

        // All 24 blocks are parsed successfully
        println!("Blocks added: {n}");
        assert_eq!(n, 24);

        // Root branch
        // - height = 10
        // - ??? leaves
        assert_eq!(state.dangling_branches.clone().len(), 2);

        // 2 dangling branch
        // - 1: height = 1
        // - 2: height = 1
        // - 1 leaf
        assert_eq!(state.dangling_branches.clone().len(), 2);
        state
            .dangling_branches
            .iter()
            .enumerate()
            .for_each(|(_, tree)| {
                assert_eq!(tree.height(), 1);
            });
        state
            .dangling_branches
            .iter()
            .enumerate()
            .for_each(|(_, tree)| {
                assert_eq!(tree.leaves().len(), 1);
            });

        // root branch
        println!("=== Root Branch ===");
        let mut tree = String::new();
        state
            .root_branch
            .branches
            .write_formatted(&mut tree)
            .unwrap();
        println!("{tree}");

        // dangling branches
        for (n, branch) in state.dangling_branches.iter().enumerate() {
            println!("=== Dangling Branch {n} ===");
            let mut tree = String::new();
            branch.branches.write_formatted(&mut tree).unwrap();
            println!("{tree}");
        }

        // check all children.height = 1 + parent.height
        // check all children.parent_hash = parent.state_hash
        // check all children.parent_hash = parent.state_hash
        for dangling_branch in state.dangling_branches.iter() {
            for node_id in dangling_branch
                .branches
                .traverse_level_order_ids(dangling_branch.branches.root_node_id().unwrap())
                .unwrap()
            {
                let node = dangling_branch.branches.get(&node_id).unwrap();
                for child_id in node.children() {
                    let parent_block = &dangling_branch.branches.get(&node_id).unwrap().data();
                    let child_block = &dangling_branch.branches.get(child_id).unwrap().data();
                    assert_eq!(child_block.height, 1 + parent_block.height);
                    assert_eq!(
                        child_block.blockchain_length.unwrap(),
                        1 + parent_block.blockchain_length.unwrap()
                    );
                    assert_eq!(
                        node.data().state_hash,
                        dangling_branch
                            .branches
                            .get(child_id)
                            .unwrap()
                            .data()
                            .parent_hash
                    );
                }
            }
        }

        // longest chain
        let longest_chain = state.root_branch.longest_chain();
        let display_chain = longest_chain
            .iter()
            .map(|x| &x.0[0..12])
            .collect::<Vec<&str>>();

        println!("=== Longest dangling chain ===");
        println!("{display_chain:?}");

        // ten blocks in the longest chain
        assert_eq!(longest_chain.len(), 10);
    }
}
