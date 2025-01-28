pub mod constants;

use crate::{
    base::{
        blockchain_length::BlockchainLength, numeric::Numeric, public_key::PublicKey,
        scheduled_time::ScheduledTime, state_hash::StateHash, Balance,
    },
    block::vrf_output::VrfOutput,
    ledger::{public_key::PublicKey, LedgerHash},
    mina_blocks::common::*,
};
use constants::Constants;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolState {
    pub previous_state_hash: StateHash,
    pub body: ProtocolStateBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolStateBody {
    pub genesis_state_hash: StateHash,
    pub blockchain_state: BlockchainState,
    pub consensus_state: ConsensusState,
    pub constants: Constants,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockchainState {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub timestamp: u64,

    pub genesis_ledger_hash: LedgerHash,
    pub ledger_proof_statement: LedgerProofStatement,
    pub staged_ledger_hash: StagedLedgerHash,
    pub body_reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerProofStatement {
    pub connecting_ledger_left: LedgerHash,
    pub connecting_ledger_right: LedgerHash,
    pub source: Source,
    pub target: Source,
    pub supply_increase: SupplyAdjustment,
    pub fee_excess: Vec<FeeExcess>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeExcess {
    pub token: String,
    pub amount: SupplyAdjustment,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SupplyAdjustment {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub magnitude: u64,

    pub sgn: (SupplyAdjustmentSign,),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SupplyAdjustmentSign {
    Pos,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub first_pass_ledger: LedgerHash,
    pub second_pass_ledger: LedgerHash,
    pub pending_coinbase_stack: PendingCoinbaseStack,
    pub local_state: LocalState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalState {
    pub stack_frame: String,
    pub call_stack: String,
    pub transaction_commitment: String,
    pub full_transaction_commitment: String,
    pub excess: SupplyAdjustment,
    pub supply_increase: SupplyAdjustment,
    pub ledger: LedgerHash,
    pub success: bool,
    pub account_update_index: String,
    pub failure_status_tbl: Vec<Option<serde_json::Value>>,
    pub will_succeed: bool,
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
pub struct StagedLedgerHash {
    pub non_snark: NonSnark,
    pub pending_coinbase_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonSnark {
    pub ledger_hash: LedgerHash,
    pub aux_hash: String,
    pub pending_coinbase_aux: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub epoch_count: Numeric<u32>,
    pub min_window_density: Numeric<u32>,
    pub sub_window_densities: Vec<Numeric<u32>>,
    pub global_slot_since_genesis: Numeric<u32>,
    pub blockchain_length: BlockchainLength,
    pub last_vrf_output: VrfOutput,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub global_slot_since_genesis: u32,

    pub blockchain_length: BlockchainLength,
    pub curr_global_slot_since_hard_fork: GlobalSlotNumbers,
    pub staking_epoch_data: EpochData,
    pub next_epoch_data: EpochData,
    pub has_ancestor_in_same_checkpoint_window: bool,
    pub block_stake_winner: PublicKey,
    pub block_creator: PublicKey,
    pub coinbase_receiver: PublicKey,
    pub supercharge_coinbase: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalSlotNumbers {
    pub slot_number: Numeric<u32>,
    pub slots_per_epoch: Numeric<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpochData {
    pub ledger: LedgerData,
    pub seed: String,
    pub start_checkpoint: StateHash,
    pub lock_checkpoint: StateHash,
    pub epoch_length: Numeric<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerData {
    pub total_currency: Numeric<u64>,
    pub hash: LedgerHash,
}

/////////////////
// Conversions //
/////////////////

impl From<&SupplyAdjustment> for i64 {
    fn from(value: &SupplyAdjustment) -> Self {
        match value.sgn.0 {
            SupplyAdjustmentSign::Neg => -(value.magnitude as i64),
            SupplyAdjustmentSign::Pos => value.magnitude as i64,
        }
    }
}
