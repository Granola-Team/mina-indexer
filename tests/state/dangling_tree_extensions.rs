use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PrecomputedBlock, Block, BlockHash},
    state::{branch::Leaf, ExtensionType, State},
};

// extend a branch with a new leaf
#[tokio::test]
async fn dangling_simple_forward_extension_one_leaf() {
    //      0
    // 0 => |
    //      1
    let log_dir = PathBuf::from("../tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir);

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    if let Some(root_block) = block_parser.next().await.expect("IO Error on block_parser") {
        assert_eq!(
            root_block.state_hash,
            "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
        );
        let mut state = State::new(&root_block, None).unwrap();

        // before extension quantities
        let before_root = state.dangling_branches.get(0).unwrap().root.clone();
        let before_branches = state.dangling_branches.get(0).unwrap().branches.clone();
        let before_root_id = before_branches.root_node_id().unwrap();
        let before_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
        let before_root_leaf = before_branches.get(&before_root_id).unwrap().data();
        let before_leaf_block = &before_leaves
            .get(&before_root_id)
            .expect("before: root = leaf")
            .block;

        // before root is also a leaf
        assert_eq!(&before_root_leaf.block, before_leaf_block);

        // extend the branch with a child of the root
        // let mut state2 = State::new(&root_block, None).unwrap();
        let mut child_block: PrecomputedBlock = block_parser
            .next()
            .await
            .expect("IO Error on block_parser")
            .unwrap();
        while BlockHash::previous_state_hash(&child_block).block_hash != root_block.state_hash {
            child_block = block_parser
                .next()
                .await
                .expect("IO Error on block_parser")
                .expect("Ran out of logs to parse");
        }
        assert_eq!(
            child_block.state_hash,
            "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
        );

        // DanglingNew extension  type
        assert_eq!(state.add_block(&child_block), ExtensionType::DanglingNew);

        // after extension quantities
        let after_root = state.dangling_branches.get(0).unwrap().root.clone();
        let after_branches = state.dangling_branches.get(0).unwrap().branches.clone();
        let after_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
        let after_root_id = after_branches.root_node_id().unwrap();
        let after_root_leaf = after_branches.get(&after_root_id).unwrap().data();

        // branch root should match the tree's root
        assert_eq!(before_root, before_root_leaf.block);
        assert_eq!(&after_root, &after_root_leaf.block);

        let mut w = String::new();
        before_branches.write_formatted(&mut w).unwrap();
        println!("Before tree:{}", w);

        let mut w = String::new();
        after_branches.write_formatted(&mut w).unwrap();
        println!("After tree:{}", w);

        // extension-specific checks
        // before root has no children
        assert!(before_branches
            .children_ids(&before_root_id)
            .expect("before branch child")
            .next()
            .is_none());

        // after root has one child
        let mut after_children = after_branches
            .children_ids(&after_root_id)
            .expect("after branch child")
            .collect::<Vec<&NodeId>>();
        assert_eq!(after_children.len(), 1);
        let after_child = after_children.pop().unwrap();
        let after_child_block = Block::from_precomputed(&child_block, 1);
        assert_eq!(
            after_child_block,
            after_leaves
                .get(after_child)
                .expect("There should be a leaf block")
                .block
        );
        println!(
            "After leaf: {:?}",
            after_leaves.get(after_child).unwrap().block.clone()
        );

        // root shouldn't change
        assert_eq!(before_root, after_root);

        // only one node in before tree
        assert_eq!(before_branches.height(), 1);

        // after tree has one more node than before tree
        assert_eq!(after_branches.height(), 1 + before_branches.height());

        // before root is also a leaf
        assert!(before_leaves.contains_key(before_root_id));
        assert_eq!(
            before_leaves.get(&before_root_id).unwrap().block,
            before_root
        );

        // after root isn't a leaf
        assert!(!after_leaves.contains_key(after_root_id));
    }
}

/// extend a branch backwards with the root's parent
#[tokio::test]
async fn simple_backward_extension() {
    //      1
    // 0 => |
    //      0
    let log_dir = PathBuf::from("../tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir);

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    if let Some(new_root_block) = block_parser.next().await.expect("IO Error on block_parser") {
        // get the block we want
        assert_eq!(
            new_root_block.state_hash,
            "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
        );
        let mut old_root_block: PrecomputedBlock = block_parser
            .next()
            .await
            .expect("IO Error on block_parser")
            .unwrap();
        while BlockHash::previous_state_hash(&old_root_block).block_hash
            != new_root_block.state_hash
        {
            old_root_block = block_parser
                .next()
                .await
                .expect("IO Error on block_parser")
                .expect("Ran out of logs to parse");
        }
        // state is created with the child of the next block
        let mut state = State::new(&old_root_block, None).unwrap();

        // before extension quantities
        let before_root = state.dangling_branches.get(0).unwrap().root.clone();
        let before_branches = state.dangling_branches.get(0).unwrap().branches.clone();
        let before_root_id = before_branches.root_node_id().unwrap();
        let before_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
        let before_root_leaf = before_branches.get(&before_root_id).unwrap().data();
        let before_leaf_block = &before_leaves
            .get(&before_root_id)
            .expect("before: root = leaf")
            .block;

        println!("=== Before tree === ");
        println!(
            "Before leaves map: {:?}",
            before_leaves
                .clone()
                .iter()
                .collect::<Vec<(&NodeId, &Leaf<_>)>>()
        );

        // root is also a leaf
        assert_eq!(&before_root_leaf.block, before_leaf_block);

        // extend the branch with a parent of the root
        let extension_type1 = state.add_block(&old_root_block);
        assert_eq!(
            old_root_block.state_hash,
            "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
        );
        assert_eq!(extension_type1, ExtensionType::DanglingNew);
        let extension_type2 = state.add_block(&new_root_block);

        // after extension quantities
        let after_root = state.dangling_branches.get(0).unwrap().root.clone();
        let after_branches = state.dangling_branches.get(0).unwrap().branches.clone();
        let after_leaves = state.dangling_branches.get(0).unwrap().leaves.clone();
        let after_root_id = after_branches.root_node_id().unwrap();
        let after_root_leaf = after_branches.get(&after_root_id).unwrap().data();

        // simple reverse extension of a dangling branch
        assert_eq!(extension_type2, ExtensionType::DanglingSimpleReverse);

        let mut w = String::new();
        before_branches.write_formatted(&mut w).unwrap();
        println!("{}", w);

        println!("=== After tree ===");
        println!(
            "After leaves map: {:?}",
            after_leaves
                .clone()
                .iter()
                .collect::<Vec<(&NodeId, &Leaf<_>)>>()
                .iter()
                .map(|(n, l)| (n, l.block.clone()))
        );

        let mut w = String::new();
        after_branches.write_formatted(&mut w).unwrap();
        println!("{}", w);

        // extension-specific checks
        // before root has no children
        assert!(before_branches
            .children_ids(&before_root_id)
            .expect("before branch child")
            .next()
            .is_none());

        // after root has one child
        let mut after_children = after_branches
            .children_ids(&after_root_id)
            .expect("after branch child")
            .collect::<Vec<&NodeId>>();
        assert_eq!(after_children.len(), 1);
        let after_child = after_children.pop().unwrap();
        let after_child_block = Block::from_precomputed(&old_root_block, 1);
        assert_eq!(
            after_child_block,
            after_leaves
                .get(after_child)
                .expect("There should be a leaf block")
                .block
        );

        // branch root should match the tree's root
        assert_eq!(&before_root, &before_root_leaf.block);
        assert_eq!(&after_root, &after_root_leaf.block);

        // root shouldn't change
        assert_ne!(before_root, after_root);

        // only one node in before tree
        assert_eq!(before_branches.height(), 1);

        // after tree has one more node than before tree
        assert_eq!(after_branches.height(), 1 + before_branches.height());

        // before root is also a leaf
        assert!(before_leaves.contains_key(before_root_id));
        assert_eq!(
            before_leaves.get(&before_root_id).unwrap().block,
            before_root
        );

        // after root isn't a leaf
        assert!(!after_leaves.contains_key(after_root_id));
    }
}

// TODO genesis block => RootNew
// TODO more complex test scenarios
