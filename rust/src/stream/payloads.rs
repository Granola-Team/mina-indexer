use super::mainnet_block_models::{CommandSummary, CompletedWorks, FeeTransfer, FeeTransferViaCoinbase};
use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockAncestorPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize)]
pub struct BerkeleyBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MainnetBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub user_command_count: usize,
    pub user_commands: Vec<CommandSummary>,
    pub snark_work_count: usize,
    pub snark_work: Vec<CompletedWorks>,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub fee_transfer_via_coinbase: Option<FeeTransferViaCoinbase>,
    pub fee_transfers: Vec<FeeTransfer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockCanonicityUpdatePayload {
    pub height: u64,
    pub state_hash: String,
    pub canonical: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GenesisBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub unix_timestamp: u64,
}

impl Default for GenesisBlockPayload {
    fn default() -> Self {
        Self::new()
    }
}

impl GenesisBlockPayload {
    pub fn new() -> Self {
        Self {
            height: 1,
            state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            previous_state_hash: "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d".to_string(),
            last_vrf_output: "NfThG1r1GxQuhaGLSJWGxcpv24SudtXG4etB0TnGqwg=".to_string(),
            unix_timestamp: 1615939200000,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnarkWorkSummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub prover: String,
    pub fee: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnarkCanonicitySummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub prover: String,
    pub fee: f64,
    pub canonical: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BlockSummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub user_command_count: usize,
    pub snark_work_count: usize,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub last_vrf_output: String,
    pub is_berkeley_block: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserCommandSummaryPayload {
    pub height: u64,
    pub state_hash: String,
    // pub txn_hash: String,    //
    pub timestamp: u64,
    pub txn_type: String,
    pub status: String,
    pub sender: String,
    pub receiver: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub amount_nanomina: u64,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum InternalCommandType {
    Coinbase,
    FeeTransferViaCoinbase,
    FeeTransfer,
}

#[derive(Serialize, Deserialize)]
pub struct InternalCommandPayload {
    pub internal_command_type: InternalCommandType,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub amount_nanomina: u64,
    pub recipient: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserCommandCanonicityPayload {
    pub height: u64,
    pub state_hash: String,
    // pub txn_hash: String,    //
    pub timestamp: u64,
    pub txn_type: String,
    pub status: String,
    pub sender: String,
    pub receiver: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub amount_nanomina: u64,
    pub canonical: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum DoubleEntryType {
    BlockReward,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum TxnType {
    Debit,
    Credit,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LedgerEntry {
    pub value: u64,
    pub txn_type: TxnType,
    pub account: String, // New field for account information
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DoubleEntryPayload {
    pub entry_type: DoubleEntryType,
    pub lhs_entry: LedgerEntry,
    pub rhs_entry: Vec<LedgerEntry>,
}

// Builder for DoubleEntryPayload
#[derive(Debug)]
pub struct DoubleEntryPayloadBuilder {
    entry_type: Option<DoubleEntryType>,
    lhs_entry: Option<LedgerEntry>,
    rhs_entry: Vec<LedgerEntry>,
}

impl Default for DoubleEntryPayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubleEntryPayloadBuilder {
    pub fn new() -> Self {
        Self {
            entry_type: None,
            lhs_entry: None,
            rhs_entry: Vec::new(),
        }
    }

    pub fn entry_type(mut self, entry_type: DoubleEntryType) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    pub fn lhs_entry(mut self, value: u64, txn_type: TxnType, account: String) -> Self {
        self.lhs_entry = Some(LedgerEntry { value, txn_type, account });
        self
    }

    pub fn add_rhs_entry(mut self, value: u64, txn_type: TxnType, account: String) -> Self {
        self.rhs_entry.push(LedgerEntry { value, txn_type, account });
        self
    }

    pub fn build(self) -> Result<DoubleEntryPayload, &'static str> {
        Ok(DoubleEntryPayload {
            entry_type: self.entry_type.ok_or("entry_type is required")?,
            lhs_entry: self.lhs_entry.ok_or("lhs_entry is required")?,
            rhs_entry: self.rhs_entry,
        })
    }
}

impl DoubleEntryPayload {
    pub fn builder() -> DoubleEntryPayloadBuilder {
        DoubleEntryPayloadBuilder::new()
    }
}

#[cfg(test)]
mod double_entry_payload_builder_tests {
    use super::*;

    #[test]
    fn test_double_entry_payload_builder() {
        let payload = DoubleEntryPayload::builder()
            .entry_type(DoubleEntryType::BlockReward)
            .lhs_entry(1000, TxnType::Debit, "Account_A".to_string())
            .add_rhs_entry(500, TxnType::Credit, "Account_B".to_string())
            .add_rhs_entry(500, TxnType::Credit, "Account_C".to_string())
            .build()
            .expect("Failed to build DoubleEntryPayload");

        // Verify that the payload is built as expected
        assert_eq!(payload.entry_type, DoubleEntryType::BlockReward);
        assert_eq!(payload.lhs_entry.value, 1000);
        assert_eq!(payload.lhs_entry.txn_type, TxnType::Debit);
        assert_eq!(payload.lhs_entry.account, "Account_A");

        assert_eq!(payload.rhs_entry.len(), 2);
        assert_eq!(payload.rhs_entry[0].value, 500);
        assert_eq!(payload.rhs_entry[0].txn_type, TxnType::Credit);
        assert_eq!(payload.rhs_entry[0].account, "Account_B");

        assert_eq!(payload.rhs_entry[1].value, 500);
        assert_eq!(payload.rhs_entry[1].txn_type, TxnType::Credit);
        assert_eq!(payload.rhs_entry[1].account, "Account_C");
    }

    #[test]
    fn test_builder_missing_entry_type() {
        let result = DoubleEntryPayload::builder()
            .lhs_entry(1000, TxnType::Debit, "Account_A".to_string())
            .add_rhs_entry(500, TxnType::Credit, "Account_B".to_string())
            .build();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "entry_type is required");
    }

    #[test]
    fn test_builder_missing_lhs_entry() {
        let result = DoubleEntryPayload::builder()
            .entry_type(DoubleEntryType::BlockReward)
            .add_rhs_entry(500, TxnType::Credit, "Account_B".to_string())
            .build();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "lhs_entry is required");
    }
}
