//! Chain data

pub mod store;

mod id;
mod network;

use crate::{base::state_hash::StateHash, block::precomputed::PcbVersion, constants::*};
use std::collections::HashMap;

// re-export types
pub type ChainId = id::ChainId;
pub type Network = network::Network;

#[derive(Debug)]
pub struct ChainData(pub HashMap<StateHash, (PcbVersion, ChainId)>);

/////////////
// default //
/////////////

impl std::default::Default for ChainData {
    fn default() -> Self {
        // v1 chain data
        let v1_genesis_state_hash: StateHash = MAINNET_GENESIS_HASH.into();
        let v1_chain_id = ChainId::v1();

        // v2 chain data
        let v2_genesis_state_hash: StateHash = HARDFORK_GENESIS_HASH.into();
        let v2_chain_id = ChainId::v2();

        Self(HashMap::from([
            (v1_genesis_state_hash, (PcbVersion::V1, v1_chain_id)),
            (v2_genesis_state_hash, (PcbVersion::V2, v2_chain_id)),
        ]))
    }
}
