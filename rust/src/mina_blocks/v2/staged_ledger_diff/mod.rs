pub mod command;
pub mod completed_work;

use super::protocol_state::SupplyAdjustment;
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
    Two,
    One,
    Zero,
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

    #[serde(rename = "Zkapp_command")]
    ZkappCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandData {
    UserCommandData(UserCommandData),
    ZkappCommandData(ZkappCommandData),
}

/// User command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCommandData {
    #[serde(deserialize_with = "from_str")]
    pub signer: PublicKey,

    pub payload: UserCommandPayload,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCommandPayload {
    pub common: UserCommandPayloadCommon,
    pub body: (UserCommandPayloadKind, UserCommandPayloadBody),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserCommandPayloadKind {
    Payment,

    #[serde(rename = "Stake_delegation")]
    StakeDelegation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserCommandPayloadBody {
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

/// Zkapp command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkappCommandData {
    pub fee_payer: FeePayer,
    pub account_updates: Vec<AccountUpdates>,

    // base58 encoded memo
    pub memo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeePayer {
    pub body: FeePayerBody,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeePayerBody {
    #[serde(deserialize_with = "from_str")]
    pub public_key: PublicKey,

    #[serde(deserialize_with = "from_decimal_str")]
    pub fee: u64,

    #[serde(deserialize_with = "from_str_opt")]
    pub valid_until: Option<u64>,

    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdates {
    pub elt: Elt,
    pub stack_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Elt {
    pub account_update: AccountUpdate,
    pub account_update_digest: String,
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub body: AccountUpdateBody,
    pub authorization: Authorization,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthorizationKind {
    #[serde(rename = "None_given")]
    NoneGiven,

    Either,
    Proof,
    Signature,
    Impossible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authorization {
    #[serde(rename = "None_given")]
    NoneGiven,

    Proof((AuthorizationKind, String)),
    Signature((AuthorizationKind, String)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdateBody {
    #[serde(deserialize_with = "from_str")]
    pub public_key: PublicKey,

    pub token_id: String,
    pub update: Update,
    pub balance_change: SupplyAdjustment,
    pub increment_nonce: bool,
    pub events: (ZkappEvents,),
    pub actions: (ZkappActions,),
    pub call_data: String,
    pub preconditions: Preconditions,
    pub use_full_commitment: bool,
    pub implicit_account_creation_fee: bool,
    pub may_use_token: (MayUseToken,),
    pub authorization_kind: (AuthorizationKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MayUseToken {
    No,
    Yes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkappActions(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkappEvents(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Update {
    // one for each app state field element
    pub app_state: [(UpdateKind,); 8],

    pub delegate: (UpdateKind,),
    pub verification_key: (UpdateKind,),
    pub permissions: (UpdateKind,),
    pub zkapp_uri: (UpdateKind,),
    pub token_symbol: (UpdateKind,),
    pub timing: (UpdateKind,),
    pub voting_for: (UpdateKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateKind {
    Keep,
    Set(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preconditions {
    pub network: NetworkPreconditions,
    pub account: AccountPreconditions,
    pub valid_while: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkPreconditions {
    pub snarked_ledger_hash: (PreconditionKind,),
    pub blockchain_length: (PreconditionKind,),
    pub min_window_density: (PreconditionKind,),
    pub total_currency: (PreconditionKind,),
    pub global_slot_since_genesis: (PreconditionKind,),
    pub staking_epoch_data: StakingEpochDataPreconditions,
    pub next_epoch_data: StakingEpochDataPreconditions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StakingEpochDataPreconditions {
    pub ledger: LedgerPreconditions,
    pub seed: (PreconditionKind,),
    pub start_checkpoint: (PreconditionKind,),
    pub lock_checkpoint: (PreconditionKind,),
    pub epoch_length: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerPreconditions {
    pub hash: (PreconditionKind,),
    pub total_currency: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountPreconditions {
    pub balance: (PreconditionKind,),
    pub nonce: (PreconditionKind,),
    pub receipt_chain_hash: (PreconditionKind,),
    pub delegate: (PreconditionKind,),
    pub state: Vec<(PreconditionKind,)>,
    pub action_state: (PreconditionKind,),
    pub proved_state: (PreconditionKind,),
    pub is_new: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreconditionKind {
    Ignore,
    Check(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub elt: Box<Elt>,
    pub stack_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCommandPayloadCommon {
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
