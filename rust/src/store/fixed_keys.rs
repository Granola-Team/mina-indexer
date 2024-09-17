pub trait FixedKeys {
    const CHAIN_ID_KEY: &'static [u8] = "current_chain_id".as_bytes();
    const BEST_TIP_STATE_HASH_KEY: &'static [u8] = "best_tip_state_hash".as_bytes();
    const NEXT_EVENT_SEQ_NUM_KEY: &'static [u8] = "next_event_seq_num".as_bytes();
    const MAX_CANONICAL_KEY: &'static [u8] = "max_canonical_blockchain_length".as_bytes();
    const KNOWN_GENESIS_STATE_HASHES_KEY: &'static [u8] = "genesis_state_hashes".as_bytes();
    const KNOWN_GENESIS_PREV_STATE_HASHES_KEY: &'static [u8] =
        "genesis_prev_state_hashes".as_bytes();
    const NUM_BLOCK_BYTES_PROCESSED: &'static [u8] = "num_block_bytes_processed".as_bytes();

    // version info
    const INDEXER_STORE_VERSION_KEY: &'static [u8] = "indexer_store_version".as_bytes();

    // indexed totals
    const TOTAL_NUM_ACCOUNTS_KEY: &'static [u8] = "total_num_accounts".as_bytes();
    const TOTAL_NUM_BLOCKS_KEY: &'static [u8] = "total_num_blocks".as_bytes();
    const TOTAL_NUM_BLOCKS_SUPERCHARGED_KEY: &'static [u8] =
        "total_num_blocks_supercharged".as_bytes();
    const TOTAL_NUM_SNARKS_KEY: &'static [u8] = "total_num_snarks".as_bytes();
    const TOTAL_NUM_FEE_TRANSFERS_KEY: &'static [u8] = "total_num_fee_transfers".as_bytes();
    const TOTAL_NUM_USER_COMMANDS_KEY: &'static [u8] = "total_num_user_commands".as_bytes();
}
