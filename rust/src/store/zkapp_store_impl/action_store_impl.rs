use crate::{
    ledger::{public_key::PublicKey, token::TokenAddress},
    mina_blocks::v2::ActionState,
    store::{zkapp::actions::ZkappActionStore, IndexerStore, Result},
};

impl ZkappActionStore for IndexerStore {
    fn add_actions(
        &self,
        _pk: &PublicKey,
        _token: &TokenAddress,
        _actions: Vec<ActionState>,
    ) -> Result<u32> {
        todo!()
    }

    fn get_action(
        &self,
        _pk: &PublicKey,
        _token: &TokenAddress,
        _n: u32,
    ) -> Result<Option<ActionState>> {
        todo!()
    }

    fn get_num_actions(&self, _pk: &PublicKey, _token: &TokenAddress) -> Result<Option<u32>> {
        todo!()
    }

    fn remove_actions(&self, _pk: &PublicKey, _token: &TokenAddress, _n: u32) -> Result<u32> {
        todo!()
    }
}
