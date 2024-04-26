use super::ChainId;
use crate::block::Network;

pub trait ChainIdStore {
    /// Persists a (chain id, network) pair
    ///
    /// Error propogates from db
    fn set_chain_id_for_network(&self, chain_id: &ChainId, network: &Network)
        -> anyhow::Result<()>;

    /// Gets the network for the given chain id
    ///
    /// Error if not present
    fn get_network(&self, chain_id: &ChainId) -> anyhow::Result<Network>;

    /// Gets the current network
    ///
    /// Error if not present
    fn get_current_network(&self) -> anyhow::Result<Network>;

    /// Gets the current chain id
    ///
    /// Error if not present
    fn get_chain_id(&self) -> anyhow::Result<ChainId>;
}
