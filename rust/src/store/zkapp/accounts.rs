//! Zkapp account store

use crate::{
    base::public_key::PublicKey,
    ledger::{account::Account, token::TokenAddress},
    store::Result,
};

pub trait ZkappAccountStore {
    /// Add the zkapp account
    fn add_zkapp_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        zkapp_account: Account,
    ) -> Result<()>;
}
