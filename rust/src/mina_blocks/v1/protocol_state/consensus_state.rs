use crate::{
    base::{
        blockchain_length::BlockchainLength, numeric::Numeric, public_key::PublicKey,
        state_hash::StateHash, Balance,
    },
    ledger::LedgerHash,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub blockchain_length: BlockchainLength,
    pub epoch_count: Numeric<u32>,
    pub min_window_density: Numeric<u32>,
    pub total_currency: Balance,
    pub global_slot_since_genesis: Numeric<u32>,
    pub sub_window_densities: Vec<Numeric<u32>>,
    pub block_stake_winner: PublicKey,
    pub block_creator: PublicKey,
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
    pub slot_number: Numeric<u32>,
    pub slots_per_epoch: Numeric<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StakingEpochData {
    pub ledger: Ledger,
    pub seed: String,

    #[serde(deserialize_with = "from_str")]
    pub start_checkpoint: StateHash,

    #[serde(deserialize_with = "from_str")]
    pub lock_checkpoint: StateHash,
    pub epoch_length: Numeric<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(deserialize_with = "from_str")]
    pub hash: LedgerHash,

    #[serde(deserialize_with = "from_str")]
    pub total_currency: u64,
}
