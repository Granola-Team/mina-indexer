pub const GENESIS_STATE_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
pub const TRANSITION_FRONTIER_DISTANCE: usize = 290;
pub const MAINNET_COINBASE_REWARD: u64 = 720_000_000_000;
pub const CHANNEL_MESSAGE_CAPACITY: usize = 10_000_000;
pub const POSTGRES_CONNECTION_STRING: &str = "host=localhost port=9002 user=mina_indexer password=mina_indexer dbname=mina_indexer";
pub const FILE_PUBLISHER_ACTOR_ID: &str = "FilePublisher";
pub const MAINNET_EPOCH_SLOT_COUNT: u64 = 7140;
pub const HEIGHT_SPREAD_MSG_THROTTLE: usize = 10_000; // we get an update on every 10000th height notification
