//! Token account trait

use super::address::TokenAddress;
use crate::base::public_key::PublicKey;

pub trait TokenAccount {
    fn public_key(&self) -> PublicKey;

    fn token(&self) -> TokenAddress;
}
