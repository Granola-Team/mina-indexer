use crate::{
    base::{
        amount::Amount, nonce::Nonce, public_key::PublicKey, scheduled_time::ScheduledTime, Balance,
    },
    command::TxnHash,
    ledger::token::TokenId,
};
use serde::{Deserialize, Serialize};

/// The staged ledger diff represents the list of changes applied to the
/// blockchain's ledger state between two consecutive blocks. It encapsulates
/// transactions, fees, and other modifications that update the ledger from its
/// previous state to the current one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    pub completed_works: Vec<CompletedWork>,
    pub commands: Vec<Command>,
    pub coinbase: Coinbase,
    pub internal_command_balances: Vec<(InternalCommandKind, InternalCommandBalanceData)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedWork {
    pub fee: Amount,
    pub prover: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub data: Option<(SignedCommandTag, SignedCommandData)>,
    pub status: Option<(SignedCommandStatus, AuxiliaryData, BalanaceData)>,
    pub txn_hash: Option<TxnHash>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignedCommandTag {
    #[serde(rename = "Signed_command")]
    SignedCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignedCommandStatus {
    Applied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCommandData {
    pub payload: SignedCommandPayload,
    pub signer: PublicKey,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCommandPayload {
    pub common: PayloadCommon,
    pub body: PayloadBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayloadBody {
    Delgation(PayloadKind, DelegationPayload),
    Payment(PayloadKind, PaymentPayload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PayloadKind {
    Payment,

    #[serde(rename = "Stake_delegation")]
    StakeDelegation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegationPayload(DelegationKind, DelegationPayloadBody);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DelegationKind {
    #[serde(rename = "Set_delegate")]
    SetDelegate,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegationPayloadBody {
    pub delegator: PublicKey,
    pub new_delegate: PublicKey,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub amount: Balance,
    pub token_id: TokenId,
    pub source_pk: PublicKey,
    pub receiver_pk: PublicKey,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadCommon {
    // base 58 encoded
    pub memo: String,
    pub fee_payer_pk: PublicKey,
    pub nonce: Nonce,
    pub valid_until: ScheduledTime,
    pub fee: Amount,
    pub fee_token: TokenId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Coinbase {
    Zero(CoinbaseKind),
    One(CoinbaseKind, Option<CoinbaseFeeTransfer>),
    Two(
        CoinbaseKind,
        Option<CoinbaseFeeTransfer>,
        Option<CoinbaseFeeTransfer>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoinbaseKind {
    Zero,
    One,
    Two,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoinbaseFeeTransfer {
    pub receiver_pk: PublicKey,
    pub fee: Balance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuxiliaryData {
    Applied(AppliedAuxData),
    Failed(FailureReason),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedAuxData {
    pub fee_payer_account_creation_fee_paid: Option<Balance>,
    pub receiver_account_creation_fee_paid: Option<Balance>,
    pub created_token: Option<TokenId>,
}

/// See https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/transaction_status.ml

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

    #[serde(rename = "Not_token_owner")]
    NotTokenOwner,

    #[serde(rename = "Mismatched_token_permissions")]
    MismatchedTokenPermissions,
    Overflow,

    #[serde(rename = "Signed_command_on_snapp_account")]
    SignedCommandOnSnappAccount,

    #[serde(rename = "Snapp_account_not_present")]
    SnappAccountNotPresent,

    #[serde(rename = "Update_not_permitted")]
    UpdateNotPermitted,

    #[serde(rename = "Incorrect_nonce")]
    IncorrectNonce,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanaceData {
    pub fee_payer_balance: Option<Balance>,
    pub source_balance: Option<Balance>,
    pub receiver_balance: Option<Balance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InternalCommandBalanceData {
    Coinbase(CoinbaseBalanceData),
    FeeTransfer(FeeTransferBalanceData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InternalCommandKind {
    Coinbase,

    #[serde(rename = "Fee_transfer")]
    FeeTransfer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoinbaseBalanceData {
    pub coinbase_receiver_balance: Option<Balance>,
    pub fee_transfer_receiver_balance: Option<Balance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeTransferBalanceData {
    pub receiver1_balance: Option<Balance>,
    pub receiver2_balance: Option<Balance>,
}
