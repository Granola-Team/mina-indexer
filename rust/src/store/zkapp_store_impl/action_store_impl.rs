//! Zkapp action store impl

use crate::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    mina_blocks::v2::ActionState,
    store::{
        column_families::ColumnFamilyHelpers, zkapp::actions::ZkappActionStore, IndexerStore,
        Result,
    },
    utility::store::{
        common::from_be_bytes,
        zkapp::actions::{zkapp_actions_key, zkapp_actions_pk_num_key},
    },
};
use anyhow::Context;
use log::trace;

impl ZkappActionStore for IndexerStore {
    fn add_actions(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        actions: &[ActionState],
    ) -> Result<u32> {
        trace!("Adding actions to token account ({pk}, {token}): {actions:?}");

        let idx = self.get_num_actions(pk, token)?.unwrap_or_default();
        let mut num = idx;

        // add each action
        for action in actions.iter() {
            self.set_action(pk, token, action, num)?;
            num += 1;
        }

        // update number of actions
        self.database.put_cf(
            self.zkapp_actions_pk_num_cf(),
            zkapp_actions_pk_num_key(token, pk),
            num.to_be_bytes(),
        )?;

        Ok(num)
    }

    fn get_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ActionState>> {
        trace!("Getting action {index} for token account ({pk}, {token})");

        Ok(self
            .database
            .get_pinned_cf(self.zkapp_actions_cf(), zkapp_actions_key(token, pk, index))?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .context(format!("missing {index} action for ({pk}, {token})"))
                    .unwrap()
            }))
    }

    fn set_action(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        action: &ActionState,
        index: u32,
    ) -> Result<()> {
        trace!("Setting action {index} for token account ({pk}, {token})");

        Ok(self.database.put_cf(
            self.zkapp_actions_cf(),
            zkapp_actions_key(token, pk, index),
            serde_json::to_vec(action)?,
        )?)
    }

    fn get_num_actions(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting number of actions for token account ({pk}, {token})");

        Ok(self
            .database
            .get_cf(
                self.zkapp_actions_pk_num_cf(),
                zkapp_actions_pk_num_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn remove_actions(&self, pk: &PublicKey, token: &TokenAddress, n: u32) -> Result<u32> {
        trace!("Removing {n} actions from token account ({pk}, {token})");

        let mut num = self.get_num_actions(pk, token)?.unwrap_or_default();
        assert!(n <= num);

        // remove each action
        for _ in 0..n {
            num -= 1;
            self.remove_action(pk, token, num)?;
        }

        // update number of actions
        self.database.put_cf(
            self.zkapp_actions_pk_num_cf(),
            zkapp_actions_pk_num_key(token, pk),
            num.to_be_bytes(),
        )?;

        Ok(num)
    }

    fn remove_action(&self, pk: &PublicKey, token: &TokenAddress, index: u32) -> Result<()> {
        trace!("Removing {index}-th action from token account ({pk}, {token})");

        Ok(self
            .database
            .delete_cf(self.zkapp_actions_cf(), zkapp_actions_key(token, pk, index))?)
    }
}
