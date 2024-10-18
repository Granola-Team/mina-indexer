pub mod command;
pub mod completed_work;

use super::protocol_state::SupplyAdjustment;
use crate::{
    ledger::public_key::PublicKey,
    mina_blocks::common::*,
    protocol::serialization_types::{
        signatures::SignatureJson, staged_ledger_diff::TransactionStatusFailedType,
    },
};
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

impl Eq for Command {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Status {
    Status((StatusKind,)),
    StatusAndFailure(StatusKind, (((TransactionStatusFailedType,),),)),
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
    UserCommandData(Box<UserCommandData>),
    ZkappCommandData(ZkappCommandData),
}

/// User command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCommandData {
    #[serde(deserialize_with = "from_str")]
    pub signer: PublicKey,

    pub payload: UserCommandPayload,
    pub signature: SignatureJson,
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappCommandData {
    pub fee_payer: FeePayer,
    pub account_updates: Vec<AccountUpdates>,

    // base58 encoded memo
    pub memo: String,
}

impl Eq for ZkappCommandData {}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FeePayer {
    pub body: FeePayerBody,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdates {
    pub elt: Elt,
    pub stack_hash: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Elt {
    pub account_update: AccountUpdate,
    pub account_update_digest: String,
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub body: AccountUpdateBody,
    pub authorization: Authorization,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ProofOrSignature {
    Proof,
    Signature,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authorization {
    #[serde(rename = "None_given")]
    NoneGiven,

    Either,
    Proof,
    Proof_((ProofOrSignature, String)),
    Signature,
    Signature_((ProofOrSignature, String)),
    Impossible,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdateBody {
    #[serde(deserialize_with = "from_str")]
    pub public_key: PublicKey,

    pub token_id: String,
    pub update: Update,
    pub balance_change: SupplyAdjustment,
    pub increment_nonce: bool,
    pub events: Vec<ZkappEvents>,
    pub actions: Vec<ZkappActions>,
    pub call_data: String,
    pub preconditions: Preconditions,
    pub use_full_commitment: bool,
    pub implicit_account_creation_fee: bool,
    pub may_use_token: (MayUseToken,),
    pub authorization_kind: Authorization,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MayUseToken {
    No,
    Yes,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappActions(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappEvents(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateKind {
    Keep,
    Set(String),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Preconditions {
    pub network: NetworkPreconditions,
    pub account: AccountPreconditions,
    pub valid_while: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NetworkPreconditions {
    pub snarked_ledger_hash: (PreconditionKind,),
    pub blockchain_length: (PreconditionKind,),
    pub min_window_density: (PreconditionKind,),
    pub total_currency: (PreconditionKind,),
    pub global_slot_since_genesis: (PreconditionKind,),
    pub staking_epoch_data: StakingEpochDataPreconditions,
    pub next_epoch_data: StakingEpochDataPreconditions,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StakingEpochDataPreconditions {
    pub ledger: LedgerPreconditions,
    pub seed: (PreconditionKind,),
    pub start_checkpoint: (PreconditionKind,),
    pub lock_checkpoint: (PreconditionKind,),
    pub epoch_length: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LedgerPreconditions {
    pub hash: (PreconditionKind,),
    pub total_currency: (PreconditionKind,),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreconditionKind {
    Ignore,
    Check(String),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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
