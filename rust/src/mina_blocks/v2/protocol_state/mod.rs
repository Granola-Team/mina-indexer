use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, LedgerHash},
    mina_blocks::common::*,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolState {
    #[serde(deserialize_with = "from_str")]
    pub previous_state_hash: BlockHash,

    pub body: ProtocolStateBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolStateBody {
    #[serde(deserialize_with = "from_str")]
    pub genesis_state_hash: BlockHash,

    pub blockchain_state: BlockchainState,
    pub consensus_state: ConsensusState,
    pub constants: GenesisConstants,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenesisConstants {
    #[serde(deserialize_with = "from_str")]
    pub k: u32,

    #[serde(deserialize_with = "from_str")]
    pub slots_per_epoch: u32,

    #[serde(deserialize_with = "from_str")]
    pub slots_per_sub_window: u32,

    #[serde(deserialize_with = "from_str")]
    pub delta: u32,

    #[serde(deserialize_with = "from_str_opt")]
    pub grace_period_slots: Option<u32>,

    #[serde(deserialize_with = "from_str")]
    pub genesis_state_timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockchainState {
    #[serde(deserialize_with = "from_str")]
    pub genesis_ledger_hash: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub timestamp: u64,

    pub ledger_proof_statement: LedgerProofStatement,
    pub staged_ledger_hash: StagedLedgerHash,
    pub body_reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerProofStatement {
    #[serde(deserialize_with = "from_str")]
    pub connecting_ledger_left: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub connecting_ledger_right: String,

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupplyAdjustment {
    #[serde(deserialize_with = "from_str")]
    pub magnitude: u64,
    pub sgn: (SupplyAdjustmentSign,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SupplyAdjustmentSign {
    Pos,
    Neg,
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
pub struct LocalState {
    pub stack_frame: String,
    pub call_stack: String,
    pub transaction_commitment: String,
    pub full_transaction_commitment: String,
    pub excess: SupplyAdjustment,
    pub supply_increase: SupplyAdjustment,

    #[serde(deserialize_with = "from_str")]
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
    #[serde(deserialize_with = "from_str")]
    pub ledger_hash: LedgerHash,

    pub aux_hash: String,
    pub pending_coinbase_aux: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusState {
    #[serde(deserialize_with = "from_str")]
    pub blockchain_length: u32,

    #[serde(deserialize_with = "from_str")]
    pub epoch_count: u32,

    #[serde(deserialize_with = "from_str")]
    pub min_window_density: u32,

    #[serde(deserialize_with = "vec_from_str")]
    pub sub_window_densities: Vec<String>,

    #[serde(deserialize_with = "from_str")]
    pub last_vrf_output: String,

    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,

    #[serde(deserialize_with = "from_str")]
    pub global_slot_since_genesis: u32,
    pub curr_global_slot_since_hard_fork: GlobalSlotNumbers,
    pub staking_epoch_data: EpochData,
    pub next_epoch_data: EpochData,
    pub has_ancestor_in_same_checkpoint_window: bool,

    #[serde(deserialize_with = "from_str")]
    pub block_stake_winner: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub block_creator: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub coinbase_receiver: PublicKey,
    pub supercharge_coinbase: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalSlotNumbers {
    #[serde(deserialize_with = "from_str")]
    pub slot_number: u32,

    #[serde(deserialize_with = "from_str")]
    pub slots_per_epoch: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpochData {
    pub ledger: LedgerData,
    pub seed: String,

    #[serde(deserialize_with = "from_str")]
    pub start_checkpoint: BlockHash,

    #[serde(deserialize_with = "from_str")]
    pub lock_checkpoint: BlockHash,

    #[serde(deserialize_with = "from_str")]
    pub epoch_length: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerData {
    #[serde(deserialize_with = "from_str")]
    pub hash: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,
}
