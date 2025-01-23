//! Zkapp action store trait

use crate::{
    ledger::{public_key::PublicKey, token::TokenAddress},
    mina_blocks::v2::ActionState,
    store::Result,
};

pub trait ZkappActionStore {
    /// Add actions to the token account
    ///
    /// Returns the total number of actions for the account
    fn add_actions(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        actions: &[ActionState],
    ) -> Result<u32>;

    /// Get the `index`th action for the token account
    fn get_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ActionState>>;

    /// Set the `index`th action for the token account
    fn set_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        action: &ActionState,
        index: u32,
    ) -> Result<()>;

    /// Get the total number of actions associated with the token account
    fn get_num_actions(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>>;

    /// Remove the specified number of actions from the account
    ///
    /// Returns the number of remaining actions
    fn remove_actions(&self, pk: &PublicKey, token: &TokenAddress, num: u32) -> Result<u32>;

    /// Remove the action at the specified index from the account
    fn remove_action(&self, pk: &PublicKey, token: &TokenAddress, index: u32) -> Result<()>;
}
