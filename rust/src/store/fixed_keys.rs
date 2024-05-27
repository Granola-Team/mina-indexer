pub trait FixedKeys {
    const CHAIN_ID_KEY: &'static [u8] = "current_chain_id".as_bytes();
    const BEST_TIP_BLOCK_KEY: &'static [u8] = "best_tip_block".as_bytes();
    const NEXT_EVENT_SEQ_NUM_KEY: &'static [u8] = "next_event_seq_num".as_bytes();
    const MAX_CANONICAL_KEY: &'static [u8] = "max_canonical_blockchain_length".as_bytes();
    const TOTAL_NUM_BLOCKS_KEY: &'static [u8] = "total_num_blocks".as_bytes();
    const KNOWN_GENESIS_STATE_HASHES_KEY: &'static [u8] = "genesis_state_hashes".as_bytes();
    const KNOWN_GENESIS_PREV_STATE_HASHES_KEY: &'static [u8] =
        "genesis_prev_state_hashes".as_bytes();
}
