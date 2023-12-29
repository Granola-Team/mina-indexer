pub mod block;
pub mod canonical;
pub mod client;
pub mod ipc;
pub mod receiver;
pub mod server;
pub mod state;
pub mod store;

use state::ledger::account::Amount;

pub const BLOCK_REPORTING_FREQ_NUM: u32 = 5000;
pub const BLOCK_REPORTING_FREQ_SEC: u64 = 180;
pub const CANONICAL_UPDATE_THRESHOLD: u32 = PRUNE_INTERVAL_DEFAULT / 5;
pub const MAINNET_CANONICAL_THRESHOLD: u32 = 10;
pub const MAINNET_GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
pub const MAINNET_TRANSITION_FRONTIER_K: u32 = 290;
pub const MAINNET_ACCOUNT_CREATION_FEE: Amount = Amount(1e9 as u64);
pub const PRUNE_INTERVAL_DEFAULT: u32 = 10;
pub const SOCKET_NAME: &str = "mina-indexer.sock";
