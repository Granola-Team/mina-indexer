use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::chain::{store::ChainStore, ChainId, Network};
use log::trace;

impl ChainStore for IndexerStore {
    fn set_chain_id_for_network(
        &self,
        chain_id: &ChainId,
        network: &Network,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting chain id '{}' for network '{}'",
            chain_id.0,
            network
        );

        let chain_bytes = chain_id.0.as_bytes();

        // add the new pair
        self.database.put_cf(
            self.chain_id_to_network_cf(),
            chain_bytes,
            network.to_string().as_bytes(),
        )?;

        // update current chain_id
        self.database.put(Self::CHAIN_ID_KEY, chain_bytes)?;
        Ok(())
    }

    fn get_network(&self, chain_id: &ChainId) -> anyhow::Result<Network> {
        trace!("Getting network for chain id: {}", chain_id.0);
        Ok(Network::from(
            self.database
                .get_pinned_cf(self.chain_id_to_network_cf(), chain_id.0.as_bytes())?
                .expect("network should exist in database")
                .to_vec(),
        ))
    }

    fn get_current_network(&self) -> anyhow::Result<Network> {
        trace!("Getting current network");
        self.get_network(&self.get_chain_id()?)
    }

    fn get_chain_id(&self) -> anyhow::Result<ChainId> {
        trace!("Getting chain id");
        Ok(ChainId(String::from_utf8(
            self.database
                .get(Self::CHAIN_ID_KEY)?
                .expect("chain id should exist in database"),
        )?))
    }
}
