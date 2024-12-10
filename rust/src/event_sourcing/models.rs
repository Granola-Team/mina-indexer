use core::fmt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreviousStateHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateHashPreviousStateHash {
    pub state_hash: String,
    pub previous_state_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletedWorksNanomina {
    pub fee_nanomina: u64,
    pub prover: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Height(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LastVrfOutput(pub String);

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum CommandStatus {
    Applied,
    #[default]
    Failed,
}

impl fmt::Display for CommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            CommandStatus::Applied => "Applied",
            CommandStatus::Failed => "Failed",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum CommandType {
    #[default]
    Payment,
    StakeDelegation,
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            CommandType::Payment => "Payment",
            CommandType::StakeDelegation => "StakeDelegation",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default, Hash, Debug)]
pub struct CommandSummary {
    pub memo: String,
    pub fee_payer: String,
    pub sender: String,
    pub receiver: String,
    pub status: CommandStatus,
    pub txn_type: CommandType,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub amount_nanomina: u64,
}

impl CommandSummary {
    // TODO: this needs to use bin_prot
    // but for now we'll make up a hash
    pub fn txn_hash(&self) -> String {
        let serialized = sonic_rs::to_string(self).unwrap();
        // Use a SHA-256 hasher
        let mut hasher = Sha256::new();
        hasher.update(serialized);
        // Return the hash as a hexadecimal string
        format!("{:x}", hasher.finalize())
    }
}
