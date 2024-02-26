// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Structure of a global slot

use crate::protocol::serialization_types::common::{GlobalSlotNumberV1, LengthV1, U32Json};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::Versioned2;
use serde::{Deserialize, Serialize};

/// A global slot
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct GlobalSlot {
    /// The global slot number of a chain or block
    pub slot_number: GlobalSlotNumberV1,
    /// Number of slots per epoch
    pub slots_per_epoch: LengthV1,
}

/// A global slot (v1)
pub type GlobalSlotV1 = Versioned2<GlobalSlot, 1, 1>;

/// A global slot (json)
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(GlobalSlot)]
pub struct GlobalSlotJson {
    /// The global slot number of a chain or block
    pub slot_number: U32Json,
    /// Number of slots per epoch
    pub slots_per_epoch: U32Json,
}
