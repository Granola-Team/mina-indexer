use mina_indexer::block::{
    parser::BlockParser,
    precomputed::{PcbVersion, PrecomputedBlock},
};
use std::path::PathBuf;

/// Ingests v1 & v2 blocks
/// checks the block parser version before & after each
#[tokio::test]
async fn hardfork() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/post_hardfork");
    let mut block_parser = BlockParser::new_testing(&blocks_dir).unwrap();

    let v1_blocks = [
        "3NKTG8sg2vKQSUfe2D7nTxe1t4TDzRVubSxp4SUyHUWyXEpUVwqo".to_string(), /* mainnet-359605-3NKTG8sg2vKQSUfe2D7nTxe1t4TDzRVubSxp4SUyHUWyXEpUVwqo */
        "3NLw1pazmm1SWCqLLzbnwnBAKCzWR1KPVodKeXfbbp29fbJF5iio".to_string(), /* mainnet-359606-3NLw1pazmm1SWCqLLzbnwnBAKCzWR1KPVodKeXfbbp29fbJF5iio */
    ];
    let v2_blocks = [
        "3NK7T1MeiFA4ALVxqZLuGrWr1PeufYQAm9i1TfMnN9Cu6U5crhot".to_string(), /* mainnet-359606-3NK7T1MeiFA4ALVxqZLuGrWr1PeufYQAm9i1TfMnN9Cu6U5crhot */
        "3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg".to_string(), /* mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg */
        "3NKXo8ugDzgiJBc3zZTejrdJSoNXRCxM9kAEXzzxeCGzJRMX3NkP".to_string(), /* mainnet-359607-3NKXo8ugDzgiJBc3zZTejrdJSoNXRCxM9kAEXzzxeCGzJRMX3NkP */
    ];

    // start with default PCB version
    assert_eq!(block_parser.version, PcbVersion::default());

    while let Some((block, _)) = block_parser.next_block().await? {
        let pcb: PrecomputedBlock = block.into();

        // after consuming a v1 block
        if v1_blocks.contains(&pcb.state_hash().0) {
            assert_eq!(block_parser.version, PcbVersion::V1);
        }

        // after consuming a v2 block
        if v2_blocks.contains(&pcb.state_hash().0) {
            assert_eq!(block_parser.version, PcbVersion::V2);
        }
    }

    // final consumed block is v2
    assert_eq!(block_parser.version, PcbVersion::V2);
    Ok(())
}
