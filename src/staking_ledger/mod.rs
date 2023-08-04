pub mod staking_ledger_store;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StakingLedger {
    pub epoch_number: u32,
    pub ledger_hash: String,
    pub accounts: Vec<StakingLedgerAccount>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NanoMina(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce(u32);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StakingLedgerAccount {
    pub pk: String,
    pub balance: String,
    pub delegate: String,
    pub epoch_number: Option<i32>,
    pub ledger_hash: Option<String>,
    pub nonce: Option<String>, // u32
    pub receipt_chain_hash: String,
    pub token: String, // u32
    pub voting_for: String,
}
