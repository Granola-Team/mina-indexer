// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Proof messages used by the protocol state proof

#![allow(missing_docs)] // Don't actually know what many of the types fields are for yet

use crate::protocol::serialization_types::field_and_curve_elements::{
    ECPointJson, ECPointV1, ECPointVecJson, ECPointVecV1, FiniteECPoint, FiniteECPointJson,
};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::{Versioned, Versioned2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProofMessages {
    pub l_comm: ProofMessageWithoutDegreeBoundListV1,
    pub r_comm: ProofMessageWithoutDegreeBoundListV1,
    pub o_comm: ProofMessageWithoutDegreeBoundListV1,
    pub z_comm: ProofMessageWithoutDegreeBoundListV1,
    pub t_comm: ProofMessageWithDegreeBoundV1,
}

pub type ProofMessagesV1 = Versioned<ProofMessages, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ProofMessages)]
pub struct ProofMessagesJson {
    pub l_comm: ProofMessageWithoutDegreeBoundListJson,
    pub r_comm: ProofMessageWithoutDegreeBoundListJson,
    pub o_comm: ProofMessageWithoutDegreeBoundListJson,
    pub z_comm: ProofMessageWithoutDegreeBoundListJson,
    pub t_comm: ProofMessageWithDegreeBoundJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProofMessageWithoutDegreeBoundList(pub Vec<FiniteECPoint>);

pub type ProofMessageWithoutDegreeBoundListV1 =
    Versioned2<ProofMessageWithoutDegreeBoundList, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ProofMessageWithoutDegreeBoundList)]
pub struct ProofMessageWithoutDegreeBoundListJson(pub Vec<FiniteECPointJson>);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProofMessageWithDegreeBound {
    pub unshifted: ECPointVecV1,
    pub shifted: ECPointV1,
}

pub type ProofMessageWithDegreeBoundV1 = Versioned<ProofMessageWithDegreeBound, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ProofMessageWithDegreeBound)]
pub struct ProofMessageWithDegreeBoundJson {
    pub unshifted: ECPointVecJson,
    pub shifted: ECPointJson,
}
