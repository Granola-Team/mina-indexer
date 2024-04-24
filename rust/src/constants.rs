use crate::ledger::account::Amount;

// indexer constants
pub const BLOCK_REPORTING_FREQ_NUM: u32 = 1000;
pub const BLOCK_REPORTING_FREQ_SEC: u64 = 180;
pub const LEDGER_CADENCE: u32 = 100;
pub const CANONICAL_UPDATE_THRESHOLD: u32 = PRUNE_INTERVAL_DEFAULT / 5;
pub const MAINNET_CANONICAL_THRESHOLD: u32 = 10;
pub const PRUNE_INTERVAL_DEFAULT: u32 = 10;

// mina constants
pub const MAINNET_BLOCK_SLOT_TIME_MILLIS: u64 = 180000;
pub const MAINNET_GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
pub const MAINNET_GENESIS_PREV_STATE_HASH: &str =
    "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d";
pub const MAINNET_GENESIS_LAST_VRF_OUTPUT: &str = "NfThG1r1GxQuhaGLSJWGxcpv24SudtXG4etB0TnGqwg=";
pub const MAINNET_GENESIS_TIMESTAMP: u64 = 1615939200000;
pub const MAINNET_GENESIS_LEDGER_HASH: &str = "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee";
pub const MAINNET_TRANSITION_FRONTIER_K: u32 = 290;
pub const MAINNET_ACCOUNT_CREATION_FEE: Amount = Amount(1e9 as u64);
pub const MAINNET_COINBASE_REWARD: u64 = 720000000000;
pub const MAINNET_EPOCH_SLOT_COUNT: u32 = 7140;
pub const MAINNET_CHAIN_ID: &str =
    "5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1";

/// Convert epoch milliseconds to global slot number
pub fn millis_to_global_slot(millis: i64) -> u64 {
    (millis as u64 - MAINNET_GENESIS_TIMESTAMP) / MAINNET_BLOCK_SLOT_TIME_MILLIS
}
