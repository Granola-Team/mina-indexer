use crate::{
    ledger::{public_key::PublicKey, LedgerHash},
    mina_blocks::common::*,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedWork {
    #[serde(deserialize_with = "from_decimal_str")]
    pub fee: u64,

    #[serde(deserialize_with = "from_str")]
    pub prover: PublicKey,
    // pub proofs: Proofs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Proofs {
    One(ProofKind, Proof),
    Two(ProofKind, Proof, Proof),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    pub statement: Statement,
    pub proof: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofKind {
    One,
    Two,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    #[serde(deserialize_with = "from_str")]
    pub connecting_ledger_left: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub connecting_ledger_right: LedgerHash,

    pub source: Source,
    pub target: Source,
    pub supply_increase: SupplyIncrease,
    pub fee_excess: Vec<FeeExcess>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    #[serde(deserialize_with = "from_str")]
    pub first_pass_ledger: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub second_pass_ledger: LedgerHash,

    pub pending_coinbase_stack: PendingCoinbaseStack,
    pub local_state: LocalState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingCoinbaseStack {
    pub data: String,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub init: String,
    pub curr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalState {
    #[serde(deserialize_with = "from_str")]
    pub ledger: LedgerHash,

    pub stack_frame: String,
    pub call_stack: String,
    pub transaction_commitment: String,
    pub full_transaction_commitment: String,
    pub excess: SupplyIncrease,
    pub supply_increase: SupplyIncrease,
    pub success: bool,
    pub account_update_index: String,

    // TODO
    pub failure_status_tbl: Vec<Option<serde_json::Value>>,
    pub will_succeed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeExcess {
    pub token: String,
    pub amount: SupplyIncrease,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupplyIncrease {
    pub magnitude: String,
    pub sgn: Vec<Sgn>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Sgn {
    Neg,
    Pos,
}
