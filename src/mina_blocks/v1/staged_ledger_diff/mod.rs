use super::common::{from_str, from_str_opt};
use serde::{Deserialize, Serialize};

/// The staged ledger diff represents the list of changes applied to the blockchain's ledger state between two consecutive blocks.
/// It encapsulates transactions, fees, and other modifications that update the ledger from its previous state to the current one.

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
    pub fee: String,
    pub prover: String,
    #[serde(skip_deserializing)]
    proofs: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub data: Option<(SignedCommandTag, SignedCommandData)>,
    pub status: Option<(SignedCommandStatus, AuxiliaryData, BalanaceData)>,
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
    pub signer: String,
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
    pub delegator: String,
    pub new_delegate: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentPayload {
    #[serde(deserialize_with = "from_str")]
    pub amount: u64,
    #[serde(deserialize_with = "from_str")]
    pub token_id: u64,
    pub source_pk: String,
    pub receiver_pk: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadCommon {
    #[serde(deserialize_with = "from_str")]
    pub fee_payer_pk: String,
    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,
    #[serde(deserialize_with = "from_str")]
    pub valid_until: u64,
    #[serde(deserialize_with = "from_str")]
    pub fee_token: u64,
    pub fee: String,
    pub memo: String,
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
    pub receiver_pk: String,
    #[serde(deserialize_with = "from_str")]
    pub fee: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuxiliaryData {
    Applied(AppliedAuxData),
    Failed(FailureReason),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedAuxData {
    #[serde(deserialize_with = "from_str_opt")]
    pub fee_payer_account_creation_fee_paid: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub receiver_account_creation_fee_paid: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub created_token: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureReason {
    Predicate,
    SourceNotPresent,
    ReceiverNotPresent,
    AmountInsufficientToCreateAccount,
    CannotPayCreationFeeInToken,
    SourceInsufficientBalance,
    SourceMinimumBalanceViolation,
    ReceiverAlreadyExists,
    NotTokenOwner,
    MismatchedTokenPermissions,
    Overflow,
    SignedCommandOnSnappAccount,
    SnappAccountNotPresent,
    UpdateNotPermitted,
    IncorrectNonce,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanaceData {
    #[serde(deserialize_with = "from_str_opt")]
    pub fee_payer_balance: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub source_balance: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub receiver_balance: Option<u64>,
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
    #[serde(deserialize_with = "from_str_opt")]
    pub coinbase_receiver_balance: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub fee_transfer_receiver_balance: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeTransferBalanceData {
    #[serde(deserialize_with = "from_str_opt")]
    pub receiver1_balance: Option<u64>,
    #[serde(deserialize_with = "from_str_opt")]
    pub receiver2_balance: Option<u64>,
}
