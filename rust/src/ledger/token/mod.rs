//! Token account, address, id, & symbol

// trait
pub mod account;

// types
mod address;
mod id;
mod symbol;

use crate::base::{amount::Amount, public_key::PublicKey};
use serde::{Deserialize, Serialize};

// re-export types
pub type TokenAddress = address::TokenAddress;
pub type TokenId = id::TokenId;
pub type TokenSymbol = symbol::TokenSymbol;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Token {
    address: TokenAddress,
    owner: Option<PublicKey>,
    symbol: TokenSymbol,
    supply: Amount,
}
