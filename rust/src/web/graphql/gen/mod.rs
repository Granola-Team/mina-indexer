use super::{DateTime, Long};
use async_graphql::*;

#[derive(InputObject)]
pub struct TransactionSourceQueryInput {
    pub and: Option<Vec<TransactionSourceQueryInput>>,
    pub or: Option<Vec<TransactionSourceQueryInput>>,
    pub public_key: Option<String>,
    // pub public_key_ne: Option<String>,
    // pub public_key_in: Option<Vec<Option<String>>>,
    // pub public_key_nin: Option<Vec<Option<String>>>,
}

#[derive(InputObject)]
pub struct TransactionFromAccountQueryInput {
    pub and: Option<Vec<TransactionFromAccountQueryInput>>,
    pub or: Option<Vec<TransactionFromAccountQueryInput>>,
    pub token: Option<u64>,
    // pub token_ne: Option<u64>,
    // pub token_nin: Option<Vec<Option<u64>>>,
    // pub token_in: Option<Vec<Option<u64>>>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandReceiverQueryInput {
    pub and: Option<Vec<BlockTransactionUserCommandReceiverQueryInput>>,
    pub or: Option<Vec<BlockTransactionUserCommandReceiverQueryInput>>,
    pub public_key: Option<String>,
    // pub public_key_ne: Option<String>,
    // pub public_key_in: Option<Vec<Option<String>>>,
    // pub public_key_nin: Option<Vec<Option<String>>>,
}

#[derive(InputObject)]
pub struct BlockTransactionFeeTransferQueryInput {
    pub type_ne: Option<String>,
    pub recipient: Option<String>,
    pub recipient_nin: Option<Vec<Option<String>>>,
    pub recipient_gt: Option<String>,
    pub type_lt: Option<String>,
    pub fee_gt: Option<Long>,
    pub fee_nin: Option<Vec<Option<Long>>>,
    pub fee_lte: Option<Long>,
    pub type_gte: Option<String>,
    pub recipient_gte: Option<String>,
    pub fee_exists: Option<bool>,
    pub or: Option<Vec<BlockTransactionFeeTransferQueryInput>>,
    pub type_gt: Option<String>,
    pub fee_in: Option<Vec<Option<Long>>>,
    pub r#type: Option<String>,
    pub fee_gte: Option<Long>,
    pub type_in: Option<Vec<Option<String>>>,
    pub recipient_lt: Option<String>,
    pub recipient_lte: Option<String>,
    pub type_lte: Option<String>,
    pub fee: Option<Long>,
    pub recipient_in: Option<Vec<Option<String>>>,
    pub recipient_ne: Option<String>,
    pub type_nin: Option<Vec<Option<String>>>,
    pub fee_lt: Option<Long>,
    pub fee_ne: Option<Long>,
    pub type_exists: Option<bool>,
    pub recipient_exists: Option<bool>,
    pub and: Option<Vec<BlockTransactionFeeTransferQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandQueryInput {
    pub id_lte: Option<String>,
    pub date_time_ne: Option<DateTime>,
    pub kind_in: Option<Vec<Option<String>>>,
    pub hash_exists: Option<bool>,
    pub fee_token_nin: Option<Vec<Option<i64>>>,
    pub failure_reason_nin: Option<Vec<Option<String>>>,
    pub kind_gte: Option<String>,
    pub token_gt: Option<i64>,
    pub id_in: Option<Vec<Option<String>>>,
    pub memo_nin: Option<Vec<Option<String>>>,
    pub to: Option<String>,
    pub fee_token_ne: Option<i64>,
    pub fee_token_gt: Option<i64>,
    pub kind_gt: Option<String>,
    pub or: Option<Vec<BlockTransactionUserCommandQueryInput>>,
    pub fee_token_lte: Option<i64>,
    pub token_exists: Option<bool>,
    pub receiver: Option<BlockTransactionUserCommandReceiverQueryInput>,
    pub amount_lt: Option<f64>,
    pub token_ne: Option<i64>,
    pub failure_reason: Option<String>,
    pub kind_lt: Option<String>,
    pub nonce: Option<i64>,
    pub to_gte: Option<String>,
    pub kind_ne: Option<String>,
    pub failure_reason_gt: Option<String>,
    pub token_gte: Option<i64>,
    pub to_lte: Option<String>,
    pub fee_token_gte: Option<i64>,
    pub date_time: Option<DateTime>,
    pub amount: Option<f64>,
    pub hash_gt: Option<String>,
    pub memo_lt: Option<String>,
    pub token_lte: Option<i64>,
    pub token: Option<i64>,
    pub block_state_hash_gte: Option<String>,
    pub amount_nin: Option<Vec<Option<f64>>>,
    pub failure_reason_ne: Option<String>,
    pub and: Option<Vec<BlockTransactionUserCommandQueryInput>>,
    pub block_state_hash_ne: Option<String>,
    pub source: Option<BlockTransactionUserCommandSourceQueryInput>,
    pub block_height_gte: Option<i64>,
    pub block_state_hash_lte: Option<String>,
    pub memo: Option<String>,
    pub to_exists: Option<bool>,
    pub from_ne: Option<String>,
    pub date_time_lt: Option<DateTime>,
    pub fee_lte: Option<f64>,
    pub from_in: Option<Vec<Option<String>>>,
    pub nonce_gte: Option<i64>,
    pub fee_token_in: Option<Vec<Option<i64>>>,
    pub nonce_nin: Option<Vec<Option<i64>>>,
    pub amount_lte: Option<f64>,
    pub fee_gt: Option<f64>,
    pub id_ne: Option<String>,
    pub from_gte: Option<String>,
    pub hash_gte: Option<String>,
    pub fee_payer: Option<BlockTransactionUserCommandFeePayerQueryInput>,
    pub nonce_lte: Option<i64>,
    pub nonce_in: Option<Vec<Option<i64>>>,
    pub date_time_exists: Option<bool>,
    pub failure_reason_lte: Option<String>,
    pub memo_lte: Option<String>,
    pub fee_payer_exists: Option<bool>,
    pub kind_nin: Option<Vec<Option<String>>>,
    pub token_lt: Option<i64>,
    pub is_delegation: Option<bool>,
    pub fee_lt: Option<f64>,
    pub memo_gt: Option<String>,
    pub from_account: Option<BlockTransactionUserCommandFromAccountQueryInput>,
    pub from: Option<String>,
    pub block_state_hash_lt: Option<String>,
    pub amount_ne: Option<f64>,
    pub block_state_hash_exists: Option<bool>,
    pub kind: Option<String>,
    pub fee_token_lt: Option<i64>,
    pub date_time_in: Option<Vec<Option<DateTime>>>,
    pub amount_in: Option<Vec<Option<f64>>>,
    pub date_time_lte: Option<DateTime>,
    pub nonce_exists: Option<bool>,
    pub date_time_gte: Option<DateTime>,
    pub block_height_gt: Option<i64>,
    pub fee_ne: Option<f64>,
    pub fee_nin: Option<Vec<Option<f64>>>,
    pub from_lte: Option<String>,
    pub fee_token_exists: Option<bool>,
    pub kind_exists: Option<bool>,
    pub date_time_nin: Option<Vec<Option<DateTime>>>,
    pub nonce_lt: Option<i64>,
    pub from_gt: Option<String>,
    pub fee_gte: Option<f64>,
    pub block_state_hash_gt: Option<String>,
    pub memo_gte: Option<String>,
    pub fee_in: Option<Vec<Option<f64>>>,
    pub nonce_ne: Option<i64>,
    pub block_height_ne: Option<i64>,
    pub date_time_gt: Option<DateTime>,
    pub block_height_in: Option<Vec<Option<i64>>>,
    pub block_height_nin: Option<Vec<Option<i64>>>,
    pub to_account_exists: Option<bool>,
    pub block_height: Option<i64>,
    pub fee: Option<f64>,
    pub block_state_hash_in: Option<Vec<Option<String>>>,
    pub from_account_exists: Option<bool>,
    pub to_nin: Option<Vec<Option<String>>>,
    pub kind_lte: Option<String>,
    pub failure_reason_gte: Option<String>,
    pub to_lt: Option<String>,
    pub amount_gt: Option<f64>,
    pub memo_exists: Option<bool>,
    pub amount_gte: Option<f64>,
    pub is_delegation_ne: Option<bool>,
    pub block_state_hash_nin: Option<Vec<Option<String>>>,
    pub to_gt: Option<String>,
    pub failure_reason_exists: Option<bool>,
    pub id_gt: Option<String>,
    pub from_exists: Option<bool>,
    pub fee_exists: Option<bool>,
    pub token_in: Option<Vec<Option<i64>>>,
    pub receiver_exists: Option<bool>,
    pub hash_ne: Option<String>,
    pub failure_reason_in: Option<Vec<Option<String>>>,
    pub hash_lte: Option<String>,
    pub to_ne: Option<String>,
    pub from_nin: Option<Vec<Option<String>>>,
    pub to_account: Option<BlockTransactionUserCommandToAccountQueryInput>,
    pub id: Option<String>,
    pub nonce_gt: Option<i64>,
    pub hash: Option<String>,
    pub block_height_lte: Option<i64>,
    pub block_height_exists: Option<bool>,
    pub is_delegation_exists: Option<bool>,
    pub id_nin: Option<Vec<Option<String>>>,
    pub fee_token: Option<i64>,
    pub to_in: Option<Vec<Option<String>>>,
    pub memo_ne: Option<String>,
    pub amount_exists: Option<bool>,
    pub id_lt: Option<String>,
    pub from_lt: Option<String>,
    pub block_height_lt: Option<i64>,
    pub memo_in: Option<Vec<Option<String>>>,
    pub token_nin: Option<Vec<Option<i64>>>,
    pub hash_nin: Option<Vec<Option<String>>>,
    pub source_exists: Option<bool>,
    pub block_state_hash: Option<String>,
    pub hash_lt: Option<String>,
    pub id_gte: Option<String>,
    pub id_exists: Option<bool>,
    pub hash_in: Option<Vec<Option<String>>>,
    pub failure_reason_lt: Option<String>,
}

#[derive(InputObject)]
pub struct BlockTransactionQueryInput {
    pub user_commands_exists: Option<bool>,
    pub user_commands: Option<Vec<Option<BlockTransactionUserCommandQueryInput>>>,
    pub coinbase_gte: Option<Long>,
    pub coinbase_receiver_account: Option<BlockTransactionCoinbaseReceiverAccountQueryInput>,
    pub fee_transfer: Option<Vec<Option<BlockTransactionFeeTransferQueryInput>>>,
    pub coinbase_nin: Option<Vec<Option<Long>>>,
    pub coinbase_lt: Option<Long>,
    pub fee_transfer_exists: Option<bool>,
    pub user_commands_in: Option<Vec<Option<BlockTransactionUserCommandQueryInput>>>,
    pub fee_transfer_in: Option<Vec<Option<BlockTransactionFeeTransferQueryInput>>>,
    pub user_commands_nin: Option<Vec<Option<BlockTransactionUserCommandQueryInput>>>,
    pub and: Option<Vec<BlockTransactionQueryInput>>,
    pub coinbase_gt: Option<Long>,
    pub coinbase_in: Option<Vec<Option<Long>>>,
    pub coinbase: Option<Long>,
    pub coinbase_ne: Option<Long>,
    pub coinbase_receiver_account_exists: Option<bool>,
    pub coinbase_exists: Option<bool>,
    pub coinbase_lte: Option<Long>,
    pub fee_transfer_nin: Option<Vec<Option<BlockTransactionFeeTransferQueryInput>>>,
    pub or: Option<Vec<BlockTransactionQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockWinnerAccountBalanceQueryInput {
    pub block_height_in: Option<Vec<Option<i64>>>,
    pub state_hash_gte: Option<String>,
    pub liquid_lt: Option<i64>,
    pub total_gt: Option<Long>,
    pub liquid_exists: Option<bool>,
    pub unknown_in: Option<Vec<Option<Long>>>,
    pub liquid_in: Option<Vec<Option<i64>>>,
    pub total_gte: Option<Long>,
    pub state_hash_gt: Option<String>,
    pub state_hash_lt: Option<String>,
    pub liquid_ne: Option<i64>,
    pub locked_lte: Option<Long>,
    pub unknown_gt: Option<Long>,
    pub total_nin: Option<Vec<Option<Long>>>,
    pub locked_exists: Option<bool>,
    pub total_lte: Option<Long>,
    pub unknown_nin: Option<Vec<Option<Long>>>,
    pub total_lt: Option<Long>,
    pub block_height_lte: Option<i64>,
    pub state_hash_in: Option<Vec<Option<String>>>,
    pub state_hash: Option<String>,
    pub block_height_exists: Option<bool>,
    pub unknown_lt: Option<Long>,
    pub liquid_gte: Option<i64>,
    pub locked_ne: Option<Long>,
    pub state_hash_lte: Option<String>,
    pub unknown_lte: Option<Long>,
    pub liquid_gt: Option<i64>,
    pub block_height_lt: Option<i64>,
    pub locked_in: Option<Vec<Option<Long>>>,
    pub locked_nin: Option<Vec<Option<Long>>>,
    pub block_height_gte: Option<i64>,
    pub locked_lt: Option<Long>,
    pub and: Option<Vec<BlockWinnerAccountBalanceQueryInput>>,
    pub unknown_ne: Option<Long>,
    pub total_exists: Option<bool>,
    pub liquid_lte: Option<i64>,
    pub liquid: Option<i64>,
    pub state_hash_nin: Option<Vec<Option<String>>>,
    pub block_height: Option<i64>,
    pub locked_gt: Option<Long>,
    pub or: Option<Vec<BlockWinnerAccountBalanceQueryInput>>,
    pub locked_gte: Option<Long>,
    pub total_ne: Option<Long>,
    pub state_hash_exists: Option<bool>,
    pub block_height_gt: Option<i64>,
    pub block_height_ne: Option<i64>,
    pub state_hash_ne: Option<String>,
    pub unknown: Option<Long>,
    pub liquid_nin: Option<Vec<Option<i64>>>,
    pub locked: Option<Long>,
    pub block_height_nin: Option<Vec<Option<i64>>>,
    pub total_in: Option<Vec<Option<Long>>>,
    pub unknown_gte: Option<Long>,
    pub total: Option<Long>,
    pub unknown_exists: Option<bool>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandFromAccountQueryInput {
    pub token_in: Option<Vec<Option<i64>>>,
    pub token_nin: Option<Vec<Option<i64>>>,
    pub token_lt: Option<i64>,
    pub token_lte: Option<i64>,
    pub token_gt: Option<i64>,
    pub token: Option<i64>,
    pub or: Option<Vec<BlockTransactionUserCommandFromAccountQueryInput>>,
    pub token_exists: Option<bool>,
    pub token_ne: Option<i64>,
    pub token_gte: Option<i64>,
    pub and: Option<Vec<BlockTransactionUserCommandFromAccountQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateBlockchainStateQueryInput {
    pub staged_ledger_hash: Option<String>,
    pub snarked_ledger_hash_lt: Option<String>,
    pub staged_ledger_hash_lt: Option<String>,
    pub date_lte: Option<Long>,
    pub snarked_ledger_hash_lte: Option<String>,
    pub utc_date_exists: Option<bool>,
    pub staged_ledger_hash_gt: Option<String>,
    pub utc_date_lte: Option<Long>,
    pub snarked_ledger_hash_gte: Option<String>,
    pub snarked_ledger_hash_exists: Option<bool>,
    pub utc_date_gte: Option<Long>,
    pub date_ne: Option<Long>,
    pub staged_ledger_hash_in: Option<Vec<Option<String>>>,
    pub and: Option<Vec<BlockProtocolStateBlockchainStateQueryInput>>,
    pub staged_ledger_hash_ne: Option<String>,
    pub utc_date_ne: Option<Long>,
    pub snarked_ledger_hash_in: Option<Vec<Option<String>>>,
    pub date_gte: Option<Long>,
    pub snarked_ledger_hash: Option<String>,
    pub staged_ledger_hash_gte: Option<String>,
    pub utc_date: Option<Long>,
    pub utc_date_nin: Option<Vec<Option<Long>>>,
    pub staged_ledger_hash_nin: Option<Vec<Option<String>>>,
    pub snarked_ledger_hash_ne: Option<String>,
    pub staged_ledger_hash_exists: Option<bool>,
    pub snarked_ledger_hash_nin: Option<Vec<Option<String>>>,
    pub date_gt: Option<Long>,
    pub utc_date_gt: Option<Long>,
    pub date_lt: Option<Long>,
    pub staged_ledger_hash_lte: Option<String>,
    pub or: Option<Vec<BlockProtocolStateBlockchainStateQueryInput>>,
    pub date: Option<Long>,
    pub snarked_ledger_hash_gt: Option<String>,
    pub date_nin: Option<Vec<Option<Long>>>,
    pub date_in: Option<Vec<Option<Long>>>,
    pub date_exists: Option<bool>,
    pub utc_date_lt: Option<Long>,
    pub utc_date_in: Option<Vec<Option<Long>>>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateConsensusStateStakingEpochDatumQueryInput {
    pub start_checkpoint_gt: Option<String>,
    pub epoch_length_lte: Option<i64>,
    pub lock_checkpoint_ne: Option<String>,
    pub lock_checkpoint_gte: Option<String>,
    pub seed: Option<String>,
    pub start_checkpoint_ne: Option<String>,
    pub epoch_length_exists: Option<bool>,
    pub lock_checkpoint_lt: Option<String>,
    pub start_checkpoint_lt: Option<String>,
    pub epoch_length_gte: Option<i64>,
    pub epoch_length_lt: Option<i64>,
    pub lock_checkpoint_gt: Option<String>,
    pub seed_nin: Option<Vec<Option<String>>>,
    pub epoch_length_ne: Option<i64>,
    pub epoch_length_gt: Option<i64>,
    pub epoch_length_in: Option<Vec<Option<i64>>>,
    pub start_checkpoint_gte: Option<String>,
    pub or: Option<Vec<BlockProtocolStateConsensusStateStakingEpochDatumQueryInput>>,
    pub lock_checkpoint_in: Option<Vec<Option<String>>>,
    pub start_checkpoint_in: Option<Vec<Option<String>>>,
    pub ledger_exists: Option<bool>,
    pub seed_in: Option<Vec<Option<String>>>,
    pub lock_checkpoint_exists: Option<bool>,
    pub ledger: Option<BlockProtocolStateConsensusStateStakingEpochDatumLedgerQueryInput>,
    pub lock_checkpoint_nin: Option<Vec<Option<String>>>,
    pub start_checkpoint_exists: Option<bool>,
    pub seed_exists: Option<bool>,
    pub lock_checkpoint_lte: Option<String>,
    pub and: Option<Vec<BlockProtocolStateConsensusStateStakingEpochDatumQueryInput>>,
    pub start_checkpoint: Option<String>,
    pub start_checkpoint_lte: Option<String>,
    pub seed_lte: Option<String>,
    pub seed_lt: Option<String>,
    pub seed_gt: Option<String>,
    pub epoch_length: Option<u32>,
    pub seed_gte: Option<String>,
    pub epoch_length_nin: Option<Vec<Option<u32>>>,
    pub lock_checkpoint: Option<String>,
    pub seed_ne: Option<String>,
    pub start_checkpoint_nin: Option<Vec<Option<String>>>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandToAccountQueryInput {
    pub token_in: Option<Vec<Option<i64>>>,
    pub token_lte: Option<i64>,
    pub token_exists: Option<bool>,
    pub and: Option<Vec<BlockTransactionUserCommandToAccountQueryInput>>,
    pub token_gt: Option<i64>,
    pub token_ne: Option<i64>,
    pub token_gte: Option<i64>,
    pub token: Option<i64>,
    pub token_lt: Option<i64>,
    pub or: Option<Vec<BlockTransactionUserCommandToAccountQueryInput>>,
    pub token_nin: Option<Vec<Option<i64>>>,
}

#[derive(InputObject)]
pub struct BlockCreatorAccountQueryInput {
    pub or: Option<Vec<BlockCreatorAccountQueryInput>>,
    pub and: Option<Vec<BlockCreatorAccountQueryInput>>,
    pub public_key: Option<String>,
}

#[derive(InputObject)]
pub struct BlockCoinbaseReceiverQueryInput {
    pub or: Option<Vec<BlockCoinbaseReceiverQueryInput>>,
    pub and: Option<Vec<BlockCoinbaseReceiverQueryInput>>,
    #[graphql(name = "public_key")]
    pub public_key: Option<String>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateConsensusStateNextEpochDatumQueryInput {
    pub lock_checkpoint: Option<String>,
    pub seed_gte: Option<String>,
    pub ledger: Option<BlockProtocolStateConsensusStateNextEpochDatumLedgerQueryInput>,
    pub epoch_length_ne: Option<i64>,
    pub lock_checkpoint_nin: Option<Vec<Option<String>>>,
    pub seed_exists: Option<bool>,
    pub lock_checkpoint_lt: Option<String>,
    pub seed_nin: Option<Vec<Option<String>>>,
    pub lock_checkpoint_exists: Option<bool>,
    pub seed: Option<String>,
    pub lock_checkpoint_in: Option<Vec<Option<String>>>,
    pub start_checkpoint_nin: Option<Vec<Option<String>>>,
    pub lock_checkpoint_ne: Option<String>,
    pub epoch_length: Option<i64>,
    pub start_checkpoint_lte: Option<String>,
    pub and: Option<Vec<BlockProtocolStateConsensusStateNextEpochDatumQueryInput>>,
    pub seed_ne: Option<String>,
    pub seed_lt: Option<String>,
    pub lock_checkpoint_lte: Option<String>,
    pub ledger_exists: Option<bool>,
    pub start_checkpoint_exists: Option<bool>,
    pub epoch_length_exists: Option<bool>,
    pub lock_checkpoint_gt: Option<String>,
    pub epoch_length_lte: Option<i64>,
    pub seed_in: Option<Vec<Option<String>>>,
    pub lock_checkpoint_gte: Option<String>,
    pub start_checkpoint_ne: Option<String>,
    pub epoch_length_gte: Option<i64>,
    pub start_checkpoint_gte: Option<String>,
    pub start_checkpoint_in: Option<Vec<Option<String>>>,
    pub seed_lte: Option<String>,
    pub start_checkpoint: Option<String>,
    pub start_checkpoint_lt: Option<String>,
    pub epoch_length_gt: Option<i64>,
    pub start_checkpoint_gt: Option<String>,
    pub epoch_length_lt: Option<i64>,
    pub seed_gt: Option<String>,
    pub or: Option<Vec<BlockProtocolStateConsensusStateNextEpochDatumQueryInput>>,
    pub epoch_length_nin: Option<Vec<Option<i64>>>,
    pub epoch_length_in: Option<Vec<Option<i64>>>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateConsensusStateNextEpochDatumLedgerQueryInput {
    pub hash_gt: Option<String>,
    pub total_currency_gt: Option<f64>,
    pub hash: Option<String>,
    pub total_currency_lt: Option<f64>,
    pub total_currency_exists: Option<bool>,
    pub hash_exists: Option<bool>,
    pub hash_ne: Option<String>,
    pub total_currency_in: Option<Vec<Option<f64>>>,
    pub total_currency_nin: Option<Vec<Option<f64>>>,
    pub total_currency_lte: Option<f64>,
    pub total_currency_ne: Option<f64>,
    pub hash_in: Option<Vec<Option<String>>>,
    pub and: Option<Vec<BlockProtocolStateConsensusStateNextEpochDatumLedgerQueryInput>>,
    pub total_currency_gte: Option<f64>,
    pub hash_lt: Option<String>,
    pub hash_gte: Option<String>,
    pub hash_lte: Option<String>,
    pub or: Option<Vec<BlockProtocolStateConsensusStateNextEpochDatumLedgerQueryInput>>,
    pub hash_nin: Option<Vec<Option<String>>>,
    pub total_currency: Option<f64>,
}

#[derive(InputObject)]
pub struct BlockSnarkJobQueryInput {
    pub prover: Option<String>,
    pub work_ids_in: Option<Vec<Option<i64>>>,
    pub fee_gt: Option<i64>,
    pub block_state_hash_exists: Option<bool>,
    pub prover_ne: Option<String>,
    pub block_height: Option<i64>,
    pub date_time: Option<DateTime>,
    pub and: Option<Vec<BlockSnarkJobQueryInput>>,
    pub block_height_lt: Option<i64>,
    pub date_time_lt: Option<DateTime>,
    pub date_time_ne: Option<DateTime>,
    pub prover_lte: Option<String>,
    pub date_time_exists: Option<bool>,
    pub fee: Option<i64>,
    pub prover_exists: Option<bool>,
    pub date_time_lte: Option<DateTime>,
    pub block_state_hash_lte: Option<String>,
    pub date_time_gte: Option<DateTime>,
    pub date_time_gt: Option<DateTime>,
    pub fee_ne: Option<i64>,
    pub fee_in: Option<Vec<Option<i64>>>,
    pub block_height_ne: Option<i64>,
    pub prover_in: Option<Vec<Option<String>>>,
    pub fee_exists: Option<bool>,
    pub work_ids_exists: Option<bool>,
    pub block_height_in: Option<Vec<Option<i64>>>,
    pub block_height_gte: Option<i64>,
    pub prover_nin: Option<Vec<Option<String>>>,
    pub fee_nin: Option<Vec<Option<i64>>>,
    pub fee_lt: Option<i64>,
    pub or: Option<Vec<BlockSnarkJobQueryInput>>,
    pub work_ids: Option<Vec<Option<i64>>>,
    pub block_height_lte: Option<i64>,
    pub block_state_hash_nin: Option<Vec<Option<String>>>,
    pub block_height_exists: Option<bool>,
    pub block_state_hash_ne: Option<String>,
    pub prover_lt: Option<String>,
    pub date_time_nin: Option<Vec<Option<DateTime>>>,
    pub block_state_hash_gt: Option<String>,
    pub block_height_gt: Option<i64>,
    pub block_state_hash_in: Option<Vec<Option<String>>>,
    pub prover_gt: Option<String>,
    pub block_state_hash_gte: Option<String>,
    pub block_state_hash_lt: Option<String>,
    pub date_time_in: Option<Vec<Option<DateTime>>>,
    pub fee_lte: Option<i64>,
    pub prover_gte: Option<String>,
    pub work_ids_nin: Option<Vec<Option<i64>>>,
    pub block_state_hash: Option<String>,
    pub fee_gte: Option<i64>,
    pub block_height_nin: Option<Vec<Option<i64>>>,
}

#[derive(InputObject)]
pub struct BlockQueryInput {
    pub creator_account: Option<BlockCreatorAccountQueryInput>,
    pub coinbase_receiver: Option<BlockCoinbaseReceiverQueryInput>,
    pub protocol_state: Option<BlockProtocolStateQueryInput>,
    pub canonical: Option<bool>,
    pub state_hash: Option<String>,
    pub block_height: Option<u32>,
    pub genesis_state_hash: Option<String>,
    pub block_stake_winner: Option<String>,

    #[graphql(name = "unique_block_producers_last_n_blocks")]
    pub unique_block_producers_last_n_blocks: Option<u32>,

    #[graphql(name = "global_slot_since_genesis")]
    pub global_slot_since_genesis: Option<u32>,

    #[graphql(name = "blockHeight_gt")]
    pub block_height_gt: Option<u32>,

    #[graphql(name = "blockHeight_gte")]
    pub block_height_gte: Option<u32>,

    #[graphql(name = "blockHeight_lt")]
    pub block_height_lt: Option<u32>,

    #[graphql(name = "blockHeight_lte")]
    pub block_height_lte: Option<u32>,

    /// Boolean or
    pub or: Option<Vec<BlockQueryInput>>,

    /// Boolean and
    pub and: Option<Vec<BlockQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateQueryInput {
    pub previous_state_hash_exists: Option<bool>,
    pub blockchain_state_exists: Option<bool>,
    pub consensus_state: Option<BlockProtocolStateConsensusStateQueryInput>,
    pub previous_state_hash_ne: Option<String>,
    pub consensus_state_exists: Option<bool>,
    pub previous_state_hash_nin: Option<Vec<Option<String>>>,
    pub previous_state_hash_lt: Option<String>,
    pub or: Option<Vec<BlockProtocolStateQueryInput>>,
    pub previous_state_hash_lte: Option<String>,
    pub blockchain_state: Option<BlockProtocolStateBlockchainStateQueryInput>,
    pub previous_state_hash_gte: Option<String>,
    pub previous_state_hash_gt: Option<String>,
    pub previous_state_hash_in: Option<Vec<Option<String>>>,
    pub and: Option<Vec<BlockProtocolStateQueryInput>>,
    pub previous_state_hash: Option<String>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateConsensusStateStakingEpochDatumLedgerQueryInput {
    pub hash_in: Option<Vec<Option<String>>>,
    pub hash_exists: Option<bool>,
    pub total_currency: Option<f64>,
    pub hash_gte: Option<String>,
    pub total_currency_lt: Option<f64>,
    pub total_currency_nin: Option<Vec<Option<f64>>>,
    pub hash: Option<String>,
    pub hash_lte: Option<String>,
    pub total_currency_exists: Option<bool>,
    pub total_currency_gt: Option<f64>,
    pub or: Option<Vec<BlockProtocolStateConsensusStateStakingEpochDatumLedgerQueryInput>>,
    pub hash_gt: Option<String>,
    pub and: Option<Vec<BlockProtocolStateConsensusStateStakingEpochDatumLedgerQueryInput>>,
    pub hash_ne: Option<String>,
    pub hash_nin: Option<Vec<Option<String>>>,
    pub total_currency_gte: Option<f64>,
    pub total_currency_in: Option<Vec<Option<f64>>>,
    pub total_currency_ne: Option<f64>,
    pub hash_lt: Option<String>,
    pub total_currency_lte: Option<f64>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandSourceQueryInput {
    pub public_key_gte: Option<String>,
    pub public_key_exists: Option<bool>,
    pub public_key_lte: Option<String>,
    pub and: Option<Vec<BlockTransactionUserCommandSourceQueryInput>>,
    pub public_key: Option<String>,
    pub public_key_nin: Option<Vec<Option<String>>>,
    pub public_key_lt: Option<String>,
    pub public_key_in: Option<Vec<Option<String>>>,
    pub or: Option<Vec<BlockTransactionUserCommandSourceQueryInput>>,
    pub public_key_gt: Option<String>,
    pub public_key_ne: Option<String>,
}

#[derive(InputObject)]
pub struct BlockWinnerAccountQueryInput {
    pub balance: Option<BlockWinnerAccountBalanceQueryInput>,
    pub public_key_in: Option<Vec<Option<String>>>,
    pub public_key_ne: Option<String>,
    pub public_key_gt: Option<String>,
    pub and: Option<Vec<BlockWinnerAccountQueryInput>>,
    pub public_key_lt: Option<String>,
    pub public_key_nin: Option<Vec<Option<String>>>,
    pub or: Option<Vec<BlockWinnerAccountQueryInput>>,
    pub public_key_lte: Option<String>,
    pub public_key_exists: Option<bool>,
    pub public_key_gte: Option<String>,
    pub public_key: Option<String>,
    pub balance_exists: Option<bool>,
}

#[derive(InputObject)]
pub struct BlockTransactionUserCommandFeePayerQueryInput {
    pub token_lt: Option<i64>,
    pub token_lte: Option<i64>,
    pub token_in: Option<Vec<Option<i64>>>,
    pub token_nin: Option<Vec<Option<i64>>>,
    pub token: Option<i64>,
    pub token_ne: Option<i64>,
    pub token_exists: Option<bool>,
    pub token_gt: Option<i64>,
    pub token_gte: Option<i64>,
    pub and: Option<Vec<BlockTransactionUserCommandFeePayerQueryInput>>,
    pub or: Option<Vec<BlockTransactionUserCommandFeePayerQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockTransactionCoinbaseReceiverAccountQueryInput {
    pub public_key_gt: Option<String>,
    pub public_key_gte: Option<String>,
    pub public_key_lte: Option<String>,
    pub public_key_exists: Option<bool>,
    pub or: Option<Vec<BlockTransactionCoinbaseReceiverAccountQueryInput>>,
    pub public_key_in: Option<Vec<Option<String>>>,
    pub public_key_nin: Option<Vec<Option<String>>>,
    pub public_key_ne: Option<String>,
    pub public_key: Option<String>,
    pub public_key_lt: Option<String>,
    pub and: Option<Vec<BlockTransactionCoinbaseReceiverAccountQueryInput>>,
}

#[derive(InputObject)]
pub struct BlockProtocolStateConsensusStateQueryInput {
    // boolean
    pub or: Option<Vec<BlockProtocolStateConsensusStateQueryInput>>,
    pub and: Option<Vec<BlockProtocolStateConsensusStateQueryInput>>,

    // block height/blockchain length
    pub block_height: Option<u32>,
    pub block_height_in: Option<Vec<Option<u32>>>,
    pub block_height_nin: Option<Vec<Option<u32>>>,
    pub block_height_lte: Option<u32>,
    pub block_height_gte: Option<u32>,
    pub block_height_gt: Option<u32>,
    pub block_height_lt: Option<u32>,
    pub block_height_ne: Option<u32>,

    pub blockchain_length_in: Option<Vec<Option<u32>>>,
    pub blockchain_length_lt: Option<u32>,
    pub blockchain_length: Option<u32>,
    pub block_height_exists: Option<bool>,
    pub blockchain_length_ne: Option<u32>,
    pub blockchain_length_gt: Option<u32>,
    pub blockchain_length_lte: Option<u32>,
    pub blockchain_length_nin: Option<Vec<Option<u32>>>,
    pub blockchain_length_exists: Option<bool>,
    pub blockchain_length_gte: Option<u32>,

    // epoch
    pub epoch: Option<u32>,
    pub epoch_gte: Option<u32>,
    pub epoch_gt: Option<u32>,
    pub epoch_lte: Option<u32>,
    pub epoch_lt: Option<u32>,
    pub epoch_ne: Option<u32>,
    pub epoch_exists: Option<bool>,
    pub epoch_in: Option<Vec<Option<u32>>>,
    pub epoch_nin: Option<Vec<Option<u32>>>,

    // slot
    pub slot: Option<u32>,
    pub slot_gte: Option<u32>,
    pub slot_gt: Option<u32>,
    pub slot_lte: Option<u32>,
    pub slot_lt: Option<u32>,
    pub slot_ne: Option<u32>,
    pub slot_exists: Option<bool>,
    pub slot_in: Option<Vec<Option<u32>>>,
    pub slot_nin: Option<Vec<Option<u32>>>,

    pub slot_since_genesis: Option<u32>,
    pub slot_since_genesis_ne: Option<u32>,
    pub slot_since_genesis_exists: Option<bool>,
    pub slot_since_genesis_in: Option<Vec<Option<u32>>>,
    pub slot_since_genesis_nin: Option<Vec<Option<u32>>>,

    #[graphql(name = "slotSinceGenesis_gte")]
    pub slot_since_genesis_gte: Option<u32>,

    #[graphql(name = "slotSinceGenesis_gt")]
    pub slot_since_genesis_gt: Option<u32>,

    #[graphql(name = "slotSinceGenesis_lte")]
    pub slot_since_genesis_lte: Option<u32>,

    #[graphql(name = "slotSinceGenesis_lt")]
    pub slot_since_genesis_lt: Option<u32>,

    // total currency
    pub total_currency: Option<f64>,
    pub total_currency_gte: Option<f64>,
    pub total_currency_gt: Option<f64>,
    pub total_currency_lte: Option<f64>,
    pub total_currency_lt: Option<f64>,
    pub total_currency_ne: Option<f64>,
    pub total_currency_exists: Option<bool>,
    pub total_currency_in: Option<Vec<Option<f64>>>,
    pub total_currency_nin: Option<Vec<Option<f64>>>,

    // next epoch data
    pub next_epoch_data: Option<BlockProtocolStateConsensusStateNextEpochDatumQueryInput>,
    pub next_epoch_data_exists: Option<bool>,

    // staking epoch data
    pub staking_epoch_data: Option<BlockProtocolStateConsensusStateStakingEpochDatumQueryInput>,
    pub staking_epoch_data_exists: Option<bool>,

    // min window density
    pub min_window_density: Option<u32>,
    pub min_window_density_gte: Option<u32>,
    pub min_window_density_gt: Option<u32>,
    pub min_window_density_ne: Option<u32>,
    pub min_window_density_lt: Option<u32>,
    pub min_window_density_exists: Option<bool>,
    pub min_window_density_lte: Option<u32>,
    pub min_window_density_in: Option<Vec<Option<u32>>>,
    pub min_window_density_nin: Option<Vec<Option<u32>>>,

    // ancestor in same checkpoint window
    pub has_ancestor_in_same_checkpoint_window_exists: Option<bool>,
    pub has_ancestor_in_same_checkpoint_window: Option<bool>,
    pub has_ancestor_in_same_checkpoint_window_ne: Option<bool>,

    // last VRF output
    pub last_vrf_output: Option<String>,
    pub last_vrf_output_gte: Option<String>,
    pub last_vrf_output_gt: Option<String>,
    pub last_vrf_output_lte: Option<String>,
    pub last_vrf_output_lt: Option<String>,
    pub last_vrf_output_nin: Option<Vec<Option<String>>>,
    pub last_vrf_output_exists: Option<bool>,
    pub last_vrf_output_in: Option<Vec<Option<String>>>,
    pub last_vrf_output_ne: Option<String>,
}

#[derive(InputObject)]
pub struct TransactionQueryInput {
    // various attributes
    pub block: Option<BlockQueryInput>,
    pub hash: Option<String>,
    pub canonical: Option<bool>,
    pub kind: Option<String>,
    pub memo: Option<String>,
    pub token: Option<String>,
    pub is_delegation: Option<bool>,
    pub zkapp: Option<bool>,

    /// Failure reason only applies to failed transactions
    pub failure_reason: Option<String>,
    pub is_applied: Option<bool>,

    // sender attributes
    pub from: Option<String>,
    pub source: Option<TransactionSourceQueryInput>,
    pub from_account: Option<TransactionFromAccountQueryInput>,

    // receiver attributes
    pub to: Option<String>,
    pub receiver: Option<TransactionReceiverQueryInput>,
    pub to_account: Option<TransactionToAccountQueryInput>,

    // fee attributes
    pub fee: Option<u64>,
    pub fee_payer: Option<TransactionFeePayerQueryInput>,
    pub fee_token: Option<u64>,
    pub fee_gt: Option<u64>,
    pub fee_gte: Option<u64>,
    pub fee_lt: Option<u64>,
    pub fee_lte: Option<u64>,

    // amount attributes
    pub amount: Option<u64>,
    pub amount_gt: Option<u64>,
    pub amount_gte: Option<u64>,
    pub amount_lte: Option<u64>,
    pub amount_lt: Option<u64>,

    // block height attributes
    pub block_height: Option<u32>,

    #[graphql(name = "blockHeight_gt")]
    pub block_height_gt: Option<u32>,

    #[graphql(name = "blockHeight_gte")]
    pub block_height_gte: Option<u32>,

    #[graphql(name = "blockHeight_lt")]
    pub block_height_lt: Option<u32>,

    #[graphql(name = "blockHeight_lte")]
    pub block_height_lte: Option<u32>,

    // global slot attributes
    pub global_slot: Option<u32>,

    #[graphql(name = "globalSlot_gt")]
    pub global_slot_gt: Option<u32>,

    #[graphql(name = "globalSlot_gte")]
    pub global_slot_gte: Option<u32>,

    #[graphql(name = "globalSlot_lt")]
    pub global_slot_lt: Option<u32>,

    #[graphql(name = "globalSlot_lte")]
    pub global_slot_lte: Option<u32>,

    // datetime attributes
    pub date_time: Option<DateTime>,
    pub date_time_gt: Option<DateTime>,
    pub date_time_gte: Option<DateTime>,
    pub date_time_lt: Option<DateTime>,
    pub date_time_lte: Option<DateTime>,

    // nonce attributes
    pub nonce: Option<u32>,
    pub nonce_lte: Option<u32>,
    pub nonce_gt: Option<u32>,
    pub nonce_lt: Option<u32>,
    pub nonce_gte: Option<u32>,

    // boolean operators
    pub and: Option<Vec<TransactionQueryInput>>,
    pub or: Option<Vec<TransactionQueryInput>>,
}

#[derive(InputObject)]
pub struct TransactionReceiverQueryInput {
    pub public_key: Option<String>,
    pub and: Option<Vec<TransactionReceiverQueryInput>>,
    pub or: Option<Vec<TransactionReceiverQueryInput>>,
}

#[derive(InputObject)]
pub struct TransactionToAccountQueryInput {
    pub token: Option<u64>,
    pub and: Option<Vec<TransactionToAccountQueryInput>>,
    pub or: Option<Vec<TransactionToAccountQueryInput>>,
}

#[derive(InputObject)]
pub struct TransactionFeePayerQueryInput {
    pub token: Option<u64>,
    pub and: Option<Vec<TransactionFeePayerQueryInput>>,
    pub or: Option<Vec<TransactionFeePayerQueryInput>>,
}
