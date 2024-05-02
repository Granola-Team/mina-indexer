use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, LedgerHash},
    mina_blocks::common::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusState {
    #[serde(deserialize_with = "from_str")]
    pub blockchain_length: u32,

    #[serde(deserialize_with = "from_str")]
    pub epoch_count: u32,

    #[serde(deserialize_with = "from_str")]
    pub min_window_density: u32,

    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,

    #[serde(deserialize_with = "from_str")]
    pub global_slot_since_genesis: u32,

    #[serde(deserialize_with = "vec_from_str")]
    pub sub_window_densities: Vec<u32>,

    #[serde(deserialize_with = "from_str")]
    pub block_stake_winner: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub block_creator: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub coinbase_receiver: PublicKey,

    pub last_vrf_output: String,
    pub curr_global_slot: CurrGlobalSlot,
    pub staking_epoch_data: StakingEpochData,
    pub next_epoch_data: StakingEpochData,
    pub has_ancestor_in_same_checkpoint_window: bool,
    pub supercharge_coinbase: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrGlobalSlot {
    #[serde(deserialize_with = "from_str")]
    pub slot_number: u32,

    #[serde(deserialize_with = "from_str")]
    pub slots_per_epoch: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StakingEpochData {
    pub ledger: Ledger,
    pub seed: String,

    #[serde(deserialize_with = "from_str")]
    pub start_checkpoint: BlockHash,

    #[serde(deserialize_with = "from_str")]
    pub lock_checkpoint: BlockHash,

    #[serde(deserialize_with = "from_str")]
    pub epoch_length: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(deserialize_with = "from_str")]
    pub hash: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,
}
