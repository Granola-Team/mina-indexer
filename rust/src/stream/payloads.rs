use super::mainnet_block_models::{CommandStatus, CommandSummary, CommandType, CompletedWorks, FeeTransfer, FeeTransferViaCoinbase};
use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NewAccountPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub account: String,
}

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
    pub fee_transfer_via_coinbase: Option<Vec<FeeTransferViaCoinbase>>,
    pub fee_transfers: Vec<FeeTransfer>,
}

impl MainnetBlockPayload {
    pub fn accounts(&self) -> Vec<String> {
        let snark_accounts: Vec<String> = self.snark_work.iter().map(|s| s.prover.to_string()).collect();
        let fee_transfer_accounts: Vec<String> = self.fee_transfers.iter().map(|ft| ft.recipient.to_string()).collect();
        let user_command_sender_accounts: Vec<String> = self.user_commands.iter().map(|ft| ft.sender.to_string()).collect();
        let user_command_receiver_accounts: Vec<String> = self.user_commands.iter().map(|ft| ft.receiver.to_string()).collect();
        let user_command_fee_payer_accounts: Vec<String> = self.user_commands.iter().map(|ft| ft.fee_payer.to_string()).collect();
        let accounts: Vec<String> = [
            snark_accounts,
            fee_transfer_accounts,
            user_command_sender_accounts,
            user_command_receiver_accounts,
            user_command_fee_payer_accounts,
            vec![self.coinbase_receiver.to_string()],
        ]
        .into_iter()
        .flatten()
        .collect();
        let unique: HashSet<_> = accounts.into_iter().collect();
        unique.into_iter().collect()
    }
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
    pub was_canonical: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GenesisBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub unix_timestamp: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockConfirmationPayload {
    pub height: u64,
    pub state_hash: String,
    pub confirmations: u8,
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
pub struct BlockLogPayload {
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
pub struct CanonicalBlockLogPayload {
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
    pub canonical: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    // pub txn_hash: String,    //
    pub timestamp: u64,
    pub txn_type: CommandType,
    pub status: CommandStatus,
    pub sender: String,
    pub receiver: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub fee_payer: String,
    pub amount_nanomina: u64,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub enum InternalCommandType {
    Coinbase,
    FeeTransferViaCoinbase,
    FeeTransfer,
}

use std::{collections::HashSet, fmt};

impl fmt::Display for InternalCommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            InternalCommandType::Coinbase => "Coinbase",
            InternalCommandType::FeeTransferViaCoinbase => "FeeTransferViaCoinbase",
            InternalCommandType::FeeTransfer => "FeeTransfer",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InternalCommandPayload {
    pub internal_command_type: InternalCommandType,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub amount_nanomina: u64,
    pub recipient: String,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InternalCommandCanonicityPayload {
    pub internal_command_type: InternalCommandType,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub amount_nanomina: u64,
    pub recipient: String,
    pub source: Option<String>,
    pub canonical: bool,
    pub was_canonical: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CanonicalUserCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    // pub txn_hash: String,    //
    pub timestamp: u64,
    pub txn_type: CommandType,
    pub status: CommandStatus,
    pub sender: String,
    pub receiver: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub fee_payer: String,
    pub amount_nanomina: u64,
    pub canonical: bool,
    pub was_canonical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleEntryRecordPayload {
    pub height: u64,
    pub state_hash: String,
    // pub txn_hash: u64,
    pub lhs: Vec<AccountingEntry>, // Multiple debit entries
    pub rhs: Vec<AccountingEntry>, // Multiple credit entries
}

impl DoubleEntryRecordPayload {
    pub fn verify(&self) {
        assert_eq!(
            self.lhs.iter().map(|e| e.amount_nanomina).sum::<u64>(),
            self.rhs.iter().map(|e| e.amount_nanomina).sum::<u64>()
        )
    }

    pub fn contains(&self, account: &str) -> bool {
        self.lhs.iter().chain(self.rhs.iter()).filter(|ae| ae.account == account).count() > 0
    }

    pub fn accounts(&self) -> Vec<String> {
        self.lhs.iter().chain(self.rhs.iter()).map(|a| a.account.to_string()).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountingEntryType {
    Debit,
    Credit,
}

impl fmt::Display for AccountingEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            AccountingEntryType::Debit => "Debit",
            AccountingEntryType::Credit => "Credit",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountingEntryAccountType {
    VirtualAddess,
    BlockchainAddress,
}

impl fmt::Display for AccountingEntryAccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            AccountingEntryAccountType::VirtualAddess => "VirtualAddress",
            AccountingEntryAccountType::BlockchainAddress => "BlockchainAddress",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountingEntry {
    pub entry_type: AccountingEntryType, // "debit" or "credit"
    pub account: String,
    pub account_type: AccountingEntryAccountType,
    pub amount_nanomina: u64,
    pub timestamp: u64,
}

impl AccountingEntry {
    pub fn contains(&self, account: &str) -> bool {
        self.account == account
    }
}
