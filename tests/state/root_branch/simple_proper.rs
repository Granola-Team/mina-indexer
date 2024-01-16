use id_tree::NodeId;
use mina_indexer::{
    block::{parser::BlockParser, Block},
    state::branch::Branch,
};
use std::path::PathBuf;

// extend a branch with a new leaf
#[tokio::test]
async fn extension() {
    //      0
    // 0 => |
    //      1
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
    // we declare this to be the root tree
    let root_tree = Branch::new(&root_block).unwrap();

    // before extension quantities
    let before_root = root_tree.root_block();
    let before_branches = root_tree.branches.clone();
    let before_root_id = before_branches.root_node_id().unwrap();
    let before_root_leaf = before_branches.get(before_root_id).unwrap().data();
    let before_leaf_block = before_branches.get(before_root_id).unwrap().data();

    // before root is also a leaf
    assert_eq!(before_root_leaf, before_leaf_block);

    // extend the branch with a child of the root
    let mut tree2 = Branch::new(&root_block).unwrap();
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
    let after_root = tree2.root_block();
    let after_branches = tree2.branches.clone();
    let after_root_id = after_branches.root_node_id().unwrap();
    let after_root_leaf = after_branches.get(after_root_id).unwrap().data();

    // branch root should match the tree's root
    assert_eq!(before_root, before_root_leaf);
    assert_eq!(after_root, after_root_leaf);

    println!("=== Before tree ===");
    println!("{root_tree:?}");

    println!("=== After tree ===");
    println!("{tree2:?}");

    // extension-specific checks
    // before root has no children
    assert!(before_branches
        .children_ids(before_root_id)
        .expect("before branch child")
        .next()
        .is_none());

    // after root has one child
    let mut after_children = after_branches
        .children_ids(after_root_id)
        .expect("after branch child")
        .collect::<Vec<&NodeId>>();
    assert_eq!(after_children.len(), 1);

    let after_child = after_children.pop().unwrap();
    let after_child_block = Block::from_precomputed(&child_block, 1);
    assert_eq!(
        &after_child_block,
        after_branches
            .get(after_child)
            .expect("There should be a leaf block")
            .data()
    );

    let a_leaf = after_branches.get(after_child).unwrap().data().clone();
    println!("After leaf: {a_leaf:?}");

    // root shouldn't change
    assert_eq!(before_root, after_root);

    // only one node in before tree
    assert_eq!(before_branches.height(), 1);

    // after tree has one more node than before tree
    assert_eq!(after_branches.height(), 1 + before_branches.height());

    // before root is also a leaf
    assert_eq!(
        before_root,
        before_branches.get(before_root_id).unwrap().data()
    );

    // after root isn't a leaf
    assert!(tree2.leaves().into_iter().all(|x| x != after_root.clone()));
}
