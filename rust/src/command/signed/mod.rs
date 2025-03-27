mod txn_hash;

use crate::{
    command::*,
    ledger::token::{TokenAddress, TokenId},
    mina_blocks::v2::{self, staged_ledger_diff::UserCommandData},
    proof_systems::signer::signature::Signature,
    protocol::serialization_types::staged_ledger_diff as mina_rs,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

// re-export [txn_hash::TxnHash]
pub type TxnHash = txn_hash::TxnHash;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum SignedCommand {
    V1(Box<mina_rs::SignedCommandV1>),
    V2(UserCommandData),
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithCreationData {
    pub signed_command: SignedCommand,
    pub is_new_receiver_account: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithStateHash {
    pub command: SignedCommand,
    pub state_hash: StateHash,
    pub is_new_receiver_account: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithData {
    pub command: SignedCommand,
    pub state_hash: StateHash,
    pub status: CommandStatusData,
    pub tx_hash: TxnHash,
    pub blockchain_length: u32,
    pub date_time: u64,
    pub nonce: Nonce,
    pub global_slot_since_genesis: u32,
}

impl SignedCommand {
    ////////////////////
    // Payload Common //
    ////////////////////

    pub fn fee(&self) -> u64 {
        match self {
            Self::V1(v1) => v1.t.t.payload.t.t.common.t.t.t.fee.t.t,
            Self::V2(UserCommandData::SignedCommandData(data)) => data.payload.common.fee.0,
            Self::V2(UserCommandData::ZkappCommandData(data)) => data.fee_payer.body.fee.0,
        }
    }

    pub fn fee_payer_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
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
            Self::V2(v2) => match &v2 {
                UserCommandData::SignedCommandData(data) => {
                    data.payload.common.fee_payer_pk.to_owned()
                }
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer.body.public_key.to_owned()
                }
            },
        }
    }

    pub fn nonce(&self) -> Nonce {
        Nonce(match self {
            Self::V1(v1) => v1.t.t.payload.t.t.common.t.t.t.nonce.t.t as u32,
            Self::V2(v2) => match &v2 {
                UserCommandData::SignedCommandData(data) => data.payload.common.nonce.0,
                UserCommandData::ZkappCommandData(data) => data.fee_payer.body.nonce.0,
            },
        })
    }

    pub fn valid_until(&self) -> i32 {
        match self {
            Self::V1(v1) => v1.t.t.payload.t.t.common.t.t.t.valid_until.t.t,
            Self::V2(v2) => match &v2 {
                UserCommandData::SignedCommandData(data) => {
                    data.payload.common.valid_until.0 as i32
                }
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer
                        .body
                        .valid_until
                        .as_ref()
                        .map_or(u64::MAX, |t| t.0) as i32
                }
            },
        }
    }

    /// Decoded memo
    pub fn memo(&self) -> String {
        match self {
            Self::V1(v1) => decode_memo(v1.t.t.payload.t.t.common.t.t.t.memo.t.0.as_slice(), true),
            Self::V2(v2) => match &v2 {
                UserCommandData::SignedCommandData(data) => {
                    decode_memo(data.payload.common.memo.as_bytes(), false)
                }
                UserCommandData::ZkappCommandData(data) => decode_memo(data.memo.as_bytes(), false),
            },
        }
    }

    /// Fee token id
    pub fn fee_token(&self) -> Option<TokenId> {
        match self {
            Self::V1(v1) => Some(v1.t.t.payload.t.t.common.t.t.t.fee_token.t.t.t.into()),
            Self::V2(_v2) => None,
        }
    }

    //////////////////
    // Payload Body //
    //////////////////

    pub fn amount(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v1) => v1.t.t.amount.t.t,
                    StakeDelegation(_) => 0,
                }
            }
            Self::V2(data) => {
                use v2::staged_ledger_diff::{SignedCommandPayloadBody::*, *};
                match data {
                    UserCommandData::SignedCommandData(data) => match &data.payload.body.1 {
                        Payment(PaymentPayload { amount, .. }) => amount.0,
                        StakeDelegation(_) => 0,
                    },
                    UserCommandData::ZkappCommandData(_data) => 0,
                }
            }
        }
    }

    pub fn receiver_pk(&self) -> Vec<PublicKey> {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v1) => vec![v1.t.t.receiver_pk.to_owned().into()],
                    StakeDelegation(v1) => match v1.t {
                        mina_rs::StakeDelegation1::SetDelegate {
                            ref new_delegate, ..
                        } => vec![new_delegate.to_owned().into()],
                    },
                }
            }
            Self::V2(data) => {
                use v2::staged_ledger_diff::{SignedCommandPayloadBody::*, *};
                match data {
                    UserCommandData::SignedCommandData(data) => match &data.payload.body.1 {
                        Payment(PaymentPayload { receiver_pk, .. }) => vec![receiver_pk.to_owned()],
                        StakeDelegation((_, StakeDelegationPayload { new_delegate })) => {
                            vec![new_delegate.to_owned()]
                        }
                    },
                    UserCommandData::ZkappCommandData(data) => data
                        .account_updates
                        .iter()
                        .map(|update| update.elt.account_update.body.public_key.to_owned())
                        .collect(),
                }
            }
        }
    }

    pub fn source_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(payment_payload) => {
                        payment_payload.t.t.source_pk.to_owned().into()
                    }
                    StakeDelegation(delegation_payload) => match delegation_payload.t {
                        mina_rs::StakeDelegation1::SetDelegate {
                            ref delegator,
                            new_delegate: _,
                        } => delegator.to_owned().into(),
                    },
                }
            }
            Self::V2(_v2) => self.fee_payer_pk(),
        }
    }

    /// All tokens involved in the transaction
    pub fn tokens(&self) -> Vec<TokenAddress> {
        let mut tokens = vec![];

        if let Self::V2(UserCommandData::ZkappCommandData(data)) = self {
            zkapp_tokens(data, &mut tokens);
            tokens
        } else {
            vec![TokenAddress::default()]
        }
    }

    // other data

    pub fn signer(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1.t.t.signer.0.t.to_owned().into(),
            Self::V2(v2) => match v2 {
                UserCommandData::SignedCommandData(data) => data.signer.to_owned(),
                UserCommandData::ZkappCommandData(data) => {
                    data.fee_payer.body.public_key.to_owned()
                }
            },
        }
    }

    pub fn all_command_public_keys(&self) -> Vec<PublicKey> {
        let mut pks = self.receiver_pk();

        pks.push(self.source_pk());
        pks.push(self.fee_payer_pk());
        pks.push(self.signer());

        pks
    }

    pub fn contains_public_key(&self, pk: &PublicKey) -> bool {
        self.all_command_public_keys().contains(pk)
    }

    pub fn kind(&self) -> CommandType {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(_) => CommandType::Payment,
                    StakeDelegation(_) => CommandType::Delegation,
                }
            }
            Self::V2(UserCommandData::SignedCommandData(data)) => {
                use v2::staged_ledger_diff::SignedCommandPayloadKind::*;
                match &data.payload.body.0 {
                    Payment => CommandType::Payment,
                    StakeDelegation => CommandType::Delegation,
                }
            }
            Self::V2(UserCommandData::ZkappCommandData(_)) => CommandType::Zkapp,
        }
    }

    pub fn is_delegation(&self) -> bool {
        matches!(self.kind(), CommandType::Delegation)
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc {
            UserCommandWithStatus::V1(v1) => match v1.t.data.t.t {
                mina_rs::UserCommand1::SignedCommand(v1) => Self::V1(Box::new(v1)),
            },
            UserCommandWithStatus::V2(v2) => Self::V2(v2.data.1),
        }
    }

    /// Returns a user command (transaction) hash
    pub fn hash_signed_command(&self) -> Result<TxnHash> {
        match self {
            Self::V1(v1) => super::hash_command_v1(v1),
            Self::V2(v2) => super::hash_command_v2(v2),
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<SignedCommandWithCreationData> {
        block
            .commands()
            .into_iter()
            .map(|u| SignedCommandWithCreationData {
                is_new_receiver_account: u.receiver_account_creation_fee_paid(),
                signed_command: Self::from(u),
            })
            .collect()
    }
}

impl SignedCommandWithStateHash {
    pub fn from(
        signed_cmd: &SignedCommand,
        state_hash: &str,
        is_new_receiver_account: bool,
    ) -> Self {
        Self {
            command: signed_cmd.clone(),
            state_hash: state_hash.into(),
            is_new_receiver_account,
        }
    }
}

impl SignedCommandWithData {
    pub fn is_zkapp_command(&self) -> bool {
        matches!(self.command.kind(), CommandType::Zkapp)
    }

    /// Only called on zkapp commands
    pub fn accounts_updated(&self) -> Vec<(PublicKey, TokenAddress, i64, bool)> {
        let mut updated = vec![];

        if let SignedCommand::V2(UserCommandData::ZkappCommandData(data)) = &self.command {
            for update in &data.account_updates {
                let pk = update.elt.account_update.body.public_key.to_owned();
                let token = update.elt.account_update.body.token_id.to_owned();
                let balance_change: i64 = (&update.elt.account_update.body.balance_change).into();
                let increment_nonce = update.elt.account_update.body.increment_nonce;

                updated.push((pk, token, balance_change, increment_nonce));
                recurse_calls_accounts_updated(&mut updated, update.elt.calls.iter());
            }
        }

        updated
    }

    /// Only called on zkapp commands
    pub fn actions(&self) -> Vec<String> {
        let mut actions = vec![];

        if let SignedCommand::V2(UserCommandData::ZkappCommandData(data)) = &self.command {
            for update in &data.account_updates {
                let mut update_actions: Vec<_> = update
                    .elt
                    .account_update
                    .body
                    .actions
                    .iter()
                    .flat_map(|actions| actions.0.to_owned())
                    .collect();

                actions.append(&mut update_actions);
                recurse_calls_actions(&mut actions, update.elt.calls.iter());
            }
        }

        actions
    }

    /// Only called on zkapp commands
    pub fn events(&self) -> Vec<String> {
        let mut events = vec![];

        if let SignedCommand::V2(UserCommandData::ZkappCommandData(data)) = &self.command {
            for update in &data.account_updates {
                let mut update_events: Vec<_> = update
                    .elt
                    .account_update
                    .body
                    .events
                    .iter()
                    .flat_map(|events| events.0.to_owned())
                    .collect();

                events.append(&mut update_events);
                recurse_calls_events(&mut events, update.elt.calls.iter());
            }
        }

        events
    }

    pub fn from(
        user_cmd: UserCommandWithStatus,
        state_hash: &str,
        blockchain_length: u32,
        date_time: u64,
        global_slot_since_genesis: u32,
    ) -> Self {
        let status = user_cmd.status_data();
        let command = SignedCommand::from(user_cmd);

        Self {
            status,
            date_time,
            blockchain_length,
            global_slot_since_genesis,
            nonce: command.nonce(),
            state_hash: state_hash.into(),
            tx_hash: command
                .hash_signed_command()
                .expect("valid transaction hash"),
            command,
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .commands()
            .into_iter()
            .map(|cmd| {
                Self::from(
                    cmd,
                    &block.state_hash().0,
                    block.blockchain_length(),
                    block.timestamp(),
                    block.global_slot_since_genesis(),
                )
            })
            .collect()
    }
}

/////////////////
// Conversions //
/////////////////

impl From<(UserCommand, bool)> for SignedCommandWithCreationData {
    fn from(value: (UserCommand, bool)) -> Self {
        match value.0 {
            UserCommand::SignedCommand(signed_command) => Self {
                signed_command,
                is_new_receiver_account: value.1,
            },
            UserCommand::ZkappCommand(_) => unreachable!(),
        }
    }
}

impl From<mina_rs::UserCommandWithStatus1> for SignedCommand {
    fn from(value: mina_rs::UserCommandWithStatus1) -> Self {
        Self::from_user_command(value.into())
    }
}

impl From<UserCommandWithStatus> for SignedCommand {
    fn from(value: UserCommandWithStatus) -> Self {
        match value {
            UserCommandWithStatus::V1(v1) => v1.t.into(),
            UserCommandWithStatus::V2(v2) => Self::V2(v2.data.1),
        }
    }
}

impl From<SignedCommandWithCreationData> for Command {
    fn from(value: SignedCommandWithCreationData) -> Command {
        let signed = value.signed_command;
        match &signed {
            SignedCommand::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(payment_payload_v1) => {
                        let mina_rs::PaymentPayload1 {
                            source_pk,
                            receiver_pk,
                            amount,
                            ..
                        } = &payment_payload_v1.t.t;
                        Command::Payment(Payment {
                            source: source_pk.to_owned().into(),
                            receiver: receiver_pk.to_owned().into(),
                            amount: amount.t.t.into(),
                            nonce: signed.nonce(),
                            is_new_receiver_account: value.is_new_receiver_account,
                        })
                    }
                    StakeDelegation(stake_delegation_v1) => {
                        let mina_rs::StakeDelegation1::SetDelegate {
                            delegator,
                            new_delegate,
                        } = stake_delegation_v1.t.to_owned();
                        Command::Delegation(Delegation {
                            delegate: new_delegate.into(),
                            delegator: delegator.into(),
                            nonce: signed.nonce(),
                        })
                    }
                }
            }
            SignedCommand::V2(v2) => {
                use v2::staged_ledger_diff::{SignedCommandPayloadBody, StakeDelegationPayload};
                match v2 {
                    UserCommandData::SignedCommandData(data) => match &data.payload.body.1 {
                        SignedCommandPayloadBody::Payment(_) => Command::Payment(Payment {
                            nonce: signed.nonce(),
                            source: signed.fee_payer_pk(),
                            amount: signed.amount().into(),
                            receiver: signed.receiver_pk().first().expect("receiver").to_owned(),
                            is_new_receiver_account: value.is_new_receiver_account,
                        }),
                        SignedCommandPayloadBody::StakeDelegation((
                            _,
                            StakeDelegationPayload { new_delegate },
                        )) => Command::Delegation(Delegation {
                            nonce: signed.nonce(),
                            delegator: signed.source_pk(),
                            delegate: new_delegate.to_owned(),
                        }),
                    },
                    UserCommandData::ZkappCommandData(data) => Command::Zkapp(data.to_owned()),
                }
            }
        }
    }
}

impl From<SignedCommandWithStateHash> for SignedCommand {
    fn from(value: SignedCommandWithStateHash) -> Self {
        value.command
    }
}

impl From<SignedCommandWithStateHash> for Command {
    fn from(value: SignedCommandWithStateHash) -> Self {
        SignedCommandWithCreationData {
            signed_command: value.command,
            is_new_receiver_account: value.is_new_receiver_account,
        }
        .into()
    }
}

impl From<SignedCommandWithStateHash> for CommandWithStateHash {
    fn from(value: SignedCommandWithStateHash) -> Self {
        Self {
            state_hash: value.state_hash.clone(),
            command: value.into(),
        }
    }
}

impl From<mina_rs::SignedCommand1> for SignedCommand {
    fn from(value: mina_rs::SignedCommand1) -> Self {
        Self::V1(Box::new(Versioned::new(Versioned::new(value))))
    }
}

impl From<SignedCommand> for serde_json::Value {
    fn from(value: SignedCommand) -> Self {
        match value {
            SignedCommand::V1(v1) => {
                let mut json = serde_json::Map::new();

                json.insert("payload".into(), payload_json_v1(&v1.t.t));
                json.insert("signer".into(), signer_v1(&v1.t.t));
                json.insert("signature".into(), signature_v1(&v1.t.t));

                serde_json::Value::Object(json)
            }
            SignedCommand::V2(UserCommandData::SignedCommandData(v2)) => {
                let mut json = serde_json::Map::new();

                json.insert("payload".into(), payload_json_v2(&v2));
                json.insert("signer".into(), signer_v2(&v2));
                json.insert("signature".into(), signature_v2(&v2));

                serde_json::Value::Object(json)
            }
            SignedCommand::V2(UserCommandData::ZkappCommandData(data)) => to_zkapp_json(&data),
        }
    }
}

pub struct SignedCommandWithKind(SignedCommand);

impl From<UserCommandWithStatus> for SignedCommandWithKind {
    fn from(value: UserCommandWithStatus) -> Self {
        Self(value.into())
    }
}

impl From<SignedCommandWithKind> for serde_json::Value {
    fn from(value: SignedCommandWithKind) -> Self {
        use serde_json::*;

        if let Value::Object(mut obj) = value.0.into() {
            obj.insert("kind".into(), Value::String("Signed_command".into()));
            Value::Object(obj)
        } else {
            Value::Null
        }
    }
}

impl From<SignedCommandWithData> for serde_json::Value {
    fn from(value: SignedCommandWithData) -> Self {
        use serde_json::*;

        let mut obj = Map::new();
        let tx_hash = Value::String(value.tx_hash.inner());
        let state_hash = Value::String(value.state_hash.0);
        let command = value.command.into();
        let status = value.status.into();
        let blockchain_length = value.blockchain_length.into();

        obj.insert("tx_hash".into(), tx_hash);
        obj.insert("command".into(), command);
        obj.insert("status".into(), status);
        obj.insert("state_hash".into(), state_hash);
        obj.insert("blockchain_length".into(), blockchain_length);

        Value::Object(obj)
    }
}

impl From<SignedCommandWithData> for SignedCommand {
    fn from(value: SignedCommandWithData) -> Self {
        value.command
    }
}

impl From<SignedCommandWithData> for Command {
    fn from(value: SignedCommandWithData) -> Self {
        SignedCommandWithCreationData {
            signed_command: value.command,
            is_new_receiver_account: value.status.receiver_account_creation_fee_paid().is_some(),
        }
        .into()
    }
}

impl std::fmt::Debug for SignedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let json: Value = self.clone().into();
        write!(f, "{}", to_string_pretty(&json).unwrap())
    }
}

impl std::fmt::Debug for SignedCommandWithData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let json: Value = self.clone().into();
        write!(f, "{}", to_string_pretty(&json).unwrap())
    }
}

impl std::fmt::Debug for SignedCommandWithStateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let mut json = Map::new();
        json.insert("command".into(), self.command.clone().into());
        json.insert(
            "state_hash".into(),
            Value::String(self.state_hash.0.clone()),
        );
        write!(f, "{}", to_string_pretty(&json).unwrap())
    }
}

fn signer_v1(value: &mina_rs::SignedCommand1) -> serde_json::Value {
    let pk: PublicKey = value.signer.0.t.to_owned().into();
    serde_json::Value::String(pk.0)
}

fn signer_v2(value: &v2::staged_ledger_diff::SignedCommandData) -> serde_json::Value {
    let pk = value.signer.0.to_owned();
    serde_json::Value::String(pk)
}

fn signature_v1(value: &mina_rs::SignedCommand1) -> serde_json::Value {
    let sig: Signature = value.signature.to_owned().into();
    serde_json::Value::String(sig.to_string())
}

fn signature_v2(value: &v2::staged_ledger_diff::SignedCommandData) -> serde_json::Value {
    serde_json::Value::String(value.signature.to_owned())
}

fn payload_json_v1(value: &mina_rs::SignedCommand1) -> serde_json::Value {
    use serde_json::*;

    let mut payload_obj = Map::new();
    let mina_rs::SignedCommand1 { ref payload, .. } = value;

    let mut common = Map::new();
    let mina_rs::SignedCommandPayloadCommon1 {
        fee,
        fee_token,
        fee_payer_pk,
        nonce,
        valid_until,
        memo,
    } = &payload.t.t.common.t.t.t;

    common.insert("fee".into(), Value::Number(Number::from(fee.t.t)));
    common.insert(
        "fee_token".into(),
        Value::Number(Number::from(fee_token.t.t.t)),
    );
    common.insert(
        "fee_payer_pk".into(),
        Value::String(PublicKey::from(fee_payer_pk.to_owned()).to_address()),
    );
    common.insert("nonce".into(), Value::Number(Number::from(nonce.t.t)));
    common.insert(
        "valid_until".into(),
        Value::Number(Number::from(valid_until.t.t as u32)),
    );
    common.insert("memo".into(), Value::String(decode_memo(&memo.t.0, true)));

    use mina_rs::SignedCommandPayloadBody1::*;
    let body = match &payload.t.t.body.t.t {
        PaymentPayload(payment_payload) => {
            let mut body_obj = Map::new();
            let mina_rs::PaymentPayload1 {
                source_pk,
                receiver_pk,
                token_id,
                amount,
            } = &payment_payload.t.t;

            body_obj.insert(
                "source_pk".into(),
                Value::String(PublicKey::from(source_pk.to_owned()).to_address()),
            );
            body_obj.insert(
                "receiver_pk".into(),
                Value::String(PublicKey::from(receiver_pk.to_owned()).to_address()),
            );
            body_obj.insert(
                "token_id".into(),
                Value::Number(Number::from(token_id.t.t.t)),
            );
            body_obj.insert("amount".into(), Value::Number(Number::from(amount.t.t)));
            body_obj.insert("kind".into(), Value::String("Payment".into()));

            Value::Object(body_obj)
        }
        StakeDelegation(stake_delegation) => {
            let mut body_obj = Map::new();
            let mina_rs::StakeDelegation1::SetDelegate {
                delegator,
                new_delegate,
            } = stake_delegation.t.to_owned();

            body_obj.insert(
                "delegator".into(),
                Value::String(PublicKey::from(delegator).to_address()),
            );
            body_obj.insert(
                "new_delegate".into(),
                Value::String(PublicKey::from(new_delegate).to_address()),
            );
            body_obj.insert("kind".into(), Value::String("Stake_delegation".into()));

            Value::Object(body_obj)
        }
    };

    payload_obj.insert("common".into(), Value::Object(common));
    payload_obj.insert("body".into(), body);
    Value::Object(payload_obj)
}

fn payload_json_v2(value: &v2::staged_ledger_diff::SignedCommandData) -> serde_json::Value {
    use serde_json::*;
    use v2::staged_ledger_diff::SignedCommandPayloadCommon;

    let mut payload_obj = Map::new();
    let SignedCommandData { ref payload, .. } = value;

    let mut common = Map::new();
    let SignedCommandPayloadCommon {
        fee,
        fee_payer_pk,
        nonce,
        valid_until,
        memo,
    } = &payload.common;

    common.insert("fee".into(), Value::Number(fee.0.into()));
    common.insert(
        "fee_payer_pk".into(),
        Value::String(fee_payer_pk.to_owned().to_address()),
    );
    common.insert("nonce".into(), Value::Number(nonce.0.into()));
    common.insert("valid_until".into(), Value::Number(valid_until.0.into()));
    common.insert(
        "memo".into(),
        Value::String(decode_memo(memo.as_bytes(), false)),
    );

    use v2::staged_ledger_diff::{SignedCommandPayloadBody::*, *};
    let body = match &payload.body.1 {
        Payment(PaymentPayload {
            amount,
            receiver_pk,
        }) => {
            let mut body_obj = Map::new();

            body_obj.insert(
                "receiver_pk".into(),
                Value::String(receiver_pk.to_owned().to_address()),
            );
            body_obj.insert("amount".into(), Value::Number(amount.0.into()));
            body_obj.insert("kind".into(), Value::String("Payment".into()));

            Value::Object(body_obj)
        }
        StakeDelegation((_, StakeDelegationPayload { new_delegate })) => {
            let mut body_obj = Map::new();

            let mut set_delegate = Map::new();
            set_delegate.insert(
                "new_delegate".into(),
                Value::String(new_delegate.to_owned().to_address()),
            );

            body_obj.insert("kind".into(), Value::String("Stake_delegation".into()));
            body_obj.insert("Set_delegate".into(), Value::Object(set_delegate));

            Value::Object(body_obj)
        }
    };

    payload_obj.insert("common".into(), Value::Object(common));
    payload_obj.insert("body".into(), body);

    Value::Object(payload_obj)
}

impl From<String> for TxnHash {
    fn from(value: String) -> Self {
        Self::new(value).expect("transaction hash")
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for TxnHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ref_inner())
    }
}

/////////////
// helpers //
/////////////

fn recurse_calls_accounts_updated<'a>(
    accounts: &mut Vec<(PublicKey, TokenAddress, i64, bool)>,
    calls: impl Iterator<Item = &'a Call>,
) {
    for update in calls {
        let pk = update.elt.account_update.body.public_key.to_owned();
        let token = update.elt.account_update.body.token_id.to_owned();
        let balance_change: i64 = (&update.elt.account_update.body.balance_change).into();
        let increment_nonce = update.elt.account_update.body.increment_nonce;

        accounts.push((pk, token, balance_change, increment_nonce));
        recurse_calls_accounts_updated(accounts, update.elt.calls.iter());
    }
}

fn recurse_calls_actions<'a>(actions: &mut Vec<String>, calls: impl Iterator<Item = &'a Call>) {
    for update in calls {
        let mut update_actions: Vec<_> = update
            .elt
            .account_update
            .body
            .actions
            .iter()
            .flat_map(|actions| actions.0.to_owned())
            .collect();

        actions.append(&mut update_actions);
        recurse_calls_actions(actions, update.elt.calls.iter());
    }
}

fn recurse_calls_events<'a>(events: &mut Vec<String>, calls: impl Iterator<Item = &'a Call>) {
    for update in calls {
        let mut update_events: Vec<_> = update
            .elt
            .account_update
            .body
            .events
            .iter()
            .flat_map(|events| events.0.to_owned())
            .collect();

        events.append(&mut update_events);
        recurse_calls_events(events, update.elt.calls.iter());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::path::PathBuf;

    #[test]
    fn txn_hash_v1() -> Result<()> {
        // refer to the hashes on Minascan
        // https://minascan.io/mainnet/tx/CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj
        // https://minascan.io/mainnet/tx/CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX

        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V1).unwrap();
        let hashes = precomputed_block.command_hashes();
        let expect = vec![
            TxnHash::V1("CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX".to_string()),
            TxnHash::V1("CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj".to_string()),
        ];

        assert_eq!(hashes, expect);
        Ok(())
    }

    #[test]
    fn txn_hash_signed_command_v2() -> Result<()> {
        let block_file = PathBuf::from("./tests/data/hardfork/mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let hashes = precomputed_block.command_hashes();

        // see https://minaexplorer.com/block/3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg
        assert_eq!(
            hashes,
            vec![TxnHash::V2(
                "5JtZ6p9SepL8GeAyjjFVj1JCQUAvSTHwzUQe9fxbNtqkA7X8ZAzf".to_string()
            )]
        );
        Ok(())
    }

    #[test]
    fn txn_hash_zkapp_command() -> Result<()> {
        let block_file = PathBuf::from("./tests/data/misc_blocks/mainnet-397612-3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let hashes = precomputed_block
            .commands()
            .into_iter()
            .filter_map(|cmd| {
                if !cmd.is_zkapp_command() {
                    // filter out non-zkapp commands
                    return None;
                }

                let cmd: SignedCommand = cmd.into();
                Some(cmd.hash_signed_command().unwrap())
            })
            .collect::<Vec<_>>();

        // see https://minaexplorer.com/block/3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK
        assert_eq!(
            hashes,
            vec![
                TxnHash::V2("5JtXBs7Xtf2QDgTmd6sdkWPLNxPXT9eu2tBWgihjB35DUZ4o1SwR".to_string()),
                TxnHash::V2("5JupfF1RRVYXdEW1KGZXad11ugerCagr2K8qjBFq692MRbuDeJCN".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    fn signed_command_json_v1() -> Result<()> {
        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V1)?;
        let signed_cmds = precomputed_block
            .commands()
            .into_iter()
            .map(|c| format!("{:?}", SignedCommand::from(c)))
            .collect::<Vec<_>>();

        let expect0 = r#"{
  "payload": {
    "body": {
      "amount": 60068000,
      "kind": "Payment",
      "receiver_pk": "B62qmbBg93wtMp1yN42nN7SuunWWNpVbBwiusvhqbxJ2yt5QonEKzVY",
      "source_pk": "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
      "token_id": 1
    },
    "common": {
      "fee": 10000000,
      "fee_payer_pk": "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
      "fee_token": 1,
      "memo": "FPayment",
      "nonce": 7295,
      "valid_until": 4294967295
    }
  },
  "signature": "27688b6b9c23dda2681fe1f09e813110f1600462e13da63515519967db316a433be34e62db7b7fd71c7c6f72b32e33f02c1d985a35d9bbfeca9387993e2006df",
  "signer": "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP"
}"#;
        let expect1 = r#"{
  "payload": {
    "body": {
      "amount": 1000,
      "kind": "Payment",
      "receiver_pk": "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
      "source_pk": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
      "token_id": 1
    },
    "common": {
      "fee": 1000000,
      "fee_payer_pk": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
      "fee_token": 1,
      "memo": "",
      "nonce": 146491,
      "valid_until": 4294967295
    }
  },
  "signature": "313aeadfa061ef68c3fdffe79e8c5dfd6e5167bd2ea9a240be8d04e29331468e23cf18712887c903844e4c1a827a77dccdbabd59e1698b3a0b33d76f8ae3861c",
  "signer": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
}"#;

        assert_eq!(signed_cmds, vec![expect0, expect1]);
        Ok(())
    }

    #[test]
    fn signed_command_json_v2() -> Result<()> {
        let block_file = PathBuf::from("./tests/data/hardfork/mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2)?;
        let signed_cmds = precomputed_block
            .commands()
            .into_iter()
            .map(|c| format!("{:?}", SignedCommand::from(c)))
            .collect::<Vec<_>>();

        for cmd in &signed_cmds {
            println!("{cmd}");
        }

        let expect0 = r#"{
  "payload": {
    "body": {
      "amount": 1000000000,
      "kind": "Payment",
      "receiver_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
    },
    "common": {
      "fee": 1100000,
      "fee_payer_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
      "memo": "",
      "nonce": 765,
      "valid_until": 4294967295
    }
  },
  "signature": "7mX5FyaaoRY5a3hKP3kqhm6A4gWo9NtoHMh7irbB3Dt326wm8gyfsEQeHKJgYqQeo7nBgFGNjCD9eC265VrECYZJqYsD5V5R",
  "signer": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
}"#;

        assert_eq!(signed_cmds, vec![expect0]);
        Ok(())
    }

    #[test]
    fn tokens() -> Result<()> {
        let path = PathBuf::from("./tests/data/hardfork/mainnet-359617-3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        assert_eq!(block.zkapp_commands().len(), 7);

        assert_eq!(
            block
                .zkapp_commands()
                .into_iter()
                .map(SignedCommand::from)
                .map(|cmd| cmd.tokens())
                .collect::<Vec<_>>(),
            vec![
                vec![TokenAddress::default()],
                vec![TokenAddress::default()],
                vec![TokenAddress::default()],
                vec![TokenAddress::default()],
                vec![TokenAddress::default()],
                vec![TokenAddress::default()],
                vec![
                    TokenAddress::default(),
                    TokenAddress::new("wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd")
                        .unwrap()
                ]
            ]
        );

        Ok(())
    }

    #[test]
    fn account_updates() -> Result<()> {
        let path = PathBuf::from("./tests/data/hardfork/mainnet-359617-3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        assert_eq!(
            block
                .zkapp_commands()
                .first()
                .cloned()
                .map(|cmd| SignedCommandWithData::from(
                    cmd,
                    "3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW",
                    359617,
                    1717548852749,
                    564498
                ))
                .map(|cmd| cmd.accounts_updated()),
            Some(vec![
                (
                    "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    TokenAddress::new("wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf")
                        .unwrap(),
                    -2000000000,
                    false,
                ),
                (
                    "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    TokenAddress::new("wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf")
                        .unwrap(),
                    2000000000,
                    false,
                ),
            ])
        );

        Ok(())
    }

    #[test]
    fn actions() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-412598-3NK6LSkCCBNoHmiRfYhYijDuxwgYQsU5GcdEMbUGhNdHDkJyrh3x.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        assert_eq!(
            block
                .zkapp_commands()
                .into_iter()
                .map(|cmd| SignedCommandWithData::from(
                    cmd,
                    "3NK6LSkCCBNoHmiRfYhYijDuxwgYQsU5GcdEMbUGhNdHDkJyrh3x",
                    412598,
                    1734508101031,
                    658716
                ))
                .flat_map(|cmd| cmd.actions())
                .collect::<Vec<_>>(),
            vec![
                "0x0000000000000000000000000000000000000000000000000000000000000001",
                "0x0000000000000000000000000000000000000000000000000000000000000003",
                "0x0000000000000000000000000000000000000000000000000000000000000006",
                "0x0000000000000000000000000000000000000000000000000000000000000006",
                "0x0000000000000000000000000000000000000000000000000000000000000003",
                "0x0000000000000000000000000000000000000000000000000000000000000005",
                "0x1CA565AB7F42B8B8BC414FA7D9C58050F394BDCF0B12E29F537787264CC7BF42",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x0000000000000000000000000000000000000000000000000000000000000001"
            ]
        );

        Ok(())
    }

    #[test]
    fn events() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-412598-3NK6LSkCCBNoHmiRfYhYijDuxwgYQsU5GcdEMbUGhNdHDkJyrh3x.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        assert_eq!(
            block
                .zkapp_commands()
                .into_iter()
                .map(|cmd| SignedCommandWithData::from(
                    cmd,
                    "3NK6LSkCCBNoHmiRfYhYijDuxwgYQsU5GcdEMbUGhNdHDkJyrh3x",
                    412598,
                    1734508101031,
                    658716
                ))
                .flat_map(|cmd| cmd.events())
                .collect::<Vec<_>>(),
            vec![
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x0000000000000000000000000000000000000000000000000000000000000001",
                "0x0000000000000000000000000000000000000000000000000000000000000003",
                "0x0000000000000000000000000000000000000000000000000000000000000006",
                "0x0000000000000000000000000000000000000000000000000000000000000006",
                "0x0000000000000000000000000000000000000000000000000000000000000003",
                "0x0000000000000000000000000000000000000000000000000000000000000005",
                "0x1CA565AB7F42B8B8BC414FA7D9C58050F394BDCF0B12E29F537787264CC7BF42",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x0000000000000000000000000000000000000000000000000000000000000001"
            ]
        );

        Ok(())
    }
}
