// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Types related to the Mina protocol state

use crate::protocol::serialization_types::common::{BlockTimeV1, LengthV1, U32Json, U64Json};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::Versioned2;
use serde::{Deserialize, Serialize};

/// Constants that define the consensus parameters
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProtocolConstants {
    /// Point of finality (number of confirmations)
    pub k: LengthV1,
    /// Number of slots per epoch
    pub slots_per_epoch: LengthV1,
    /// No of slots in a sub-window = 7
    pub slots_per_sub_window: LengthV1,
    /// Maximum permissable delay of packets (in slots after the current)
    pub delta: LengthV1,
    /// Timestamp of genesis block in unixtime
    pub genesis_state_timestamp: BlockTimeV1,
}

/// Constants that define the consensus parameters (v1)
pub type ProtocolConstantsV1 = Versioned2<ProtocolConstants, 1, 1>;

/// Constants that define the consensus parameters (json)
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ProtocolConstants)]
pub struct ProtocolConstantsJson {
    /// Point of finality (number of confirmations)
    pub k: U32Json,
    /// Number of slots per epoch
    pub slots_per_epoch: U32Json,
    /// No of slots in a sub-window = 7
    pub slots_per_sub_window: U32Json,
    /// Maximum permissable delay of packets (in slots after the current)
    pub delta: U32Json,
    /// Timestamp of genesis block in unixtime
    pub genesis_state_timestamp: U64Json,
}
