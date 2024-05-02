pub mod completed_work;

use crate::{ledger::public_key::PublicKey, mina_blocks::common::*};
use completed_work::CompletedWork;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    pub completed_works: Vec<CompletedWork>,
    pub commands: Vec<Command>,
    pub coinbase: Coinbase,
    pub internal_command_statuses: Vec<InternalCommandStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalCommandStatus(pub (StatusKind,));

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Coinbase {
    Zero((CoinbaseKind,)),
    One(CoinbaseKind, Option<CoinbasePayload>),
    Two(
        CoinbaseKind,
        Option<CoinbasePayload>,
        Option<CoinbasePayload>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoinbaseKind {
    Zero,
    One,
    Two,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoinbasePayload {
    #[serde(deserialize_with = "from_str")]
    pub receiver_pk: PublicKey,

    #[serde(deserialize_with = "from_decimal_str")]
    pub fee: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub data: (CommandKind, CommandData),
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Status {
    Status((StatusKind,)),
    StatusAndFailure(StatusKind, (((FailureReason,),),)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandKind {
    #[serde(rename = "Signed_command")]
    SignedCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandData {
    #[serde(deserialize_with = "from_str")]
    pub signer: PublicKey,

    pub payload: Payload,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Payload {
    pub common: PayloadCommon,
    pub body: (PayloadKind, PayloadBody),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PayloadKind {
    Payment,

    #[serde(rename = "Stake_delegation")]
    StakeDelegation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayloadBody {
    Payment(PaymentPayload),
    StakeDelegation(StakeDelegationPayload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentPayload {
    #[serde(deserialize_with = "from_str")]
    pub receiver_pk: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StakeDelegationPayload {
    #[serde(deserialize_with = "from_str")]
    pub new_delegate: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadCommon {
    #[serde(deserialize_with = "from_decimal_str")]
    pub fee: u64,

    #[serde(deserialize_with = "from_str")]
    pub fee_payer_pk: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,

    #[serde(deserialize_with = "from_str")]
    pub valid_until: u64,

    // Base58 encoded string
    pub memo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusKind {
    Applied,
    Failed,
}

/// See https://github.com/MinaProtocol/mina/blob/berkeley/src/lib/mina_base/transaction_status.ml

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureReason {
    Predicate,

    #[serde(rename = "Source_not_present")]
    SourceNotPresent,

    #[serde(rename = "Receiver_not_present")]
    ReceiverNotPresent,

    #[serde(rename = "Amount_insufficient_to_create_account")]
    AmountInsufficientToCreateAccount,

    #[serde(rename = "Cannot_pay_creation_fee_in_token")]
    CannotPayCreationFeeInToken,

    #[serde(rename = "Source_insufficient_balance")]
    SourceInsufficientBalance,

    #[serde(rename = "Source_minimum_balance_violation")]
    SourceMinimumBalanceViolation,

    #[serde(rename = "Receiver_already_exists")]
    ReceiverAlreadyExists,

    #[serde(rename = "Token_owner_not_caller")]
    TokenOwnerNotCaller,
    Overflow,

    #[serde(rename = "Global_excess_overflow")]
    GlobalExcessOverflow,

    #[serde(rename = "Local_excess_overflow")]
    LocalExcessOverflow,

    #[serde(rename = "Local_supply_increase_overflow")]
    LocalSupplyIncreaseOverflow,

    #[serde(rename = "Global_supply_increase_overflow")]
    GlobalSupplyIncreaseOverflow,

    #[serde(rename = "Signed_command_on_zkapp_account")]
    SignedCommandOnZkappAccount,

    #[serde(rename = "Zkapp_account_not_present")]
    ZkappAccountNotPresent,

    #[serde(rename = "Update_not_permitted_balance")]
    UpdateNotPermittedBalance,

    #[serde(rename = "Update_not_permitted_access")]
    UpdateNotPermittedAccess,

    #[serde(rename = "Update_not_permitted_timing")]
    UpdateNotPermittedTiming,

    #[serde(rename = "Update_not_permitted_delegate")]
    UpdateNotPermittedDelegate,

    #[serde(rename = "Update_not_permitted_app_state")]
    UpdateNotPermittedAppState,

    #[serde(rename = "Update_not_permitted_verification_key")]
    UpdateNotPermittedVerificationKey,

    #[serde(rename = "Update_not_permitted_action_state")]
    UpdateNotPermittedactionState,

    #[serde(rename = "Update_not_permitted_zkapp_uri")]
    UpdateNotPermittedZkappUri,

    #[serde(rename = "Update_not_permitted_token_symbol")]
    UpdateNotPermittedTokenSymbol,

    #[serde(rename = "Update_not_permitted_permissions")]
    UpdateNotPermittedpermissions,

    #[serde(rename = "Update_not_permitted_nonce")]
    UpdateNotPermittedNonce,

    #[serde(rename = "Update_not_permitted_voting_for")]
    UpdateNotPermittedVotingFor,

    #[serde(rename = "Zkapp_command_replay_check_failed")]
    ZkappCommandReplayCheckFailed,

    #[serde(rename = "Fee_payer_nonce_must_increase")]
    FeePayerNonceMustIncrease,

    #[serde(rename = "Fee_payer_must_be_signed")]
    FeePayerMustBeSigned,

    #[serde(rename = "Account_balance_precondition_unsatisfied")]
    AccountBalancePreconditionUnsatisfied,

    #[serde(rename = "Account_nonce_precondition_unsatisfied")]
    AccountNoncePreconditionUnsatisfied,

    #[serde(rename = "Account_receipt_chain_hash_precondition_unsatisfied")]
    AccountReceiptChainHashPreconditionUnsatisfied,

    #[serde(rename = "Account_delegate_precondition_unsatisfied")]
    AccountDelegatePreconditionUnsatisfied,

    #[serde(rename = "Account_action_state_precondition_unsatisfied")]
    AccountActionStatePreconditionUnsatisfied,

    #[serde(rename = "Account_app_state_precondition_unsatisfied")]
    AccountAppStatePreconditionUnsatisfied(i64),

    #[serde(rename = "Account_proved_state_precondition_unsatisfied")]
    AccountProvedStatePreconditionUnsatisfied,

    #[serde(rename = "Account_is_new_precondition_unsatisfied")]
    AccountIsNewPreconditionUnsatisfied,

    #[serde(rename = "Protocol_state_precondition_unsatisfied")]
    ProtocolStatePreconditionUnsatisfied,

    #[serde(rename = "Unexpected_verification_key_hash")]
    UnexpectedVerificationKeyHash,

    #[serde(rename = "Valid_while_precondition_unsatisfied")]
    ValidWhilePreconditionUnsatisfied,

    #[serde(rename = "Incorrect_nonce")]
    IncorrectNonce,

    #[serde(rename = "Invalid_fee_excess")]
    InvalidFeeExcess,
    Cancelled,
}
