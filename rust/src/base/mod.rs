//! Indexer base types

pub mod amount;
pub mod blockchain_length;
pub mod nonce;
pub mod numeric;
pub mod public_key;
pub mod scheduled_time;
pub mod state_hash;

pub type Balance = numeric::Numeric<u64>;
