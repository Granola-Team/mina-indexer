use mina_indexer::{
    block::{parser::BlockParser, precomputed::PrecomputedBlock},
    state::IndexerState,
};
use std::path::PathBuf;

/// Parses all blocks in ./tests/data/sequential_blocks
/// Adds them to a fresh state
/// Verifies the faithfullness of the correspondence between dangling branch
/// `leaves` and the underlying tree's leaf blocks Verifies the length of the
/// longest chain
//#[tokio::test]
async fn _extension() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&blocks_dir)?;

    let mut n = 0;
    if let Some((block, block_bytes)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        let mut state =
            IndexerState::new_testing(&block, block_bytes, None, None, None, None, None)?;
        n += 1;

        while let Some((block, _)) = block_parser.next_block().await? {
            let block: PrecomputedBlock = block.into();
            state.add_block_to_witness_tree(&block, true)?;
            n += 1;
        }

        println!("{state}");

        // all blocks parsed successfully
        println!("Blocks parsed and added: {n}");
        assert_eq!(n, 36);

        // root branch
        assert_eq!(state.root_branch.len(), 34);
        assert_eq!(state.root_branch.height(), 13);
        assert_eq!(state.root_branch.leaves().len(), 21);

        // dangling branches
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
        println!("{:?}", state.root_branch);

        // dangling branches
        for (n, branch) in state.dangling_branches.iter().enumerate() {
            println!("=== Dangling Branch {n} ===");
            println!("{branch:?}");
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
                        child_block.blockchain_length,
                        1 + parent_block.blockchain_length
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
        assert_eq!(longest_chain.len(), 13);
    }

    Ok(())
}
