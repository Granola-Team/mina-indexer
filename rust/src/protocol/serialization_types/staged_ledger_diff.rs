// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! In this context a diff refers to a difference between two states of the
//! blockchain. In this case it is between the current state and the proposed
//! next state.

#![allow(missing_docs)] // Don't actually know what many of the types fields are for yet

use crate::{
    command::TxnHash,
    protocol::serialization_types::{
        common::{
            AmountV1, DecimalJson, ExtendedU32, ExtendedU64_2, ExtendedU64_3, U32Json, U64Json,
        },
        signatures::{PublicKey2V1, PublicKeyJson, PublicKeyV1, SignatureJson, SignatureV1},
        snark_work::{TransactionSnarkWorkJson, TransactionSnarkWorkV1},
        version_bytes,
    },
};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::{
    impl_mina_enum_json_serde, impl_mina_enum_json_serde_with_option, Versioned, Versioned2,
    Versioned3,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smart_default::SmartDefault;
use std::str::FromStr;

/// Top level wrapper type for a StagedLedgerDiff
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StagedLedgerDiff {
    pub diff: StagedLedgerDiffTupleV1,
}

/// Top level wrapper type for a StagedLedgerDiff (json)
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(StagedLedgerDiff)]
pub struct StagedLedgerDiffJson {
    pub diff: StagedLedgerDiffTupleJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StagedLedgerDiffTuple(pub StagedLedgerPreDiffV1, pub Option<StagedLedgerPreDiffV1>);

pub type StagedLedgerDiffTupleV1 = Versioned<StagedLedgerDiffTuple, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(StagedLedgerDiffTuple)]
pub struct StagedLedgerDiffTupleJson(
    pub StagedLedgerPreDiffJson,
    pub Option<StagedLedgerPreDiffJson>,
);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StagedLedgerPreDiff {
    pub completed_works: Vec<TransactionSnarkWorkV1>,
    pub commands: Vec<UserCommandWithStatusV1>,
    pub coinbase: CoinBaseV1,
    pub internal_command_balances: Vec<InternalCommandBalanceDataV1>,
}

pub type StagedLedgerPreDiffV1 = Versioned2<StagedLedgerPreDiff, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(StagedLedgerPreDiff)]
pub struct StagedLedgerPreDiffJson {
    pub completed_works: Vec<TransactionSnarkWorkJson>,
    pub commands: Vec<UserCommandWithStatusJson>,
    pub coinbase: CoinBaseJson,
    pub internal_command_balances: Vec<InternalCommandBalanceDataJson>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct UserCommandWithStatus1 {
    pub data: UserCommandV1,
    pub status: TransactionStatusV1,
    pub txn_hash: Option<TxnHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct UserCommandWithStatus2 {
    pub data: UserCommandV2,
    pub status: TransactionStatusV2,
    pub txn_hash: Option<TxnHash>,
}

// v1 pre-hardfork
pub type UserCommandWithStatusV1 = Versioned<UserCommandWithStatus1, 1>;

// v2 post-hardfork
pub type UserCommandWithStatusV2 = Versioned<UserCommandWithStatus2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(UserCommandWithStatus1)]
pub struct UserCommandWithStatusJson {
    pub data: UserCommandJson,
    pub status: TransactionStatusJson,
    pub txn_hash: Option<TxnHash>,
}

// User command versions
// Poly: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/user_command.ml#L3-L27
// V2:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/user_command.ml#L78-L80
// V1:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/user_command.ml#L116-L117

/// v1 pre-hardfork
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum UserCommand1 {
    SignedCommand(SignedCommandV1),
}

/// v2 post-hardfork
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum UserCommand2 {
    SignedCommand(Box<SignedCommandV2>),
    ZkappCommand(ZkappCommandV1),
}

// v1 pre-hardfork
// ---------------
// type t =
//   (Signed_command.Stable.V1.t, Snapp_command.Stable.V1.t) Poly.Stable.V1.t
pub type UserCommandV1 = Versioned2<UserCommand1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t =
//   (Signed_command.Stable.V2.t, Zkapp_command.Stable.V1.t) Poly.Stable.V2.t
pub type UserCommandV2 = Versioned2<UserCommand2, 2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum UserCommandJsonProxy {
    #[serde(rename = "Signed_command")]
    SignedCommand(SignedCommandJson),
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(UserCommand1)]
#[auto_from(UserCommandJsonProxy)]
pub enum UserCommandJson {
    SignedCommand(SignedCommandJson),
}

impl_mina_enum_json_serde!(UserCommandJson, UserCommandJsonProxy);

// Signed command versions
// Poly: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command.ml#L19-L34
// V2:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command.ml#L50-L54
// V1:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command.ml#L80-L84

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommand1 {
    pub payload: SignedCommandPayloadV1,
    pub signer: PublicKey2V1,
    pub signature: SignatureV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommand2 {
    pub payload: SignedCommandPayloadV2,
    pub signer: PublicKey2V1,
    pub signature: SignatureV1,
}

// v1 pre-hardfork
// ---------------
// type t =
//   ( Payload.Stable.V1.t
//   , Public_key.Stable.V1.t
//   , Signature.Stable.V1.t )
//   Poly.Stable.V1.t
pub type SignedCommandV1 = Versioned2<SignedCommand1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t =
//   ( Payload.Stable.V2.t
//   , Public_key.Stable.V1.t
//   , Signature.Stable.V1.t )
//   Poly.Stable.V1.t
pub type SignedCommandV2 = Versioned2<SignedCommand2, 2, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(SignedCommand1)]
pub struct SignedCommandJson {
    pub payload: SignedCommandPayloadJson,
    pub signer: PublicKeyJson,
    pub signature: SignatureJson,
}

// Signed command payload versions
// Poly: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L239-L243
// V2:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L257
// V1:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L266

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommandPayload1 {
    pub common: SignedCommandPayloadCommonV1,
    pub body: SignedCommandPayloadBodyV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommandPayload2 {
    pub common: SignedCommandPayloadCommonV2,
    pub body: SignedCommandPayloadBodyV2,
}

// v1 pre-hardfork
// ---------------
// type t = (Common.Stable.V1.t, Body.Stable.V1.t) Poly.Stable.V1.t
pub type SignedCommandPayloadV1 = Versioned2<SignedCommandPayload1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t = (Common.Stable.V2.t, Body.Stable.V2.t) Poly.Stable.V1.t
pub type SignedCommandPayloadV2 = Versioned2<SignedCommandPayload2, 2, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(SignedCommandPayload1)]
pub struct SignedCommandPayloadJson {
    pub common: SignedCommandPayloadCommonJson,
    pub body: SignedCommandPayloadBodyJson,
}

// Signed command payload common versions
// Poly: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L32-L64
// V2:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L70-L76
// V1:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L85-L92

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommandPayloadCommon1 {
    pub fee: AmountV1,
    pub fee_token: SignedCommandFeeTokenV1,
    pub fee_payer_pk: PublicKeyV1,
    pub nonce: ExtendedU32,
    pub valid_until: ExtendedU32,
    pub memo: SignedCommandMemoV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommandPayloadCommon2 {
    pub fee: AmountV1,
    pub fee_payer_pk: PublicKeyV1,
    pub nonce: ExtendedU32,
    pub valid_until: ExtendedU32,
    pub memo: SignedCommandMemoV1,
}

// v1 pre-hardfork
// ---------------
// type t =
//   ( Currency.Fee.Stable.V1.t
//   , Public_key.Compressed.Stable.V1.t
//   , Token_id.Stable.V1.t
//   , Account_nonce.Stable.V1.t
//   , Global_slot_legacy.Stable.V1.t
//   , Memo.Stable.V1.t )
//   Poly.Stable.V1.t
pub type SignedCommandPayloadCommonV1 = Versioned3<SignedCommandPayloadCommon1, 1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t =
//   ( Currency.Fee.Stable.V1.t
//   , Public_key.Compressed.Stable.V1.t
//   , Account_nonce.Stable.V1.t
//   , Global_slot_since_genesis.Stable.V1.t
//   , Memo.Stable.V1.t )
//   Poly.Stable.V2.t
pub type SignedCommandPayloadCommonV2 = Versioned3<SignedCommandPayloadCommon2, 2, 2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(SignedCommandPayloadCommon1)]
pub struct SignedCommandPayloadCommonJson {
    pub fee: DecimalJson,
    pub fee_token: U64Json,
    pub fee_payer_pk: PublicKeyJson,
    pub nonce: U32Json,
    pub valid_until: U32Json,
    pub memo: SignedCommandMemoJson,
}

// Signed command body versions
// V2: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L179-L181
// V1: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/signed_command_payload.ml#L190-L192

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SignedCommandPayloadBody1 {
    PaymentPayload(PaymentPayloadV1),
    StakeDelegation(StakeDelegationV1),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SignedCommandPayloadBody2 {
    PaymentPayload(PaymentPayloadV2),
    StakeDelegation(StakeDelegationV2),
}

// v1 pre-hardfork
// ---------------
// type t =
//   | Payment of Payment_payload.Stable.V1.t
//   | Stake_delegation of Stake_delegation.Stable.V1.t
pub type SignedCommandPayloadBodyV1 = Versioned2<SignedCommandPayloadBody1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t = Mina_wire_types.Mina_base.Signed_command_payload.Body.V2.t =
//   | Payment of Payment_payload.Stable.V2.t
//   | Stake_delegation of Stake_delegation.Stable.V2.t
pub type SignedCommandPayloadBodyV2 = Versioned2<SignedCommandPayloadBody2, 2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum SignedCommandPayloadBodyJsonProxy {
    #[serde(rename = "Payment")]
    PaymentPayload(PaymentPayloadJson),
    #[serde(rename = "Stake_delegation")]
    StakeDelegation(StakeDelegationJson),
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(SignedCommandPayloadBody1)]
#[auto_from(SignedCommandPayloadBodyJsonProxy)]
pub enum SignedCommandPayloadBodyJson {
    PaymentPayload(PaymentPayloadJson),
    StakeDelegation(StakeDelegationJson),
}

impl_mina_enum_json_serde_with_option!(
    SignedCommandPayloadBodyJson,
    SignedCommandPayloadBodyJsonProxy,
    false
);

// Payment payload versions
// Poly: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/payment_payload.ml#L9-L31
// V2:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/payment_payload.ml#L37-L38
// V1:   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/payment_payload.ml#L47-L51

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PaymentPayload1 {
    pub source_pk: PublicKeyV1,
    pub receiver_pk: PublicKeyV1,
    pub token_id: ExtendedU64_3,
    pub amount: AmountV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PaymentPayload2 {
    pub receiver_pk: PublicKeyV1,
    pub amount: AmountV1,
}

// v1 per-hardfork
// ---------------
// type t =
//   ( Public_key.Compressed.Stable.V1.t
//   , Token_id.Stable.V1.t
//   , Amount.Stable.V1.t )
//   Poly.Stable.V1.t
pub type PaymentPayloadV1 = Versioned2<PaymentPayload1, 1, 1>;

// v2 post-hardfork
// ----------------
// type t =
//   (Public_key.Compressed.Stable.V1.t, Amount.Stable.V1.t) Poly.Stable.V2.t
pub type PaymentPayloadV2 = Versioned2<PaymentPayload2, 2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(PaymentPayload1)]
pub struct PaymentPayloadJson {
    pub source_pk: PublicKeyJson,
    pub receiver_pk: PublicKeyJson,
    pub token_id: U64Json,
    pub amount: U64Json,
}

// Stake delegation versions
// V2: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/stake_delegation.ml#L11-L12
// V1: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/stake_delegation.ml#L21-L25

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum StakeDelegation1 {
    SetDelegate {
        delegator: PublicKeyV1,
        new_delegate: PublicKeyV1,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum StakeDelegation2 {
    SetDelegate { new_delegate: PublicKeyV1 },
}

// v1 pre-hardfork
// ---------------
// type t = Mina_wire_types.Mina_base.Stake_delegation.V1.t =
//   | Set_delegate of
//     { delegator : Public_key.Compressed.Stable.V1.t
//     ; new_delegate : Public_key.Compressed.Stable.V1.t
//     }
pub type StakeDelegationV1 = Versioned<StakeDelegation1, 1>;

// v2 post-harfork
// ---------------
// type t = Mina_wire_types.Mina_base.Stake_delegation.V2.t =
//   | Set_delegate of { new_delegate : Public_key.Compressed.Stable.V1.t }
pub type StakeDelegationV2 = Versioned<StakeDelegation2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum StakeDelegationJsonProxy {
    #[serde(rename = "Set_delegate")]
    SetDelegate {
        delegator: PublicKeyJson,
        new_delegate: PublicKeyJson,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(StakeDelegation1)]
#[auto_from(StakeDelegationJsonProxy)]
pub enum StakeDelegationJson {
    SetDelegate {
        delegator: PublicKeyJson,
        new_delegate: PublicKeyJson,
    },
}

impl_mina_enum_json_serde!(StakeDelegationJson, StakeDelegationJsonProxy);

pub type SignedCommandFeeTokenV1 = Versioned3<u64, 1, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignedCommandMemo(pub Vec<u8>);

pub type SignedCommandMemoV1 = Versioned<SignedCommandMemo, 1>;

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(SignedCommandMemo)]
pub struct SignedCommandMemoJson(pub Vec<u8>);

impl Serialize for SignedCommandMemoJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = bs58::encode(self.0.as_slice())
            .with_check_version(version_bytes::USER_COMMAND_MEMO)
            .into_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SignedCommandMemoJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decoded = bs58::decode(s)
            .with_check(Some(version_bytes::USER_COMMAND_MEMO))
            .into_vec()
            .map_err(<D::Error as serde::de::Error>::custom)?;
        // Skip base58 check byte
        Ok(Self(decoded.into_iter().skip(1).collect()))
    }
}

// Zkapp command version
// V1: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/zkapp_command.ml#L55-L63

/// v2 post-harfork
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkappCommand {
    // TODO use mina_blocks::v2::staged_ledger_diff::Command to implement
}

pub type ZkappCommandV1 = Versioned2<ZkappCommand, 1, 1>;

// Transaction status versions
// V2: https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/transaction_status.ml#L474-L476

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TransactionStatus1 {
    Applied(
        TransactionStatusAuxiliaryDataV1,
        TransactionStatusBalanceDataV1,
    ),
    Failed(
        Vec<TransactionStatusFailedTypeV1>,
        TransactionStatusBalanceDataV1,
    ),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TransactionStatus2 {
    Applied,
    Failed(Vec<Vec<Vec<TransactionStatusFailedTypeV2>>>),
}

// v1 pre-hardfork
pub type TransactionStatusV1 = Versioned<TransactionStatus1, 1>;

// v2 post-hardfork
pub type TransactionStatusV2 = Versioned<TransactionStatus2, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum TransactionStatusJsonProxy {
    Applied(
        TransactionStatusAuxiliaryDataJson,
        TransactionStatusBalanceDataJson,
    ),
    Failed(
        Vec<TransactionStatusFailedTypeJson>,
        TransactionStatusBalanceDataJson,
    ),
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionStatus1)]
#[auto_from(TransactionStatusJsonProxy)]
pub enum TransactionStatusJson {
    Applied(
        TransactionStatusAuxiliaryDataJson,
        TransactionStatusBalanceDataJson,
    ),
    Failed(
        Vec<TransactionStatusFailedTypeJson>,
        TransactionStatusBalanceDataJson,
    ),
}

impl_mina_enum_json_serde!(TransactionStatusJson, TransactionStatusJsonProxy);

#[derive(Default, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionStatusAuxiliaryData {
    pub fee_payer_account_creation_fee_paid: Option<AmountV1>,
    pub receiver_account_creation_fee_paid: Option<AmountV1>,
    pub created_token: Option<ExtendedU64_3>,
}

pub type TransactionStatusAuxiliaryDataV1 = Versioned<TransactionStatusAuxiliaryData, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionStatusAuxiliaryData)]
pub struct TransactionStatusAuxiliaryDataJson {
    pub fee_payer_account_creation_fee_paid: Option<U64Json>,
    pub receiver_account_creation_fee_paid: Option<U64Json>,
    pub created_token: Option<U64Json>,
}

/// See https://github.com/MinaProtocol/mina/blob/berkeley/src/lib/mina_base/transaction_status.ml

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TransactionStatusFailedType {
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

impl FromStr for TransactionStatusFailedType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Predicate" => Ok(Self::Predicate),
            "Source_not_present" => Ok(Self::SourceNotPresent),
            "Receiver_not_present" => Ok(Self::ReceiverNotPresent),
            "Amount_insufficient_to_create_account" => Ok(Self::AmountInsufficientToCreateAccount),
            "Cannot_pay_creation_fee_in_token" => Ok(Self::CannotPayCreationFeeInToken),
            "Source_insufficient_balance" => Ok(Self::SourceInsufficientBalance),
            "Source_minimum_balance_violation" => Ok(Self::SourceMinimumBalanceViolation),
            "Receiver_already_exists" => Ok(Self::ReceiverAlreadyExists),
            "Token_owner_not_caller" => Ok(Self::TokenOwnerNotCaller),
            "Overflow" => Ok(Self::Overflow),
            "Global_excess_overflow" => Ok(Self::GlobalExcessOverflow),
            "Local_excess_overflow" => Ok(Self::LocalExcessOverflow),
            "Local_supply_increase_overflow" => Ok(Self::LocalSupplyIncreaseOverflow),
            "Global_supply_increase_overflow" => Ok(Self::GlobalSupplyIncreaseOverflow),
            "Signed_command_on_zkapp_account" => Ok(Self::SignedCommandOnZkappAccount),
            "Zkapp_account_not_present" => Ok(Self::ZkappAccountNotPresent),
            "Update_not_permitted_balance" => Ok(Self::UpdateNotPermittedBalance),
            "Update_not_permitted_access" => Ok(Self::UpdateNotPermittedAccess),
            "Update_not_permitted_timing" => Ok(Self::UpdateNotPermittedTiming),
            "Update_not_permitted_delegate" => Ok(Self::UpdateNotPermittedDelegate),
            "Update_not_permitted_app_state" => Ok(Self::UpdateNotPermittedAppState),
            "Update_not_permitted_verification_key" => Ok(Self::UpdateNotPermittedVerificationKey),
            "Update_not_permitted_action_state" => Ok(Self::UpdateNotPermittedactionState),
            "Update_not_permitted_zkapp_uri" => Ok(Self::UpdateNotPermittedZkappUri),
            "Update_not_permitted_token_symbol" => Ok(Self::UpdateNotPermittedTokenSymbol),
            "Update_not_permitted_permissions" => Ok(Self::UpdateNotPermittedpermissions),
            "Update_not_permitted_nonce" => Ok(Self::UpdateNotPermittedNonce),
            "Update_not_permitted_voting_for" => Ok(Self::UpdateNotPermittedVotingFor),
            "Zkapp_command_replay_check_failed" => Ok(Self::ZkappCommandReplayCheckFailed),
            "Fee_payer_nonce_must_increase" => Ok(Self::FeePayerNonceMustIncrease),
            "Fee_payer_must_be_signed" => Ok(Self::FeePayerMustBeSigned),
            "Account_balance_precondition_unsatisfied" => {
                Ok(Self::AccountBalancePreconditionUnsatisfied)
            }
            "Account_nonce_precondition_unsatisfied" => {
                Ok(Self::AccountNoncePreconditionUnsatisfied)
            }
            "Account_receipt_chain_hash_precondition_unsatisfied" => {
                Ok(Self::AccountReceiptChainHashPreconditionUnsatisfied)
            }
            "Account_delegate_precondition_unsatisfied" => {
                Ok(Self::AccountDelegatePreconditionUnsatisfied)
            }
            "Account_action_state_precondition_unsatisfied" => {
                Ok(Self::AccountActionStatePreconditionUnsatisfied)
            }
            "Account_proved_state_precondition_unsatisfied" => {
                Ok(Self::AccountProvedStatePreconditionUnsatisfied)
            }
            "Account_is_new_precondition_unsatisfied" => {
                Ok(Self::AccountIsNewPreconditionUnsatisfied)
            }
            "Protocol_state_precondition_unsatisfied" => {
                Ok(Self::ProtocolStatePreconditionUnsatisfied)
            }
            "Unexpected_verification_key_hash" => Ok(Self::UnexpectedVerificationKeyHash),
            "Valid_while_precondition_unsatisfied" => Ok(Self::ValidWhilePreconditionUnsatisfied),
            "Incorrect_nonce" => Ok(Self::IncorrectNonce),
            "Invalid_fee_excess" => Ok(Self::InvalidFeeExcess),
            "Cancelled" => Ok(Self::Cancelled),
            s if s.starts_with("Account_app_state_precondition_unsatisfied") => {
                // Handle the parameterized variant
                if let Some(num_str) = s.split(',').nth(1) {
                    if let Ok(num) = num_str.trim().parse::<i64>() {
                        return Ok(Self::AccountAppStatePreconditionUnsatisfied(num));
                    }
                }
                Err(format!(
                    "Invalid Account_app_state_precondition_unsatisfied parameter: {}",
                    s
                ))
            }
            _ => Err(format!("Unknown failure type: {}", s)),
        }
    }
}

// v1 pre-hardfork
pub type TransactionStatusFailedTypeV1 = Versioned<TransactionStatusFailedType, 1>;

// v2 post-hardfork
pub type TransactionStatusFailedTypeV2 = Versioned<TransactionStatusFailedType, 2>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum TransactionStatusFailedTypeJsonProxy {
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionStatusFailedType)]
#[auto_from(TransactionStatusFailedTypeJsonProxy)]
pub enum TransactionStatusFailedTypeJson {
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

#[derive(Default, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionStatusBalanceData {
    pub fee_payer_balance: Option<ExtendedU64_3>,
    pub source_balance: Option<ExtendedU64_3>,
    pub receiver_balance: Option<ExtendedU64_3>,
}

pub type TransactionStatusBalanceDataV1 = Versioned<TransactionStatusBalanceData, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(TransactionStatusBalanceData)]
pub struct TransactionStatusBalanceDataJson {
    pub fee_payer_balance: Option<U64Json>,
    pub source_balance: Option<U64Json>,
    pub receiver_balance: Option<U64Json>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, SmartDefault)]
pub enum CoinBase {
    #[default]
    #[serde(rename = "Zero")]
    None,
    #[serde(rename = "One")]
    Coinbase(Option<CoinBaseFeeTransferV1>),
    #[serde(rename = "Two")]
    CoinbaseAndFeeTransferViaCoinbase(Option<CoinBaseFeeTransferV1>, Option<CoinBaseFeeTransferV1>),
}

pub type CoinBaseV1 = Versioned<CoinBase, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, SmartDefault)]
enum CoinBaseJsonProxy {
    #[default]
    #[serde(rename = "Zero")]
    None,
    #[serde(rename = "One")]
    Coinbase(Option<CoinBaseFeeTransferJson>),
    #[serde(rename = "Two")]
    CoinbaseAndFeeTransferViaCoinbase(
        Option<CoinBaseFeeTransferJson>,
        Option<CoinBaseFeeTransferJson>,
    ),
}

#[derive(Clone, Debug, Eq, PartialEq, SmartDefault, AutoFrom)]
#[auto_from(CoinBase)]
#[auto_from(CoinBaseJsonProxy)]
pub enum CoinBaseJson {
    #[default]
    None,
    Coinbase(Option<CoinBaseFeeTransferJson>),
    CoinbaseAndFeeTransferViaCoinbase(
        Option<CoinBaseFeeTransferJson>,
        Option<CoinBaseFeeTransferJson>,
    ),
}

impl_mina_enum_json_serde!(CoinBaseJson, CoinBaseJsonProxy);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CoinBaseFeeTransfer {
    pub receiver_pk: PublicKeyV1,
    pub fee: ExtendedU64_2,
}

pub type CoinBaseFeeTransferV1 = Versioned2<CoinBaseFeeTransfer, 1, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(CoinBaseFeeTransfer)]
pub struct CoinBaseFeeTransferJson {
    pub receiver_pk: PublicKeyJson,
    pub fee: DecimalJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum InternalCommandBalanceData {
    CoinBase(CoinBaseBalanceDataV1),
    FeeTransfer(FeeTransferBalanceDataV1),
}

pub type InternalCommandBalanceDataV1 = Versioned<InternalCommandBalanceData, 1>;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum InternalCommandBalanceDataJsonProxy {
    #[serde(rename = "Coinbase")]
    CoinBase(CoinBaseBalanceDataJson),
    #[serde(rename = "Fee_transfer")]
    FeeTransfer(FeeTransferBalanceDataJson),
}

#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(InternalCommandBalanceData)]
#[auto_from(InternalCommandBalanceDataJsonProxy)]
pub enum InternalCommandBalanceDataJson {
    CoinBase(CoinBaseBalanceDataJson),
    FeeTransfer(FeeTransferBalanceDataJson),
}

impl_mina_enum_json_serde!(
    InternalCommandBalanceDataJson,
    InternalCommandBalanceDataJsonProxy
);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CoinBaseBalanceData {
    pub coinbase_receiver_balance: ExtendedU64_3,
    // FIXME: No test coverage yet
    pub fee_transfer_receiver_balance: Option<ExtendedU64_3>,
}

pub type CoinBaseBalanceDataV1 = Versioned<CoinBaseBalanceData, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(CoinBaseBalanceData)]
pub struct CoinBaseBalanceDataJson {
    pub coinbase_receiver_balance: U64Json,
    pub fee_transfer_receiver_balance: Option<U64Json>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FeeTransferBalanceData {
    pub receiver1_balance: ExtendedU64_3,
    // FIXME: No test coverage yet
    pub receiver2_balance: Option<ExtendedU64_3>,
}

pub type FeeTransferBalanceDataV1 = Versioned<FeeTransferBalanceData, 1>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(FeeTransferBalanceData)]
pub struct FeeTransferBalanceDataJson {
    pub receiver1_balance: U64Json,
    pub receiver2_balance: Option<U64Json>,
}

impl std::fmt::Display for TransactionStatusFailedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f, "{}", s.trim_matches('"'))
    }
}
