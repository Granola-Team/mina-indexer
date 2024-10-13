use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
    },
    state::IndexerState,
};
use std::path::PathBuf;

/// Witness tree ingests:
/// - the final pre-hardfork v1 block,
/// - hardfork genesis block,
/// - post-hardfork v1 & v2 blocks
#[tokio::test]
async fn hardfork() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/hardfork");
    let mut block_parser =
        BlockParser::new_length_sorted_filtered(&blocks_dir, PcbVersion::default(), None, None)?;

    // final pre-harfork v1 block
    let (root_block, root_block_bytes) = block_parser.next_block().await?.unwrap();
    let root_block: PrecomputedBlock = root_block.into();
    assert_eq!(
        root_block.state_hash().0,
        "3NLRTfY4kZyJtvaP4dFenDcxfoMfT3uEpkWS913KkeXLtziyVd15"
    );

    // root the witness tree at the final pre-harfork v1 block
    let mut state =
        IndexerState::new_testing(&root_block, root_block_bytes, None, None, None, None, None)?;

    // ingest the remaining blocks
    while let Some((block, _)) = block_parser.next_block().await? {
        state.add_block_to_witness_tree(&block.into(), true, false)?;
    }

    // root branch
    println!("=== Root Branch ===");
    println!("{}", state.root_branch);

    assert_eq!(state.root_branch.len(), 7);
    assert_eq!(state.root_branch.height(), 4);
    assert_eq!(state.root_branch.leaves().len(), 3);

    // dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    let best_chain = state
        .best_chain()
        .into_iter()
        .map(|b| b.state_hash.0)
        .collect::<Vec<_>>();
    assert_eq!(
        best_chain,
        vec![
            "3NKXo8ugDzgiJBc3zZTejrdJSoNXRCxM9kAEXzzxeCGzJRMX3NkP",
            "3NK7T1MeiFA4ALVxqZLuGrWr1PeufYQAm9i1TfMnN9Cu6U5crhot",
            "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP",
            "3NLRTfY4kZyJtvaP4dFenDcxfoMfT3uEpkWS913KkeXLtziyVd15"
        ]
    );

    Ok(())
}
