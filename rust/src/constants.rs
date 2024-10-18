use crate::ledger::account::Amount;
use chrono::{DateTime, SecondsFormat, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// version

pub const GIT_COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("GIT_COMMIT_HASH"));

// indexer constants

pub const BLOCK_REPORTING_FREQ_NUM: u32 = 1000;
pub const BLOCK_REPORTING_FREQ_SEC: u64 = 180;
pub const LEDGER_CADENCE: u32 = 100;
pub const CANONICAL_UPDATE_THRESHOLD: u32 = PRUNE_INTERVAL_DEFAULT / 5;
pub const MAINNET_CANONICAL_THRESHOLD: u32 = 10;
pub const PRUNE_INTERVAL_DEFAULT: u32 = 10;

// mina constants

pub const MINA_SCALE: u64 = 1_000_000_000;
pub const MINA_SCALE_DEC: Decimal = dec!(1_000_000_000);

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

// protocol constants

pub const MAINNET_PROTOCOL_CONSTANTS: &[u32] = &[
    MAINNET_TRANSITION_FRONTIER_K,
    MAINNET_EPOCH_SLOT_COUNT,
    MAINNET_SLOTS_PER_SUB_WINDOW,
    MAINNET_DELTA,
    MAINNET_TXPOOL_MAX_SIZE,
];
pub const MAINNET_EPOCH_SLOT_COUNT: u32 = 7140;
pub const MAINNET_SLOTS_PER_SUB_WINDOW: u32 = 7;
pub const MAINNET_DELTA: u32 = 0;
pub const MAINNET_TXPOOL_MAX_SIZE: u32 = 3000;

// constraint system digests

pub const MAINNET_CONSTRAINT_SYSTEM_DIGESTS: &[&str] = &[
    MAINNET_DIGEST_TXN_MERGE,
    MAINNET_DIGEST_TXN_BASE,
    MAINNET_DIGEST_BLOCKCHAIN_STEP,
];
pub const MAINNET_DIGEST_TXN_MERGE: &str = "d0f8e5c3889f0f84acac613f5c1c29b1";
pub const MAINNET_DIGEST_TXN_BASE: &str = "922bd415f24f0958d610607fc40ef227";
pub const MAINNET_DIGEST_BLOCKCHAIN_STEP: &str = "06d85d220ad13e03d51ef357d2c9d536";

pub const MAINNET_CHAIN_ID: &str =
    "5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1";

// post hardfork

pub const HARDFORK_GENSIS_HASH: &str = "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP";
pub const HARDFORK_GENESIS_TIMESTAMP: u64 = 1717545600000;
pub const HARDFORK_GENESIS_PREV_STATE_HASH: &str =
    "3NLRTfY4kZyJtvaP4dFenDcxfoMfT3uEpkWS913KkeXLtziyVd15";
pub const HARDFORK_GENESIS_LEDGER_HASH: &str =
    "jwNw4qb6tnNhpQNxiMLem9WumxZTwmbSx3fYXW4FP3hZRkoQJSE";
pub const HARDFORK_GENESIS_LAST_VRF_OUTPUT: &str =
    "FSBXKqZKgSiy1T6SsjbrT0i84oDkBpUVsLH1zRviuIj0DjuGEXs=";

pub const HARDFORK_CONSTRAINT_SYSTEM_DIGESTS: &[&str] = &[
    HARDFORK_DIGEST_TXN_MERGE,
    HARDFORK_DIGEST_TXN_BASE,
    HARDFORK_DIGEST_BLOCKCHAIN_STEP,
];
pub const HARDFORK_DIGEST_TXN_MERGE: &str = "b8879f677f622a1d86648030701f43e1";
pub const HARDFORK_DIGEST_TXN_BASE: &str = "d31948e661cc662675b0c079458f714a";
pub const HARDFORK_DIGEST_BLOCKCHAIN_STEP: &str = "14ab5562ed292de7a3deb9e12f00aec0";

pub const HARDFORK_PROTOCOL_NETWORK_VERSION_DIGEST: &str = "eccbc87e4b5ce2fe28308fd9f2a7baf3";
pub const HARDFORK_PROTOCOL_TXN_VERSION_DIGEST: &str = "eccbc87e4b5ce2fe28308fd9f2a7baf3";

pub const HARDFORK_CHAIN_ID: &str =
    "a7351abc7ddf2ea92d1b38cc8e636c271c1dfd2c081c637f62ebc2af34eb7cc1";

// Name service constants
pub const MINA_EXPLORER_NAME_SERVICE_ADDRESS: &str =
    "B62qjzJvc59DdG9ahht9rwxkEz7GedKuUMsnaVTuXFUeANKqfBeWpRE";
pub const MINA_SEARCH_NAME_SERVICE_ADDRESS: &str =
    "B62qjMINASEARCHMINASEARCHMINASEARCHMINASEARCHMINASEARCH";
pub const NAME_SERVICE_MEMO_PREFIX: &str = "Name: ";

/// Convert epoch milliseconds to an ISO 8601 formatted date
pub fn millis_to_iso_date_string(millis: i64) -> String {
    from_timestamp_millis(millis).to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Convert epoch milliseconds to DateTime<Utc>
pub fn from_timestamp_millis(millis: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(millis).unwrap()
}

/// Convert epoch milliseconds to global slot number
pub fn millis_to_global_slot(millis: i64) -> u32 {
    let millis_since_genesis = millis as u64 - MAINNET_GENESIS_TIMESTAMP;
    (millis_since_genesis / MAINNET_BLOCK_SLOT_TIME_MILLIS) as u32
}

pub mod berkeley {
    pub const BERKELEY_GENESIS_STATE_HASH: &str =
        "3NK512ryRJvj1TUKGgPoGZeHSNbn37e9BbnpyeqHL9tvKLeD8yrY";
    pub const BERKELEY_GENESIS_TIMESTAMP: u64 = 1706882461000;
}
