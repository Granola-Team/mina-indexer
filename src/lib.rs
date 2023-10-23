pub mod block;
pub mod client;
pub mod gql;
pub mod ipc;
pub mod receiver;
pub mod server;
pub mod staking_ledger;
pub mod state;
pub mod store;

pub const BLOCK_REPORTING_FREQ_NUM: u32 = 5000;
pub const BLOCK_REPORTING_FREQ_SEC: u64 = 180;
pub const CANONICAL_UPDATE_THRESHOLD: u32 = PRUNE_INTERVAL_DEFAULT / 5;
pub const MAINNET_CANONICAL_THRESHOLD: u32 = 10;
pub const MAINNET_GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
pub const MAINNET_TRANSITION_FRONTIER_K: u32 = 290;
pub const PRUNE_INTERVAL_DEFAULT: u32 = 10;
pub const SOCKET_NAME: &str = "mina-indexer.sock";

pub fn display_duration(duration: std::time::Duration) -> String {
    let duration_as_secs = duration.as_secs();
    let duration_as_mins = duration_as_secs as f32 / 60.;
    let duration_as_hrs = duration_as_mins / 60.;

    if duration_as_mins < 2. {
        format!("{duration:?}")
    } else if duration_as_hrs < 2. {
        format!("{duration_as_mins}min")
    } else {
        format!("{duration_as_hrs}hr")
    }
}
