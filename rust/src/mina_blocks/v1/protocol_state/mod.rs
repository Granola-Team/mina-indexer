pub mod blockchain_state;
pub mod consensus_state;
pub mod constants;

use self::{blockchain_state::*, consensus_state::*, constants::*};
use crate::{
    block::BlockHash,
    mina_blocks::common::from_str,
    // protocol::serialization_types::protocol_state as mina_rs,
};
use serde::{Deserialize, Serialize};

/// The Protocol State represents a snapshot of the blockchain's current state,
/// including consensus information, network parameters, and references to
/// previous blocks.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolState {
    #[serde(deserialize_with = "from_str")]
    pub previous_state_hash: BlockHash,
    pub body: ProtocolStateBody,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolStateBody {
    #[serde(deserialize_with = "from_str")]
    pub genesis_state_hash: BlockHash,

    pub blockchain_state: BlockchainState,
    pub consensus_state: ConsensusState,
    pub constants: Constants,
}

// TODO
// impl From<mina_rs::ProtocolState> for ProtocolState {
//     fn from(value: mina_rs::ProtocolState) -> Self {
//         Self {

//         }
//     }
// }
