use std::path::PathBuf;

use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::{branch::Branch, ledger::LedgerMock},
};

// extend a branch with a new leaf
#[tokio::test]
async fn simple_proper_extension() {
    //      0
    // 0 => |
    //      1
    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let root_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );
    // we declare this to be the root tree
    let root_tree = Branch::new(&root_block, LedgerMock {}).unwrap();

    // before extension quantities
    let before_root = root_tree.root;
    let before_branches = root_tree.branches;
    let before_root_id = before_branches.root_node_id().unwrap();
    let before_leaves = root_tree.leaves;
    let before_root_leaf = before_branches.get(&before_root_id).unwrap().data();
    let before_leaf_block = &before_leaves
        .get(&before_root_id)
        .expect("before: root = leaf")
        .block;

    // before root is also a leaf
    assert_eq!(&before_root_leaf.block, before_leaf_block);

    // extend the branch with a child of the root
    let mut tree2 = Branch::new(&root_block, LedgerMock {}).unwrap();
    let child_block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    assert_eq!(
        child_block.state_hash,
        "3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC".to_owned()
    );
    tree2
        .simple_extension(&child_block)
        .expect("new leaf should be inserted");

    // after extension quantities
    let after_root = tree2.root;
    let after_branches = tree2.branches;
    let after_leaves = tree2.leaves;
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

// TODO simple_forward_extension_many_leaves
//      0          0
//     / \        / \
//    1   2  =>  1   2
//                   |
//                   3

// TODO add dangling block to branch
// TODO complex extensions
