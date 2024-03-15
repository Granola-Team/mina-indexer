// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Types related to the Mina protocol state

use crate::protocol::serialization_types::{
    blockchain_state::{BlockchainStateJson, BlockchainStateV1},
    common::{HashV1, StateHashV1Json},
    consensus_state::{ConsensusStateJson, ConsensusStateV1},
    protocol_constants::{ProtocolConstantsJson, ProtocolConstantsV1},
};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::Versioned2;
use serde::{Deserialize, Serialize};

/// Body of the protocol state
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProtocolStateBody {
    /// Genesis protocol state hash (used for hardforks)
    pub genesis_state_hash: HashV1,
    /// Ledger related state
    pub blockchain_state: BlockchainStateV1,
    /// Consensus related state
    pub consensus_state: ConsensusStateV1,
    /// Consensus constants
    pub constants: ProtocolConstantsV1,
}

/// Body of the protocol state (v1)
pub type ProtocolStateBodyV1 = Versioned2<ProtocolStateBody, 1, 1>;

/// Body of the protocol state (json)
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ProtocolStateBody)]
pub struct ProtocolStateBodyJson {
    /// Genesis protocol state hash (used for hardforks)
    pub genesis_state_hash: StateHashV1Json,
    /// Ledger related state
    pub blockchain_state: BlockchainStateJson,
    /// Consensus related state
    pub consensus_state: ConsensusStateJson,
    /// Consensus constants
    pub constants: ProtocolConstantsJson,
}
