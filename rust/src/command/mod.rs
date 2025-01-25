pub mod internal;
pub mod signed;
pub mod store;
pub mod zkapp;

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::signed::{SignedCommand, SignedCommandWithKind},
    ledger::{amount::Amount, nonce::Nonce, public_key::PublicKey},
    mina_blocks::v2::{
        self,
        staged_ledger_diff::{
            SignedCommandPayloadBody, StakeDelegationPayload, UserCommandData, ZkappCommandData,
        },
    },
    protocol::serialization_types::staged_ledger_diff as mina_rs,
    utility::functions::nanomina_to_mina,
};
use log::trace;
use mina_serialization_versioned::Versioned;
use serde::{Deserialize, Serialize};
use signed::SignedCommandWithCreationData;

// re-export types
pub type TxnHash = signed::TxnHash;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserCommand {
    SignedCommand(SignedCommand),
    ZkappCommand(ZkappCommand),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum CommandType {
    Payment,
    Delegation,
    // Zkapp,
}

#[derive(PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum Command {
    Payment(Payment),

    #[serde(rename = "Stake_delegation")]
    Delegation(Delegation),

    Zkapp(v2::staged_ledger_diff::ZkappCommandData),
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CommandWithStateHash {
    pub command: Command,
    pub state_hash: BlockHash,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub source: PublicKey,
    pub nonce: Nonce,
    pub amount: Amount,
    pub receiver: PublicKey,
    pub is_new_receiver_account: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: PublicKey,
    pub nonce: Nonce,
    pub delegate: PublicKey,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CommandStatusData {
    Applied {
        auxiliary_data: Option<mina_rs::TransactionStatusAuxiliaryData>,
        balance_data: Option<mina_rs::TransactionStatusBalanceData>,
    },
    Failed(
        Vec<mina_rs::TransactionStatusFailedType>,
        Option<mina_rs::TransactionStatusBalanceData>,
    ),
}

impl CommandStatusData {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    fn balance_data(&self) -> Option<&mina_rs::TransactionStatusBalanceData> {
        if let Self::Applied { balance_data, .. } = self {
            return balance_data.as_ref();
        }
        None
    }

    fn auxiliary_data(&self) -> Option<&mina_rs::TransactionStatusAuxiliaryData> {
        if let Self::Applied { auxiliary_data, .. } = self {
            return auxiliary_data.as_ref();
        }
        None
    }

    pub fn fee_payer_balance(&self) -> Option<u64> {
        self.balance_data()
            .and_then(|b| b.fee_payer_balance.as_ref().map(|b| b.t.t.t))
    }

    pub fn receiver_balance(&self) -> Option<u64> {
        self.balance_data()
            .and_then(|b| b.receiver_balance.as_ref().map(|b| b.t.t.t))
    }

    pub fn source_balance(&self) -> Option<u64> {
        self.balance_data()
            .and_then(|b| b.source_balance.as_ref().map(|b| b.t.t.t))
    }

    pub fn fee_payer_account_creation_fee_paid(&self) -> Option<u64> {
        self.auxiliary_data().and_then(|b| {
            b.fee_payer_account_creation_fee_paid
                .as_ref()
                .map(|b| b.t.t)
        })
    }

    pub fn receiver_account_creation_fee_paid(&self) -> Option<u64> {
        self.auxiliary_data()
            .and_then(|b| b.receiver_account_creation_fee_paid.as_ref().map(|b| b.t.t))
    }

    pub fn created_token(&self) -> Option<u64> {
        self.auxiliary_data()
            .and_then(|b| b.created_token.as_ref().map(|b| b.t.t.t))
    }

    pub fn from_transaction_status_v1(data: &mina_rs::TransactionStatus1) -> Self {
        match data {
            mina_rs::TransactionStatus1::Applied(auxiliary_data, balance_data) => Self::Applied {
                auxiliary_data: Some(auxiliary_data.t.to_owned()),
                balance_data: Some(balance_data.t.to_owned()),
            },
            mina_rs::TransactionStatus1::Failed(fails, balance_data) => Self::Failed(
                fails.iter().map(|reason| reason.t.to_owned()).collect(),
                Some(balance_data.t.to_owned()),
            ),
        }
    }

    pub fn from_transaction_status_v2(data: &v2::staged_ledger_diff::Status) -> Self {
        use v2::staged_ledger_diff::Status::*;
        match data {
            Status(_) => Self::Applied {
                auxiliary_data: None,
                balance_data: None,
            },
            StatusAndFailure(_, (((fails,),),)) => Self::Failed(
                vec![fails]
                    .into_iter()
                    .map(|reason| reason.to_owned())
                    .collect(),
                None,
            ),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum UserCommandWithStatus {
    V1(Box<mina_rs::UserCommandWithStatusV1>),
    V2(v2::staged_ledger_diff::UserCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkappCommand(v2::staged_ledger_diff::ZkappCommandData);

pub trait UserCommandWithStatusT {
    fn is_applied(&self) -> bool;

    fn is_zkapp_command(&self) -> bool;

    fn status_data(&self) -> CommandStatusData;

    fn contains_public_key(&self, pk: &PublicKey) -> bool;

    fn to_command(&self) -> Command;

    fn sender(&self) -> PublicKey;

    fn receiver(&self) -> Vec<PublicKey>;

    fn fee_payer_pk(&self) -> PublicKey;

    fn fee(&self) -> u64;

    fn nonce(&self) -> Nonce;

    fn amount(&self) -> u64;

    fn memo(&self) -> String;

    fn signer(&self) -> PublicKey;

    fn receiver_account_creation_fee_paid(&self) -> bool;
}

impl UserCommandWithStatusT for UserCommandWithStatus {
    fn is_applied(&self) -> bool {
        self.status_data().is_applied()
    }

    fn is_zkapp_command(&self) -> bool {
        use v2::staged_ledger_diff as v2;

        matches!(
            *self,
            UserCommandWithStatus::V2(v2::UserCommand {
                data: (_, v2::UserCommandData::ZkappCommandData { .. }),
                ..
            })
        )
    }

    fn receiver_account_creation_fee_paid(&self) -> bool {
        self.status_data()
            .receiver_account_creation_fee_paid()
            .is_some()
    }

    fn status_data(&self) -> CommandStatusData {
        match self {
            Self::V1(v1) => CommandStatusData::from_transaction_status_v1(&v1.t.status.t),
            Self::V2(v2) => CommandStatusData::from_transaction_status_v2(&v2.status),
        }
    }

    fn contains_public_key(&self, pk: &PublicKey) -> bool {
        let signed = SignedCommand::from(self.clone());
        signed.all_command_public_keys().contains(pk)
    }

    fn to_command(&self) -> Command {
        match self {
            Self::V1(v1) => match &v1.t.data.t.t {
                mina_rs::UserCommand1::SignedCommand(v1) => {
                    use mina_rs::SignedCommandPayloadBody1::*;
                    match &v1.t.t.payload.t.t.body.t.t {
                        PaymentPayload(v1) => {
                            let mina_rs::PaymentPayload1 {
                                source_pk,
                                receiver_pk,
                                amount,
                                ..
                            } = &v1.t.t;
                            Command::Payment(Payment {
                                source: source_pk.to_owned().into(),
                                receiver: receiver_pk.to_owned().into(),
                                nonce: self.nonce(),
                                amount: amount.t.t.into(),
                                is_new_receiver_account: self.receiver_account_creation_fee_paid(),
                            })
                        }
                        StakeDelegation(v1) => {
                            let mina_rs::StakeDelegation1::SetDelegate {
                                delegator,
                                new_delegate,
                            } = v1.t.to_owned();
                            Command::Delegation(Delegation {
                                delegator: delegator.into(),
                                delegate: new_delegate.into(),
                                nonce: self.nonce(),
                            })
                        }
                    }
                }
            },
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(v2) => match &v2.payload.body.1 {
                    SignedCommandPayloadBody::Payment(v2::staged_ledger_diff::PaymentPayload {
                        receiver_pk,
                        amount,
                    }) => Command::Payment(Payment {
                        nonce: self.nonce(),
                        amount: (*amount).into(),
                        source: self.fee_payer_pk(),
                        receiver: receiver_pk.to_owned(),
                        is_new_receiver_account: self.receiver_account_creation_fee_paid(),
                    }),
                    SignedCommandPayloadBody::StakeDelegation((
                        _,
                        v2::staged_ledger_diff::StakeDelegationPayload { new_delegate },
                    )) => Command::Delegation(Delegation {
                        nonce: self.nonce(),
                        delegator: self.sender(),
                        delegate: new_delegate.to_owned(),
                    }),
                },
                UserCommandData::ZkappCommandData(v1) => Command::Zkapp(v1.to_owned()),
            },
        }
    }

    fn sender(&self) -> PublicKey {
        use mina_rs::*;
        match self {
            Self::V1(v1) => match &v1.t.data.t.t {
                UserCommand1::SignedCommand(v1) => match &v1.t.t.payload.t.t.body.t.t {
                    SignedCommandPayloadBody1::PaymentPayload(payment_payload_v1) => {
                        let PaymentPayload1 { ref source_pk, .. } = payment_payload_v1.t.t;
                        source_pk.to_owned().into()
                    }
                    SignedCommandPayloadBody1::StakeDelegation(stake_delegation_v1) => {
                        let StakeDelegation1::SetDelegate { ref delegator, .. } =
                            stake_delegation_v1.t;
                        delegator.to_owned().into()
                    }
                },
            },
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => {
                    data.payload.common.fee_payer_pk.to_owned()
                }
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer.body.public_key.to_owned()
                }
            },
        }
    }

    fn receiver(&self) -> Vec<PublicKey> {
        use mina_rs::*;
        match self {
            Self::V1(v1) => {
                let UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                match &v1.t.t.payload.t.t.body.t.t {
                    SignedCommandPayloadBody1::PaymentPayload(body) => {
                        vec![body.t.t.receiver_pk.to_owned().into()]
                    }
                    SignedCommandPayloadBody1::StakeDelegation(body) => {
                        let StakeDelegation1::SetDelegate { new_delegate, .. } = &body.t;
                        vec![new_delegate.to_owned().into()]
                    }
                }
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => match &data.payload.body.1 {
                    SignedCommandPayloadBody::Payment(payload) => {
                        vec![payload.receiver_pk.to_owned()]
                    }
                    SignedCommandPayloadBody::StakeDelegation((
                        _,
                        StakeDelegationPayload { new_delegate },
                    )) => {
                        vec![new_delegate.to_owned()]
                    }
                },
                UserCommandData::ZkappCommandData(zkapp) => zkapp
                    .account_updates
                    .iter()
                    .map(|update| update.elt.account_update.body.public_key.to_owned())
                    .collect(),
            },
        }
    }

    fn fee_payer_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                let mina_rs::UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                v1.t.t
                    .payload
                    .t
                    .t
                    .common
                    .t
                    .t
                    .t
                    .fee_payer_pk
                    .to_owned()
                    .into()
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => {
                    data.payload.common.fee_payer_pk.to_owned()
                }
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer.body.public_key.to_owned()
                }
            },
        }
    }

    fn fee(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                let mina_rs::UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                v1.t.t.payload.t.t.common.t.t.t.fee.t.t
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => data.payload.common.fee,
                UserCommandData::ZkappCommandData(data) => data.fee_payer.body.fee,
            },
        }
    }

    fn nonce(&self) -> Nonce {
        match self {
            Self::V1(v1) => {
                let mina_rs::UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                Nonce(v1.t.t.payload.t.t.common.t.t.t.nonce.t.t as u32)
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => data.payload.common.nonce.into(),
                UserCommandData::ZkappCommandData(data) => data.fee_payer.body.nonce.into(),
            },
        }
    }

    fn amount(&self) -> u64 {
        use v2::staged_ledger_diff::{PaymentPayload, SignedCommandPayloadBody::*};
        match self {
            Self::V1(v1) => {
                let mina_rs::UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                v1.t.t.payload.t.t.common.t.t.t.fee.t.t
            }
            Self::V2(cmd) => match &cmd.data.1 {
                UserCommandData::SignedCommandData(data) => match data.payload.body.1 {
                    Payment(PaymentPayload { amount, .. }) => amount,
                    StakeDelegation(_) => 0,
                },
                UserCommandData::ZkappCommandData(_data) => 0,
            },
        }
    }

    /// Decoded memo
    fn memo(&self) -> String {
        use mina_rs::*;
        match self {
            Self::V1(v1) => {
                let UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                decode_memo(&v1.t.t.payload.t.t.common.t.t.t.memo.t.0)
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => {
                    decode_memo(data.payload.common.memo.as_bytes())
                }
                UserCommandData::ZkappCommandData(data) => decode_memo(data.memo.as_bytes()),
            },
        }
    }

    fn signer(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                let mina_rs::UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                v1.t.t.signer.to_owned().into()
            }
            Self::V2(v2) => match &v2.data.1 {
                UserCommandData::SignedCommandData(data) => data.signer.to_owned(),
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer.body.public_key.to_owned()
                }
            },
        }
    }
}

pub const MEMO_LEN: usize = 32;

/// Decode memo
///
/// 0th byte - tag to distinguish digests from other data
/// 1st byte - is length, always 32 for digests
/// bytes 2 to 33 - are data, 0-right-padded if length is less than 32

pub fn decode_memo(encoded: &[u8]) -> String {
    let value = &encoded[2..(encoded[1] as usize + 2).min(encoded.len())];
    String::from_utf8(value.to_vec()).unwrap_or_default()
}

impl From<String> for mina_rs::SignedCommandMemo {
    fn from(value: String) -> Self {
        let mut bytes = value.as_bytes().to_vec();
        bytes.insert(0, bytes.len() as u8);
        bytes.insert(0, 1); // version byte
        Self(bytes)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PaymentPayload {
    V1(mina_rs::PaymentPayloadV1),
    V2 {
        payment: mina_rs::PaymentPayloadV2,
        sender: PublicKey,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum StakeDelegation {
    V1(mina_rs::StakeDelegationV1),
    V2 {
        delegation: mina_rs::StakeDelegationV2,
        sender: PublicKey,
    },
}

impl Command {
    /// Get the list of commands from the precomputed block
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block
            .commands()
            .iter()
            .filter(|command| command.is_applied())
            .map(|command| {
                use mina_rs::*;
                match command {
                    UserCommandWithStatus::V1(v1) => {
                        let UserCommand1::SignedCommand(v1) = &v1.t.data.t.t;
                        match &v1.t.t.payload.t.t.body.t.t {
                            SignedCommandPayloadBody1::PaymentPayload(v1) => {
                                let source: PublicKey = v1.t.t.source_pk.to_owned().into();
                                let receiver: PublicKey = v1.t.t.receiver_pk.to_owned().into();
                                let amount = v1.t.t.amount.t.t;

                                trace!("Payment {{ source: {source}, receiver: {receiver}, amount: {amount} }}");
                                Self::Payment(Payment {
                                    source,
                                    receiver,
                                    nonce: command.nonce(),
                                    amount: amount.into(),
                                    is_new_receiver_account: command.receiver_account_creation_fee_paid(),
                                })
                            }
                            SignedCommandPayloadBody1::StakeDelegation(v1) => {
                                let StakeDelegation1::SetDelegate { delegator, new_delegate } = v1.t.to_owned();
                                let delegator: PublicKey = delegator.into();
                                let new_delegate: PublicKey = new_delegate.into();
                                let nonce = command.nonce();

                                trace!("Delegation {{ delegator: {delegator}, new_delegate: {new_delegate}, nonce: {nonce} }}");
                                Self::Delegation(Delegation {
                                    delegate: new_delegate,
                                    delegator,
                                    nonce,
                                })
                            }
                        }
                    }
                    UserCommandWithStatus::V2(v2) => match &v2.data.1 {
                        UserCommandData::SignedCommandData(data) => match &data.payload.body.1 {
                            SignedCommandPayloadBody::Payment(payload) => {
                                let source = command.fee_payer_pk();
                                let receiver = payload.receiver_pk.to_owned();
                                let amount = payload.amount;

                                trace!("Payment {{ source: {source}, receiver: {receiver}, amount: {amount} }}");
                                Self::Payment(Payment {
                                    source,
                                    receiver,
                                    amount: amount.into(),
                                    nonce: command.nonce(),
                                    is_new_receiver_account: command.receiver_account_creation_fee_paid(),
                                })
                            }
                            SignedCommandPayloadBody::StakeDelegation(payload) => {
                                let delegator: PublicKey = command.sender();
                                let nonce = command.nonce();
                                let delegate = payload.1.new_delegate.to_owned();

                                trace!("Delegation {{ delegator: {delegator}, new_delegate: {delegate}, nonce: {nonce} }}");
                                Self::Delegation(Delegation {
                                    nonce,
                                    delegate,
                                    delegator,
                                })
                            }
                        }
                        UserCommandData::ZkappCommandData(data) => Self::Zkapp(data.to_owned()),
                    }
                }}).collect()
    }

    pub fn nonce(&self) -> Nonce {
        match self {
            Self::Delegation(Delegation { nonce, .. }) => *nonce,
            Self::Payment(Payment { nonce, .. }) => *nonce,
            Self::Zkapp(zkapp) => todo!("nonce {zkapp:?}"),
        }
    }
}

impl PaymentPayload {
    pub fn source_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1.t.t.source_pk.to_owned().into(),
            Self::V2 { sender, .. } => sender.to_owned(),
        }
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1.t.t.receiver_pk.to_owned().into(),
            Self::V2 { payment, .. } => payment.t.t.receiver_pk.to_owned().into(),
        }
    }
}

impl StakeDelegation {
    pub fn delegator(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                let mina_rs::StakeDelegation1::SetDelegate { delegator, .. } = &v1.t;
                delegator.to_owned().into()
            }
            Self::V2 { sender, .. } => sender.to_owned(),
        }
    }

    pub fn new_delegate(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                let mina_rs::StakeDelegation1::SetDelegate { new_delegate, .. } = &v1.t;
                new_delegate.to_owned().into()
            }
            Self::V2 { delegation, .. } => {
                let mina_rs::StakeDelegation2::SetDelegate { new_delegate, .. } = &delegation.t;
                new_delegate.to_owned().into()
            }
        }
    }
}

/////////////////
// Conversions //
/////////////////

impl From<(UserCommand, bool)> for Command {
    fn from(value: (UserCommand, bool)) -> Self {
        let value: SignedCommandWithCreationData = value.into();
        value.into()
    }
}

impl From<v2::staged_ledger_diff::UserCommand> for UserCommandWithStatus {
    fn from(value: v2::staged_ledger_diff::UserCommand) -> Self {
        Self::V2(value)
    }
}

impl From<mina_rs::UserCommand1> for UserCommand {
    fn from(value: mina_rs::UserCommand1) -> Self {
        let mina_rs::UserCommand1::SignedCommand(v1) = value;
        Self::SignedCommand(SignedCommand::V1(Box::new(v1)))
    }
}

impl From<v2::staged_ledger_diff::UserCommand> for UserCommand {
    fn from(value: v2::staged_ledger_diff::UserCommand) -> Self {
        match &value.data.1 {
            UserCommandData::SignedCommandData(_data) => {
                Self::SignedCommand(SignedCommand::V2(value.data.1))
            }
            UserCommandData::ZkappCommandData(data) => {
                Self::ZkappCommand(ZkappCommand(data.to_owned()))
            }
        }
    }
}

impl From<mina_rs::UserCommandWithStatus1> for UserCommandWithStatus {
    fn from(value: mina_rs::UserCommandWithStatus1) -> Self {
        Self::V1(Box::new(Versioned::new(value)))
    }
}

impl From<UserCommandWithStatus> for Command {
    fn from(value: UserCommandWithStatus) -> Self {
        let account_creation_fee_paid = value.receiver_account_creation_fee_paid();
        match value {
            UserCommandWithStatus::V1(v1) => {
                (v1.t.data.t.t.to_owned().into(), account_creation_fee_paid).into()
            }
            UserCommandWithStatus::V2(v2) => (v2.into(), account_creation_fee_paid).into(),
        }
    }
}

// status conversions

impl From<UserCommandWithStatus> for CommandStatusData {
    fn from(value: UserCommandWithStatus) -> Self {
        value.status_data()
    }
}

impl From<v2::staged_ledger_diff::Status> for mina_rs::TransactionStatus2 {
    fn from(value: v2::staged_ledger_diff::Status) -> Self {
        use v2::staged_ledger_diff::{Status, StatusKind};

        match value {
            Status::Status((StatusKind::Applied,)) => Self::Applied,
            Status::StatusAndFailure(StatusKind::Failed, (((failure,),),)) => {
                Self::Failed(vec![Versioned::new(failure)])
            }
            _ => unreachable!(),
        }
    }
}

// debug/display

impl std::fmt::Debug for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result: String = match self {
            Self::Payment => "PAYMENT".into(),
            Self::Delegation => "STAKE_DELEGATION".into(),
        };
        write!(f, "{result}")
    }
}

//////////////////////
// JSON conversions //
//////////////////////

impl From<CommandStatusData> for serde_json::Value {
    fn from(value: CommandStatusData) -> Self {
        use serde_json::*;

        match value {
            CommandStatusData::Applied {
                auxiliary_data,
                balance_data,
            } => {
                let mut applied_obj = Map::new();
                let status = Value::String("Applied".into());
                let aux_json = to_auxiliary_json(&auxiliary_data);
                let balance_json = to_balance_json(&balance_data);

                applied_obj.insert("kind".into(), status);
                applied_obj.insert("auxiliary_data".into(), aux_json);
                applied_obj.insert("balance_data".into(), balance_json);

                Value::Object(applied_obj)
            }
            CommandStatusData::Failed(reason, balance_data) => {
                let mut failed_obj = Map::new();
                let status = Value::String("Failed".into());
                let reason_json = Value::Array(
                    reason
                        .iter()
                        .map(|r| {
                            Value::String(serde_json::to_string(&r).expect("serialize reason"))
                        })
                        .collect(),
                );
                let balance_json = to_balance_json(&balance_data);

                failed_obj.insert("kind".into(), status);
                failed_obj.insert("reason".into(), reason_json);
                failed_obj.insert("balance_data".into(), balance_json);

                Value::Object(failed_obj)
            }
        }
    }
}

impl From<Command> for serde_json::Value {
    fn from(value: Command) -> Self {
        use serde_json::*;

        let mut json = Map::new();
        match value {
            Command::Payment(Payment {
                source,
                receiver,
                nonce,
                amount,
                is_new_receiver_account: _,
            }) => {
                let mut payment = Map::new();

                payment.insert("source".into(), Value::String(source.to_address()));
                payment.insert("receiver".into(), Value::String(receiver.to_address()));
                payment.insert("amount".into(), Value::Number(Number::from(amount.0)));
                payment.insert("nonce".into(), Value::Number(Number::from(nonce)));
                json.insert("Payment".into(), Value::Object(payment));

                Value::Object(json)
            }

            Command::Delegation(Delegation {
                delegate,
                delegator,
                nonce,
            }) => {
                let mut delegation = Map::new();

                delegation.insert("delegate".into(), Value::String(delegate.to_address()));
                delegation.insert("delegator".into(), Value::String(delegator.to_address()));
                delegation.insert("nonce".into(), Value::Number(Number::from(nonce)));
                json.insert("Stake_delegation".into(), Value::Object(delegation));

                Value::Object(json)
            }

            Command::Zkapp(zkapp) => to_zkapp_json(&zkapp),
        }
    }
}

pub fn to_zkapp_json(data: &ZkappCommandData) -> Value {
    let json = serde_json::to_value(data).expect("zkapp json value");
    to_mina_json(json)
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let json: Value = self.clone().into();
        write!(f, "{}", to_string_pretty(&json).unwrap())
    }
}

impl From<UserCommandWithStatus> for serde_json::Value {
    fn from(value: UserCommandWithStatus) -> Self {
        use serde_json::*;

        let user_cmd: UserCommandWithStatus = match value {
            UserCommandWithStatus::V1(v1) => v1.inner().into(),
            UserCommandWithStatus::V2(_) => value,
        };
        let status: CommandStatusData = user_cmd.clone().into();
        let data: SignedCommandWithKind = user_cmd.into();

        let mut object = Map::new();
        object.insert("data".into(), data.into());
        object.insert("status".into(), status.into());

        Value::Object(object)
    }
}

impl std::fmt::Debug for UserCommandWithStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let json: Value = self.clone().into();
        write!(f, "{}", to_string_pretty(&json).unwrap())
    }
}

fn to_auxiliary_json(
    auxiliary_data: &Option<mina_rs::TransactionStatusAuxiliaryData>,
) -> serde_json::Value {
    use serde_json::*;

    let mut auxiliary_obj = Map::new();
    let fee_payer_account_creation_fee_paid = auxiliary_data
        .as_ref()
        .and_then(|aux| {
            aux.fee_payer_account_creation_fee_paid
                .as_ref()
                .map(|amt| Value::Number(Number::from(amt.t.t)))
        })
        .unwrap_or(Value::Null);
    let receiver_account_creation_fee_paid = auxiliary_data
        .as_ref()
        .and_then(|aux| {
            aux.receiver_account_creation_fee_paid
                .as_ref()
                .map(|amt| Value::Number(Number::from(amt.t.t)))
        })
        .unwrap_or(Value::Null);
    let created_token = auxiliary_data
        .as_ref()
        .and_then(|aux| {
            aux.created_token
                .as_ref()
                .map(|id| Value::Number(Number::from(id.t.t.t)))
        })
        .unwrap_or(Value::Null);

    auxiliary_obj.insert(
        "fee_payer_account_creation_fee_paid".into(),
        fee_payer_account_creation_fee_paid,
    );
    auxiliary_obj.insert(
        "receiver_account_creation_fee_paid".into(),
        receiver_account_creation_fee_paid,
    );
    auxiliary_obj.insert("created_token".into(), created_token);

    Value::Object(auxiliary_obj)
}

fn to_balance_json(
    balance_data: &Option<mina_rs::TransactionStatusBalanceData>,
) -> serde_json::Value {
    use serde_json::*;

    let mut balance_obj = Map::new();
    let fee_payer_balance = balance_data
        .as_ref()
        .and_then(|bal| {
            bal.fee_payer_balance
                .as_ref()
                .map(|amt| Value::Number(Number::from(amt.t.t.t)))
        })
        .unwrap_or(Value::Null);
    let receiver_balance = balance_data
        .as_ref()
        .and_then(|bal| {
            bal.receiver_balance
                .as_ref()
                .map(|amt| Value::Number(Number::from(amt.t.t.t)))
        })
        .unwrap_or(Value::Null);
    let source_balance = balance_data
        .as_ref()
        .and_then(|bal| {
            bal.source_balance
                .as_ref()
                .map(|amt| Value::Number(Number::from(amt.t.t.t)))
        })
        .unwrap_or(Value::Null);

    balance_obj.insert("fee_payer_balance".into(), fee_payer_balance);
    balance_obj.insert("receiver_balance".into(), receiver_balance);
    balance_obj.insert("source_balance".into(), source_balance);

    Value::Object(balance_obj)
}

use serde_json::Value;

fn convert(test: bool, value: Value) -> Value {
    match value {
        Value::Number(n) => Value::String(n.to_string()),
        Value::Object(mut obj) => {
            obj.iter_mut().for_each(|(key, x)| {
                if test && (key == "memo" || key == "signature") {
                    *x = Value::Null
                } else {
                    *x = convert(test, x.clone())
                }
            });
            Value::Object(obj)
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|json| convert(test, json)).collect())
        }
        x => x,
    }
}

fn fee_convert(value: Value) -> Value {
    match value {
        Value::Object(mut obj) => {
            obj.iter_mut().for_each(|(key, x)| {
                if key == "fee" {
                    if let Ok(nanomina) = x.clone().to_string().parse::<u64>() {
                        *x = Value::String(nanomina_to_mina(nanomina));
                    }
                } else {
                    *x = fee_convert(x.clone())
                }
            });
            Value::Object(obj)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(fee_convert).collect()),
        x => x,
    }
}

/// Convert to Mina precomputed block json format
pub fn to_mina_format(json: Value) -> Value {
    match json {
        Value::Object(mut obj) => {
            let keys: Vec<String> = obj.keys().cloned().collect();
            if keys.contains(&"data".into()) {
                // signed command
                if let Value::Object(mut data) = obj["data"].clone() {
                    let kind = obj["data"]["kind"].clone();
                    if kind == Value::String("Signed_command".into()) {
                        data.remove("kind");
                        obj["data"] = Value::Array(vec![kind, Value::Object(data)]);
                    }
                }

                obj.iter_mut()
                    .for_each(|(_, v)| *v = to_mina_format(v.clone()));
                Value::Object(obj)
            } else if keys.contains(&"body".into()) {
                // payment/delegation
                if let Value::Object(mut body) = obj["body"].clone() {
                    let kind = obj["body"]["kind"].clone();

                    if kind == Value::String("Payment".into()) {
                        body.remove("kind");
                        obj["body"] =
                            Value::Array(vec![kind.to_owned(), Value::Object(body.to_owned())]);
                    }

                    if kind == Value::String("Stake_delegation".into()) {
                        body.remove("kind");

                        if let Some(set_delegate) = body.remove("Set_delegate") {
                            obj["body"] = Value::Array(vec![
                                kind,
                                Value::Array(vec!["Set_delegate".into(), set_delegate]),
                            ]);
                        } else {
                            obj["body"] = Value::Array(vec![kind, Value::Object(body)]);
                        }
                    }
                }

                obj.iter_mut()
                    .for_each(|(_, v)| *v = to_mina_format(v.clone()));
                Value::Object(obj)
            } else if keys.contains(&"kind".into())
                && keys.contains(&"auxiliary_data".into())
                && keys.contains(&"balance_data".into())
            {
                // applied status
                Value::Array(vec![
                    obj["kind"].clone(),
                    obj["auxiliary_data"].clone(),
                    obj["balance_data"].clone(),
                ])
            } else if keys.contains(&"kind".into())
                && keys.contains(&"reason".into())
                && keys.contains(&"balance_data".into())
            {
                // failed status
                Value::Array(vec![
                    obj["kind"].clone(),
                    obj["reason"].clone(),
                    obj["balance_data"].clone(),
                ])
            } else {
                obj.iter_mut()
                    .for_each(|(_, val)| *val = to_mina_format(val.clone()));
                Value::Object(obj)
            }
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(to_mina_format).collect()),
        x => x,
    }
}

pub fn to_mina_json(json: Value) -> Value {
    to_mina_format(convert(false, fee_convert(json)))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        block::{parser::BlockParser, precomputed::PcbVersion},
        command::decode_memo,
        constants::*,
    };
    use std::path::PathBuf;

    #[test]
    fn decode_memo_test() {
        let expected = "MIP4".to_string();
        // encoded memo for: MIP4
        let bytes: Vec<u8> = vec![
            1, 4, 77, 73, 80, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 177, 160, 56, 149,
        ];
        let actual = decode_memo(&bytes);
        assert_eq!(&expected, &actual);
    }

    #[tokio::test]
    async fn mainnet_from_precomputed() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw
        let log_dir = PathBuf::from("./tests/data/non_sequential_blocks");
        let mut bp = BlockParser::new_with_canonical_chain_discovery(
            &log_dir,
            PcbVersion::V1,
            MAINNET_CANONICAL_THRESHOLD,
            false,
            BLOCK_REPORTING_FREQ_NUM,
        )
        .await
        .unwrap();
        let (block, _) = bp
            .get_precomputed_block("3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw")
            .await
            .unwrap();

        let mut payments = Vec::new();
        let mut delegations = Vec::new();
        for command in Command::from_precomputed(&block) {
            match command {
                Command::Payment(Payment {
                    source,
                    receiver,
                    nonce,
                    amount,
                    is_new_receiver_account: _,
                }) => {
                    println!("s: {source:?}");
                    println!("r: {receiver:?}");
                    println!("n: {nonce}");
                    println!("a: {}", amount.0);
                    payments.push((source, receiver, amount));
                }

                Command::Delegation(Delegation {
                    delegate,
                    delegator,
                    nonce,
                }) => {
                    println!("d: {delegate:?}");
                    println!("t: {delegator:?}");
                    println!("n: {nonce}");
                    delegations.push((delegate, delegator));
                }

                Command::Zkapp(_) => unreachable!("no zkapp commands pre-hardfork"),
            }
        }

        {
            let expected_payments: Vec<(String, String, u64)> = Vec::from([
                (
                    "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG".to_string(),
                    "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs".to_string(),
                    536900000000,
                ),
                (
                    "B62qmx2tqhBo6UJE7MnKZaANkUUFzXYqYTAfdfaThVp6qEET6eBjjxv".to_string(),
                    "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs".to_string(),
                    22461950000,
                ),
                (
                    "B62qkRodi7nj6W1geB12UuW2XAx2yidWZCcDthJvkf9G4A6G5GFasVQ".to_string(),
                    "B62qjXJM4ceboQWDetGqEreDGrNBgz7vpE4pA6ANUnZnY1aCVmBFkeg".to_string(),
                    34950000000,
                ),
                (
                    "B62qkRodi7nj6W1geB12UuW2XAx2yidWZCcDthJvkf9G4A6G5GFasVQ".to_string(),
                    "B62qks3mcCJyrtAwWQraUEFgvkeMRqDBZ8guR1Sfn2DdxkYHCxtf4cp".to_string(),
                    26821540000,
                ),
                (
                    "B62qiW9Qwv9UnKfNKdBm6hRLNDobv46rVhX1trGdB35YCNT33CSCVt5".to_string(),
                    "B62qrzT6RGecDyB8qxEQozjnXuoVFvHqtqWsbRzxdPwbBfLtXh6oqLN".to_string(),
                    2664372460000,
                ),
                (
                    "B62qjtgrHSZmEtDknYjQxgW4Kv4VSTCfUJjo11m7RuPqSh2f5PQKwSf".to_string(),
                    "B62qjdHDsvxeu7DvnDyjqAb1V9LTRXZAc2i4ACttaXYrXn7sTWfshN1".to_string(),
                    376000000000,
                ),
                (
                    "B62qrAWZFqvgJbfU95t1owLAMKtsDTAGgSZzsBJYUzeQZ7dQNMmG5vw".to_string(),
                    "B62qjdk4R6rjtrJpWypvMcpNMdfyqgxHEAz88UnzbMK4TzELiGbhQ97".to_string(),
                    30000000,
                ),
                (
                    "B62qrAWZFqvgJbfU95t1owLAMKtsDTAGgSZzsBJYUzeQZ7dQNMmG5vw".to_string(),
                    "B62qjdk4R6rjtrJpWypvMcpNMdfyqgxHEAz88UnzbMK4TzELiGbhQ97".to_string(),
                    30000000,
                ),
                (
                    "B62qroyTTjddSX4LQrY9eZX5Qy3NtVsTGZpwmUNEvyvwjkqmV6Qng9J".to_string(),
                    "B62qox5t2dmZD2DbUfHLZqgCepqLAryyCqKx58WJHGhGEkgcnm9eFti".to_string(),
                    2461233340000,
                ),
                (
                    "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".to_string(),
                    "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM".to_string(),
                    1000,
                ),
                (
                    "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".to_string(),
                    "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM".to_string(),
                    1000,
                ),
                (
                    "B62qmPc8Ziq7txW48YPf4qtavcD5mcQVjEAGo9LEZD8DeaNsNthYLsz".to_string(),
                    "B62qmRpvhsNujrT6i5rwkw5HX3yt6P9brtKumL4nkHTFktrykMpfJS8".to_string(),
                    16655643423,
                ),
                (
                    "B62qmPc8Ziq7txW48YPf4qtavcD5mcQVjEAGo9LEZD8DeaNsNthYLsz".to_string(),
                    "B62qq96XNJZsiWomcDdsSagEzQenKcvNZ885NzJini5wG1feb8DgSxd".to_string(),
                    16630819368,
                ),
                (
                    "B62qmPc8Ziq7txW48YPf4qtavcD5mcQVjEAGo9LEZD8DeaNsNthYLsz".to_string(),
                    "B62qmYUXXeZHgfosEuMYuKf9KPstd7N1qGE3g2FM2G9rk8o4RxvbnTu".to_string(),
                    16604512519,
                ),
            ]);
            assert_eq!(
                expected_payments,
                payments
                    .iter()
                    .map(|(s, r, a)| (s.to_address(), r.to_address(), a.0))
                    .collect::<Vec<(String, String, u64)>>()
            );
        }

        {
            let expected_delegations = Vec::from([
                (
                    "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6".to_string(),
                    "B62qkR88P9oYWzPwJPgA5X5xbkN3LL7m3d8E8FJfG9enttiAAjYRubk".to_string(),
                ),
                (
                    "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz".to_string(),
                    "B62qmTBnmE7tPPZsx3mu44nMKirVG7Wb64XbdTe6Q7Pbu2R89PTBZpZ".to_string(),
                ),
                (
                    "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz".to_string(),
                    "B62qmTBnmE7tPPZsx3mu44nMKirVG7Wb64XbdTe6Q7Pbu2R89PTBZpZ".to_string(),
                ),
                (
                    "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz".to_string(),
                    "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi".to_string(),
                ),
            ]);
            assert_eq!(
                expected_delegations,
                delegations
                    .iter()
                    .map(|(d, t)| (d.to_address(), t.to_address()))
                    .collect::<Vec<(String, String)>>()
            );
        }
    }

    #[test]
    fn mainnet_user_command_with_status_json() -> anyhow::Result<()> {
        use crate::block::precomputed::PrecomputedBlock;
        use serde_json::*;

        fn to_mina_json(json: Value) -> Value {
            to_mina_format(super::convert(true, fee_convert(json)))
        }

        fn convert(value: Value) -> Value {
            super::convert(true, value)
        }

        fn convert_v1_to_v2(json: Value) -> Value {
            match json {
                Value::Object(mut obj) => {
                    obj.iter_mut().for_each(|(key, value)| {
                        if key == "body" {
                            if let Value::Array(arr) = value {
                                if let Value::Object(map) = arr.get_mut(1).unwrap() {
                                    map.remove("source_pk");
                                    map.remove("token_id");
                                }
                            }
                        } else if key == "common" {
                            if let Value::Object(map) = value {
                                map.remove("fee_token");
                            }
                        } else if key == "status" {
                            if let Value::Array(arr) = value {
                                *value = Value::Array(vec![arr.first().cloned().unwrap()]);
                            }
                        } else {
                            *value = convert_v1_to_v2(value.clone())
                        }
                    });
                    Value::Object(obj)
                }
                Value::Array(arr) => Value::Array(arr.into_iter().map(convert_v1_to_v2).collect()),
                x => x,
            }
        }

        // v1
        let path: PathBuf = "./tests/data/non_sequential_blocks/mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw.json".into();
        let contents = std::fs::read(path.clone())?;
        let mina_json: Value =
            from_slice::<Value>(&contents)?["staged_ledger_diff"]["diff"][0]["commands"][0].clone();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let user_cmd_with_status = block.commands()[0].clone();
        let user_cmd_with_status: Value = user_cmd_with_status.into();

        assert_eq!(
            convert(mina_json),
            to_mina_json(user_cmd_with_status.clone())
        );
        assert_eq!(
            serde_json::to_string_pretty(&to_mina_json(user_cmd_with_status)).unwrap(),
            r#"{
  "data": [
    "Signed_command",
    {
      "payload": {
        "body": [
          "Payment",
          {
            "amount": "536900000000",
            "receiver_pk": "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs",
            "source_pk": "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG",
            "token_id": "1"
          }
        ],
        "common": {
          "fee": "0.1",
          "fee_payer_pk": "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG",
          "fee_token": "1",
          "memo": null,
          "nonce": "14",
          "valid_until": "4294967295"
        }
      },
      "signature": null,
      "signer": "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"
    }
  ],
  "status": [
    "Applied",
    {
      "created_token": null,
      "fee_payer_account_creation_fee_paid": null,
      "receiver_account_creation_fee_paid": null
    },
    {
      "fee_payer_balance": "0",
      "receiver_balance": "4347326279755751",
      "source_balance": "0"
    }
  ]
}"#
        );

        // v2

        let path: PathBuf = "./tests/data/hardfork/mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg.json".into();
        let contents = std::fs::read(path.clone())?;
        let mina_json: Value = from_slice::<Value>(&contents)?["data"]["staged_ledger_diff"]
            ["diff"][0]["commands"][0]
            .clone();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let user_cmd_with_status = block.commands()[0].clone();
        let user_cmd_with_status: Value = user_cmd_with_status.into();

        assert_eq!(
            convert(mina_json),
            convert_v1_to_v2(to_mina_json(user_cmd_with_status.clone()))
        );

        assert_eq!(
            serde_json::to_string_pretty(&convert_v1_to_v2(to_mina_json(user_cmd_with_status)))
                .unwrap(),
            r#"{
  "data": [
    "Signed_command",
    {
      "payload": {
        "body": [
          "Payment",
          {
            "amount": "1000000000",
            "receiver_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
          }
        ],
        "common": {
          "fee": "0.0011",
          "fee_payer_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
          "memo": null,
          "nonce": "765",
          "valid_until": "4294967295"
        }
      },
      "signature": null,
      "signer": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
    }
  ],
  "status": [
    "Applied"
  ]
}"#
        );

        Ok(())
    }
}
