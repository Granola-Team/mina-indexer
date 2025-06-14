//! Zkapp action store trait

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    command::TxnHash,
    ledger::token::TokenAddress,
    mina_blocks::v2::{zkapp::action_state::ActionStateWithMeta, ActionState},
    store::Result,
};
use speedb::{DBIterator, Direction};

pub trait ZkappActionStore {
    /// Add actions to the token account
    ///
    /// Returns the total number of actions for the account
    fn add_actions(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        actions: &[ActionState],
        state_hash: &StateHash,
        block_height: u32,
        txn_hash: &TxnHash,
    ) -> Result<u32>;

    /// Get the `index`th action for the token account
    fn get_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ActionStateWithMeta>>;

    /// Set the `index`th action for the token account
    fn set_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        action: &ActionStateWithMeta,
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

    // Iterators //
    ///////////////

    /// Iterator over pk/token actions from start height (inclusive) to end
    /// height (exclusive)
    fn actions_iterator(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        direction: Direction,
    ) -> DBIterator<'_>;
}
