use super::{
    canonical_items_manager::CanonicalItem,
    models::{CommandStatus, CommandSummary, CommandType, CompletedWorksNanomina, FeeTransfer, FeeTransferViaCoinbase, ZkAppCommandSummary},
};
use crate::constants::MAINNET_COINBASE_REWARD;
use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UsernamePayload {
    pub username: String,
    pub address: String,
    pub height: u64,
    pub state_hash: String,
    pub canonical: bool,
}

#[derive(Serialize, Deserialize)]
pub struct NewAccountPayload {
    pub apply: bool,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub account: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StakingLedgerEntryPayload {
    pub epoch: u64,
    pub delegate: String,
    pub stake: u64,
    pub total_staked: u64,
    pub delegators_count: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ActorHeightPayload {
    pub height: u64,
    pub actor: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockAncestorPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BerkeleyBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub user_command_count: usize,
    pub user_commands: Vec<CommandSummary>,
    pub zk_app_command_count: usize,
    pub zk_app_commands: Vec<ZkAppCommandSummary>,
    pub snark_work_count: usize,
    pub snark_work: Vec<CompletedWorksNanomina>,
    pub fee_transfers: Vec<FeeTransfer>,
    pub fee_transfer_via_coinbase: Option<Vec<FeeTransferViaCoinbase>>,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EpochStakeDelegationPayload {
    pub height: u64,
    pub state_hash: String,
    pub epoch: u64,
    pub stake_nanomina: u64,
    pub source: String,
    pub recipient: String,
    pub canonical: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MainnetBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub user_command_count: usize,
    pub internal_command_count: usize,
    pub user_commands: Vec<CommandSummary>,
    pub snark_work_count: usize,
    pub snark_work: Vec<CompletedWorksNanomina>,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub fee_transfer_via_coinbase: Option<Vec<FeeTransferViaCoinbase>>,
    pub fee_transfers: Vec<FeeTransfer>,
    pub global_slot: u64,
}

impl MainnetBlockPayload {
    pub fn valid_accounts(&self) -> Vec<String> {
        // Helper function to extract accounts from user commands based on a mapper
        fn extract_accounts<F>(commands: &[CommandSummary], mapper: F) -> Vec<String>
        where
            F: Fn(&CommandSummary) -> &str,
        {
            commands
                .iter()
                .filter(|uc| uc.status == CommandStatus::Applied)
                .map(mapper)
                .map(|s| s.to_string())
                .collect()
        }

        // Collect accounts from different sources
        let fee_transfer_via_coinbase_accounts: Vec<String> = self
            .fee_transfer_via_coinbase
            .as_ref()
            .map(|fts| fts.iter().map(|ft| ft.receiver.to_string()).collect())
            .unwrap_or_default();
        let snark_accounts: Vec<String> = self.snark_work.iter().map(|s| s.prover.to_string()).collect();
        let fee_transfer_accounts: Vec<String> = self.fee_transfers.iter().map(|ft| ft.recipient.to_string()).collect();
        let user_command_sender_accounts = extract_accounts(&self.user_commands, |uc| &uc.sender);
        let user_command_receiver_accounts = extract_accounts(&self.user_commands, |uc| &uc.receiver);
        let user_command_fee_payer_accounts = extract_accounts(&self.user_commands, |uc| &uc.fee_payer);

        // Combine all accounts into a single collection
        let accounts: Vec<String> = [
            snark_accounts,
            fee_transfer_accounts,
            user_command_sender_accounts,
            user_command_receiver_accounts,
            user_command_fee_payer_accounts,
            fee_transfer_via_coinbase_accounts,
            vec![self.coinbase_receiver.to_string()],
        ]
        .into_iter()
        .flatten()
        .collect();

        let unique: HashSet<_> = accounts.into_iter().collect();
        unique.into_iter().collect()
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CanonicalMainnetBlockPayload {
    pub block: MainnetBlockPayload, // Composition
    pub canonical: bool,
    pub was_canonical: bool,
}

impl CanonicalMainnetBlockPayload {
    pub fn valid_accounts(&self) -> Vec<String> {
        self.block.valid_accounts() // Delegate to the mainnet_block method
    }
}

impl CanonicalItem for CanonicalMainnetBlockPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.block.state_hash
    }

    fn get_height(&self) -> u64 {
        self.block.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CanonicalBerkeleyBlockPayload {
    pub block: BerkeleyBlockPayload, // Composition
    pub canonical: bool,
    pub was_canonical: bool,
}

impl CanonicalBerkeleyBlockPayload {
    pub fn valid_accounts(&self) -> Vec<String> {
        todo!("Not implemented yet")
    }
}

impl CanonicalItem for CanonicalBerkeleyBlockPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.block.state_hash
    }

    fn get_height(&self) -> u64 {
        self.block.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical
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
    pub coinbase_receiver: String,
    pub coinbase_reward: u64,
    pub global_slot_since_genesis: u64,
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
            coinbase_receiver: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".to_string(),
            coinbase_reward: MAINNET_COINBASE_REWARD * 2,
            global_slot_since_genesis: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnarkWorkSummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub prover: String,
    pub fee_nanomina: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnarkCanonicitySummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub prover: String,
    pub fee_nanomina: u64,
    pub canonical: bool,
}

impl CanonicalItem for SnarkCanonicitySummaryPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, _was_canonical: bool) {
        // noop
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snark {
    pub prover: String,
    pub fee_nanomina: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchSnarkCanonicityPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub canonical: bool,
    pub snarks: Vec<Snark>,
}

impl CanonicalItem for BatchSnarkCanonicityPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, _was_canonical: bool) {
        // noop
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlockLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub user_command_count: usize,
    pub snark_work_count: usize,
    pub zk_app_command_count: usize,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub last_vrf_output: String,
    pub is_berkeley_block: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CanonicalBlockLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub user_command_count: usize,
    pub snark_work_count: usize,
    pub zk_app_command_count: usize,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub last_vrf_output: String,
    pub is_berkeley_block: bool,
    pub canonical: bool,
}

impl CanonicalItem for CanonicalBlockLogPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, _was_canonical: bool) {
        // noop
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub txn_hash: String,
    pub timestamp: u64,
    pub txn_type: CommandType,
    pub status: CommandStatus,
    pub sender: String,
    pub receiver: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub fee_payer: String,
    pub amount_nanomina: u64,
    pub global_slot: u64,
    pub memo: String,
}

#[derive(Serialize, Deserialize)]
pub struct BatchZkappCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub global_slot: u64,
    pub commands: Vec<ZkAppCommandSummary>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub enum InternalCommandType {
    Coinbase,
    FeeTransferViaCoinbase,
    FeeTransfer,
}

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InternalCommandLogPayload {
    pub internal_command_type: InternalCommandType,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub amount_nanomina: u64,
    pub recipient: String,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CanonicalInternalCommandLogPayload {
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

impl CanonicalItem for CanonicalInternalCommandLogPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical;
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CanonicalUserCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub txn_hash: String,
    pub timestamp: u64,
    pub txn_type: CommandType,
    pub status: CommandStatus,
    pub sender: String,
    pub receiver: String,
    pub memo: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub fee_payer: String,
    pub amount_nanomina: u64,
    pub canonical: bool,
    pub was_canonical: bool,
    pub global_slot: u64,
}

impl CanonicalItem for CanonicalUserCommandLogPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CanonicalBatchZkappCommandLogPayload {
    pub canonical: bool,
    pub was_canonical: bool,
    pub height: u64,
    pub state_hash: String,
    pub timestamp: u64,
    pub global_slot: u64,
    pub commands: Vec<ZkAppCommandSummary>,
}

impl CanonicalItem for CanonicalBatchZkappCommandLogPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical;
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BatchCanonicalUserCommandLogPayload {
    pub height: u64,
    pub state_hash: String,
    pub canonical: bool,
    pub was_canonical: bool,
    pub global_slot: u64,
    pub commands: Vec<CommandSummary>,
    pub timestamp: u64,
}

impl CanonicalItem for BatchCanonicalUserCommandLogPayload {
    fn set_canonical(&mut self, canonical: bool) {
        self.canonical = canonical;
    }

    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }

    fn get_height(&self) -> u64 {
        self.height
    }

    fn set_was_canonical(&mut self, was_canonical: bool) {
        self.was_canonical = was_canonical;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LedgerDestination {
    StakingLedger,
    BlockchainLedger,
    TokenLedger,
}

impl fmt::Display for LedgerDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_text = match self {
            LedgerDestination::BlockchainLedger => "BlockchainLedger",
            LedgerDestination::StakingLedger => "StakingLedger",
            LedgerDestination::TokenLedger => "TokenLedger",
        };
        write!(f, "{}", display_text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleEntryRecordPayload {
    pub height: u64,
    pub state_hash: String,
    pub ledger_destination: LedgerDestination,
    pub token_id: String,
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
    pub transfer_type: String,
    pub counterparty: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalanceDeltaPayload {
    pub balance_deltas: HashMap<String, i64>,
}
