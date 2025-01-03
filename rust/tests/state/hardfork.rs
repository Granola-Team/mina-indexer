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
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());

        state.add_block_to_witness_tree(&block, true, false)?;
    }

    // root branch
    println!("=== Root Branch ===");
    println!("{}", state.root_branch);

    assert_eq!(state.root_branch.len(), 18);
    assert_eq!(state.root_branch.height(), 14);
    assert_eq!(state.root_branch.leaves().len(), 4);

    // dangling branches
    assert_eq!(state.dangling_branches.len(), 0);

    // best chain
    let best_chain = state
        .best_chain()
        .into_iter()
        .map(|b| b.state_hash.0)
        .collect::<Vec<_>>();

    assert_eq!(
        best_chain,
        vec![
            "3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW",
            "3NLjpotw6aZ2r7Twccgr7cceXiPkdH5LqdugWCpq9tL1ZZLeDsJV",
            "3NKP2tSFCcQ5G1wDZUaFcU5KpYPmorvnHndSQ3CbBgirZ7HTK7Nm",
            "3NK3EzoJEv4udD8DpTks8osNZwGuB9GEnDWC5kf4Wd2kXrapjaKR",
            "3NLcQZw2tNfFV6hRxEPcJhpTcwnhsDsLzU1B33cyRH1WkBFWGmvb",
            "3NLcUk9u8FgvCip634qaDQFBm26ja8C3pSk2L1SQd9nSE2CEcqpQ",
            "3NKybkb8C3R5PjwkxNUVCL6tb5qVf5i4jPWkDCcyJbka9Qgvr8CG",
            "3NLe669kJ89t48btn8NX6jMy7vnWNjP9caBdGgsCw2VSMjzP1anW",
            "3NL8ym45gfcDR18fyn7WMJVwgweb4C4HYWUnwNEQuyb5TsF8Hemn",
            "3NLdgCbegvkfGm29x9NryzESJCoCC7DknrcB2TAmzyvcZjtsvJ76",
            "3NKg81uwJ61tNNbM1SkS6862AHwfRhwNQEKZemJS9UwBAzaNK8ch"
        ]
    );

    Ok(())
}
