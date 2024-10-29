// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Types related to the Transaction Snark Work
#![allow(missing_docs)]

use crate::protocol::serialization_types::{
    common::*,
    signatures::{PublicKeyJson, PublicKeyV1},
};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::{
    impl_mina_enum_json_serde, impl_mina_enum_json_serde_with_option, Versioned, Versioned2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionSnarkWork {
    // Versioned 1 byte
    pub fee: AmountV1,
    pub prover: PublicKeyV1,
}

pub type TransactionSnarkWorkV1 = Versioned<TransactionSnarkWork, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionSnarkWork)]
pub struct TransactionSnarkWorkJson {
    // Versioned 1 byte
    pub fee: DecimalJson,
    pub prover: PublicKeyJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionSnark {
    pub statement: StatementV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionSnark)]
pub struct TransactionSnarkJson {
    pub statement: StatementJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Statement {
    // Versioned 2 byte
    pub source: HashV1,
    pub target: HashV1,
    pub supply_increase: AmountV1,
    pub pending_coinbase_stack_state: PendingCoinbaseStackStateV1,
    pub fee_excess: FeeExcessPairV1,
    pub next_available_token_before: TokenIdV1,
    pub next_available_token_after: TokenIdV1,
    pub sok_digest: ByteVecV1,
}

pub type StatementV1 = Versioned2<Statement, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(Statement)]
pub struct StatementJson {
    // Versioned 2 byte
    pub source: LedgerHashV1Json,
    pub target: LedgerHashV1Json,
    pub supply_increase: U64Json,
    pub pending_coinbase_stack_state: PendingCoinbaseStackStateJson,
    pub fee_excess: FeeExcessPairJson,
    pub next_available_token_before: U64Json,
    pub next_available_token_after: U64Json,
    pub sok_digest: ByteVecJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PendingCoinbaseStackState {
    // Versioned 2 byte
    pub source: PendingCoinbaseV1,
    pub target: PendingCoinbaseV1,
}

pub type PendingCoinbaseStackStateV1 = Versioned2<PendingCoinbaseStackState, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(PendingCoinbaseStackState)]
pub struct PendingCoinbaseStackStateJson {
    // Versioned 2 byte
    pub source: PendingCoinbaseJson,
    pub target: PendingCoinbaseJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PendingCoinbase {
    // Versioned 2 byte
    pub data_stack: HashV1,
    pub state_stack: StateStackV1,
}

pub type PendingCoinbaseV1 = Versioned2<PendingCoinbase, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(PendingCoinbase)]
pub struct PendingCoinbaseJson {
    #[serde(rename = "data")]
    pub data_stack: CoinBaseStackDataV1Json,
    #[serde(rename = "state")]
    pub state_stack: StateStackJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StateStack {
    // Versioned 2 byte
    pub init: HashV1,
    pub curr: HashV1,
}

pub type StateStackV1 = Versioned2<StateStack, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(StateStack)]
pub struct StateStackJson {
    // Versioned 2 byte
    pub init: CoinBaseStackHashV1Json,
    pub curr: CoinBaseStackHashV1Json,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FeeExcess {
    pub token: TokenIdV1,
    pub amount: SignedV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(FeeExcess)]
pub struct FeeExcessJson {
    pub token: U64Json,
    pub amount: SignedJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FeeExcessPair(pub FeeExcess, pub FeeExcess);

pub type FeeExcessPairV1 = Versioned2<FeeExcessPair, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(FeeExcessPair)]
pub struct FeeExcessPairJson(pub FeeExcessJson, pub FeeExcessJson);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Signed {
    // Versioned 1 byte
    pub magnitude: AmountV1,
    pub sgn: SgnTypeV1,
}

pub type SignedV1 = Versioned<Signed, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(Signed)]
pub struct SignedJson {
    // Versioned 1 byte
    pub magnitude: DecimalJson,
    pub sgn: SgnTypeJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SgnType {
    // Versioned 1 byte
    Pos,
    Neg,
}

pub type SgnTypeV1 = Versioned<SgnType, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
pub enum SgnTypeJsonProxy {
    Pos,
    Neg,
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(SgnType)]
#[auto_from(SgnTypeJsonProxy)]
pub enum SgnTypeJson {
    Pos,
    Neg,
}

impl_mina_enum_json_serde!(SgnTypeJson, SgnTypeJsonProxy);
