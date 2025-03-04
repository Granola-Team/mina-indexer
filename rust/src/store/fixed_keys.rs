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
    const TOTAL_NUM_ZKAPP_ACCOUNTS_KEY: &'static [u8] = "total_num_zkapp_accounts".as_bytes();

    const TOTAL_NUM_BLOCKS_KEY: &'static [u8] = "total_num_blocks".as_bytes();
    const TOTAL_NUM_BLOCKS_SUPERCHARGED_KEY: &'static [u8] =
        "total_num_blocks_supercharged".as_bytes();
    const TOTAL_NUM_SNARKS_KEY: &'static [u8] = "total_num_snarks".as_bytes();
    const TOTAL_NUM_CANONICAL_SNARKS_KEY: &'static [u8] = "total_num_canonical_snarks".as_bytes();
    const TOTAL_NUM_FEE_TRANSFERS_KEY: &'static [u8] = "total_num_fee_transfers".as_bytes();
    const TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY: &'static [u8] =
        "total_num_canonical_fee_transfers_key".as_bytes();
    const TOTAL_NUM_NON_CANONICAL_FEE_TRANSFERS_KEY: &'static [u8] =
        "total_num_non_canonical_fee_transfers_key".as_bytes();

    // all user commands (non-zkapp & zkapp)
    const TOTAL_NUM_USER_COMMANDS_KEY: &'static [u8] = "total_num_user_commands".as_bytes();
    const TOTAL_NUM_APPLIED_USER_COMMANDS_KEY: &'static [u8] =
        "total_num_applied_user_commands_key".as_bytes();
    const TOTAL_NUM_FAILED_USER_COMMANDS_KEY: &'static [u8] =
        "total_num_failed_user_commands_key".as_bytes();
    const TOTAL_NUM_APPLIED_CANONICAL_USER_COMMANDS_KEY: &'static [u8] =
        "total_num_applied_canonical_user_commands_key".as_bytes();
    const TOTAL_NUM_FAILED_CANONICAL_USER_COMMANDS_KEY: &'static [u8] =
        "total_num_failed_canonical_user_commands_key".as_bytes();
    const TOTAL_NUM_CANONICAL_USER_COMMANDS_KEY: &'static [u8] =
        "total_num_canonical_user_commands_key".as_bytes();

    // zkapp user commands
    const TOTAL_NUM_ZKAPP_COMMANDS_KEY: &'static [u8] = "total_num_zkapp_commands".as_bytes();
    const TOTAL_NUM_APPLIED_ZKAPP_COMMANDS_KEY: &'static [u8] =
        "total_num_applied_zkapp_commands_key".as_bytes();
    const TOTAL_NUM_FAILED_ZKAPP_COMMANDS_KEY: &'static [u8] =
        "total_num_failed_zkapp_commands_key".as_bytes();
    const TOTAL_NUM_APPLIED_CANONICAL_ZKAPP_COMMANDS_KEY: &'static [u8] =
        "total_num_applied_canonical_zkapp_commands_key".as_bytes();
    const TOTAL_NUM_FAILED_CANONICAL_ZKAPP_COMMANDS_KEY: &'static [u8] =
        "total_num_failed_canonical_zkapp_commands_key".as_bytes();
    const TOTAL_NUM_CANONICAL_ZKAPP_COMMANDS_KEY: &'static [u8] =
        "total_num_canonical_zkapp_commands_key".as_bytes();

    const ZKAPP_TOKEN_COUNT: &'static [u8] = "zkapp_token_count".as_bytes();
}
