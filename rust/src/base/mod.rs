//! Indexer base types

pub mod amount;
pub mod blockchain_length;
pub mod check;
pub mod delegate;
pub mod nonce;
pub mod numeric;
pub mod public_key;
pub mod scheduled_time;
pub mod state_hash;
pub mod username;

pub type Balance = numeric::Numeric<u64>;
