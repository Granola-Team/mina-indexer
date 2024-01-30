pub mod blockchain_state;
pub mod consensus_state;
pub mod constants;

use self::{blockchain_state::*, consensus_state::*, constants::*};
use serde::{Deserialize, Serialize};

/// The Protocol State represents a snapshot of the blockchain's current state, including
/// consensus information, network parameters, and references to previous blocks.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: ProtocolStateBody,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolStateBody {
    pub genesis_state_hash: String,
    pub blockchain_state: BlockchainState,
    pub consensus_state: ConsensusState,
    pub constants: Constants,
}
