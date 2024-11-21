mod txn_hash;

use crate::{
    command::*,
    mina_blocks::v2::{self, staged_ledger_diff::CommandData},
    proof_systems::signer::signature::Signature,
    protocol::{
        bin_prot,
        serialization_types::{
            signatures::PublicKey2V1,
            staged_ledger_diff as mina_rs,
            version_bytes::{USER_COMMAND, V1_TXN_HASH, V2_TXN_HASH},
        },
    },
};
use blake2::digest::VariableOutput;
use serde::{Deserialize, Serialize};
use std::io::Write;

// re-export [txn_hash::TxnHash]
pub type TxnHash = txn_hash::TxnHash;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum SignedCommand {
    V1(mina_rs::SignedCommandV1),
    V2(mina_rs::SignedCommandV2),
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithCreationData {
    pub signed_command: SignedCommand,
    pub is_new_receiver_account: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithStateHash {
    pub command: SignedCommand,
    pub state_hash: BlockHash,
    pub is_new_receiver_account: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithData {
    pub command: SignedCommand,
    pub state_hash: BlockHash,
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
            Self::V2(v2) => v2.t.t.payload.t.t.common.t.t.t.fee.t.t,
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
            Self::V2(v2) => {
                v2.t.t
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
        }
    }

    pub fn nonce(&self) -> Nonce {
        let nonce = match self {
            Self::V1(v1) => v1.t.t.payload.t.t.common.t.t.t.nonce.t.t as u32,
            Self::V2(v2) => v2.t.t.payload.t.t.common.t.t.t.nonce.t.t as u32,
        };
        Nonce(nonce)
    }

    pub fn valid_until(&self) -> i32 {
        match self {
            Self::V1(v1) => v1.t.t.payload.t.t.common.t.t.t.valid_until.t.t,
            Self::V2(v2) => v2.t.t.payload.t.t.common.t.t.t.valid_until.t.t,
        }
    }

    pub fn memo(&self) -> String {
        let encoded = match self {
            Self::V1(v1) => &v1.t.t.payload.t.t.common.t.t.t.memo.t.0,
            Self::V2(v2) => &v2.t.t.payload.t.t.common.t.t.t.memo.t.0,
        };
        decode_memo(encoded)
    }

    pub fn fee_token(&self) -> Option<u64> {
        match self {
            Self::V1(v1) => Some(v1.t.t.payload.t.t.common.t.t.t.fee_token.t.t.t),
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
            Self::V2(v2) => {
                use mina_rs::SignedCommandPayloadBody2::*;
                match &v2.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v2) => v2.t.t.amount.t.t,
                    StakeDelegation(_) => 0,
                }
            }
        }
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v1) => v1.t.t.receiver_pk.to_owned().into(),
                    StakeDelegation(v1) => match v1.t {
                        mina_rs::StakeDelegation::SetDelegate {
                            ref new_delegate, ..
                        } => new_delegate.to_owned().into(),
                    },
                }
            }
            Self::V2(v2) => {
                use mina_rs::SignedCommandPayloadBody2::*;
                match &v2.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v2) => v2.t.t.receiver_pk.to_owned().into(),
                    StakeDelegation(v2) => match v2.t {
                        mina_rs::StakeDelegation::SetDelegate {
                            ref new_delegate, ..
                        } => new_delegate.to_owned().into(),
                    },
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
                        mina_rs::StakeDelegation::SetDelegate {
                            ref delegator,
                            new_delegate: _,
                        } => delegator.to_owned().into(),
                    },
                }
            }
            Self::V2(_v2) => self.fee_payer_pk(),
        }
    }

    pub fn token_id(&self) -> Option<u64> {
        match self {
            Self::V1(v1) => {
                use mina_rs::SignedCommandPayloadBody1::*;
                match &v1.t.t.payload.t.t.body.t.t {
                    PaymentPayload(v1) => Some(v1.t.t.token_id.t.t.t),
                    StakeDelegation(_v1) => None,
                }
            }
            Self::V2(_v2) => None,
        }
    }

    // other data

    pub fn signer(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1.t.t.signer.0.t.to_owned().into(),
            Self::V2(v2) => v2.t.t.signer.0.t.to_owned().into(),
        }
    }

    pub fn all_command_public_keys(&self) -> Vec<PublicKey> {
        vec![
            self.receiver_pk(),
            self.source_pk(),
            self.fee_payer_pk(),
            self.signer(),
        ]
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
            Self::V2(v2) => {
                use mina_rs::SignedCommandPayloadBody2::*;
                match &v2.t.t.payload.t.t.body.t.t {
                    PaymentPayload(_) => CommandType::Payment,
                    StakeDelegation(_) => CommandType::Delegation,
                }
            }
        }
    }

    pub fn is_delegation(&self) -> bool {
        matches!(self.kind(), CommandType::Delegation)
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc {
            UserCommandWithStatus::V1(v1) => match v1.t.data.t.t {
                mina_rs::UserCommand1::SignedCommand(v1) => Self::V1(v1),
            },
            UserCommandWithStatus::V2(v2) => match v2.t.data.t.t {
                mina_rs::UserCommand2::SignedCommand(v2) => Self::V2(v2),
                mina_rs::UserCommand2::ZkappCommand(_zkapp) => {
                    panic!("zkapp cannot be converted to signed command")
                }
            },
        }
    }

    /// Returns a user command (transaction) hash
    pub fn hash_signed_command(&self) -> anyhow::Result<TxnHash> {
        match self {
            Self::V1(v1) => {
                let mut binprot_bytes = Vec::with_capacity(TxnHash::V1_LEN * 8); // max number of bits
                bin_prot::to_writer(&mut binprot_bytes, v1)?;

                let binprot_bytes_bs58 = bs58::encode(&binprot_bytes[..])
                    .with_check_version(USER_COMMAND)
                    .into_string();
                let mut hasher = blake2::Blake2bVar::new(32)?;
                hasher.write_all(binprot_bytes_bs58.as_bytes())?;

                let mut hash = hasher.finalize_boxed().to_vec();
                hash.insert(0, hash.len() as u8);
                hash.insert(0, 1); //version byte

                Ok(TxnHash::V1(
                    bs58::encode(hash)
                        .with_check_version(V1_TXN_HASH)
                        .into_string(),
                ))
            }
            Self::V2(v2) => {
                let mut binprot_bytes = Vec::with_capacity(TxnHash::V2_LEN * 8); // max number of bits
                bin_prot::to_writer(&mut binprot_bytes, v2)?;

                let binprot_bytes_bs58 = bs58::encode(&binprot_bytes[..])
                    .with_check_version(USER_COMMAND)
                    .into_string();
                let mut hasher = blake2::Blake2bVar::new(32)?;
                hasher.write_all(binprot_bytes_bs58.as_bytes())?;

                let mut hash = hasher.finalize_boxed().to_vec();
                hash.insert(0, hash.len() as u8);

                Ok(TxnHash::V2(
                    bs58::encode(hash)
                        .with_check_version(V2_TXN_HASH)
                        .into_string(),
                ))
            }
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
    pub fn from(
        user_cmd: &UserCommandWithStatus,
        state_hash: &str,
        blockchain_length: u32,
        date_time: u64,
        global_slot_since_genesis: u32,
    ) -> Self {
        let command = SignedCommand::from(user_cmd.clone());
        Self {
            date_time,
            blockchain_length,
            global_slot_since_genesis,
            nonce: command.nonce(),
            state_hash: state_hash.into(),
            status: user_cmd.status_data(),
            tx_hash: command
                .hash_signed_command()
                .expect("valid transaction hash"),
            command,
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .commands()
            .iter()
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

impl From<CommandData> for Option<SignedCommand> {
    fn from(value: CommandData) -> Self {
        match value {
            CommandData::UserCommandData(ucd) => Some(SignedCommand::V2(Versioned::new(
                Versioned::new(ucd.as_ref().to_owned().into()),
            ))),
            CommandData::ZkappCommandData(_) => None,
        }
    }
}

impl From<v2::staged_ledger_diff::UserCommandData> for mina_rs::SignedCommand2 {
    fn from(value: v2::staged_ledger_diff::UserCommandData) -> Self {
        Self {
            payload: Versioned::new(Versioned::new(value.payload.into())),
            signer: PublicKey2V1(Versioned::new(value.signer.into())),
            signature: value.signature.into(),
        }
    }
}

impl From<v2::staged_ledger_diff::UserCommandPayload> for mina_rs::SignedCommandPayload2 {
    fn from(value: v2::staged_ledger_diff::UserCommandPayload) -> Self {
        let fee_payer = value.common.fee_payer_pk.to_owned();
        Self {
            common: Versioned::new(Versioned::new(Versioned::new(value.common.into()))),
            body: Versioned::new(Versioned::new((value.body.1, fee_payer).into())),
        }
    }
}

impl From<v2::staged_ledger_diff::UserCommandPayloadCommon>
    for mina_rs::SignedCommandPayloadCommon2
{
    fn from(value: v2::staged_ledger_diff::UserCommandPayloadCommon) -> Self {
        Self {
            fee: Versioned::new(Versioned::new(value.fee)),
            fee_payer_pk: value.fee_payer_pk.into(),
            nonce: Versioned::new(Versioned::new(value.nonce as i32)),
            valid_until: Versioned::new(Versioned::new(value.valid_until as i32)),
            memo: Versioned::new(value.memo.into()),
        }
    }
}

impl From<(v2::staged_ledger_diff::UserCommandPayloadBody, PublicKey)>
    for mina_rs::SignedCommandPayloadBody2
{
    fn from(value: (v2::staged_ledger_diff::UserCommandPayloadBody, PublicKey)) -> Self {
        match value.0 {
            v2::staged_ledger_diff::UserCommandPayloadBody::Payment(payload) => {
                Self::PaymentPayload(Versioned::new(Versioned::new(mina_rs::PaymentPayload2 {
                    receiver_pk: payload.receiver_pk.into(),
                    amount: Versioned::new(Versioned::new(payload.amount)),
                })))
            }
            v2::staged_ledger_diff::UserCommandPayloadBody::StakeDelegation(payload) => {
                Self::StakeDelegation(Versioned::new(mina_rs::StakeDelegation::SetDelegate {
                    delegator: value.1.into(),
                    new_delegate: payload.new_delegate.into(),
                }))
            }
        }
    }
}

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

impl From<mina_rs::UserCommandWithStatus2> for SignedCommand {
    fn from(value: mina_rs::UserCommandWithStatus2) -> Self {
        Self::from_user_command(value.into())
    }
}

impl From<UserCommandWithStatus> for SignedCommand {
    fn from(value: UserCommandWithStatus) -> Self {
        match value {
            UserCommandWithStatus::V1(v1) => v1.t.to_owned().into(),
            UserCommandWithStatus::V2(v2) => v2.t.to_owned().into(),
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
                        let mina_rs::StakeDelegation::SetDelegate {
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
                use mina_rs::SignedCommandPayloadBody2::*;
                match &v2.t.t.payload.t.t.body.t.t {
                    PaymentPayload(payment_payload_v2) => {
                        let mina_rs::PaymentPayload2 {
                            receiver_pk,
                            amount,
                        } = &payment_payload_v2.t.t;
                        Command::Payment(Payment {
                            source: signed.fee_payer_pk(),
                            receiver: receiver_pk.to_owned().into(),
                            amount: amount.t.t.into(),
                            nonce: signed.nonce(),
                            is_new_receiver_account: value.is_new_receiver_account,
                        })
                    }
                    StakeDelegation(stake_delegation_v2) => {
                        let mina_rs::StakeDelegation::SetDelegate {
                            delegator,
                            new_delegate,
                        } = stake_delegation_v2.t.to_owned();
                        Command::Delegation(Delegation {
                            delegate: new_delegate.into(),
                            delegator: delegator.into(),
                            nonce: signed.nonce(),
                        })
                    }
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
        Self::V1(Versioned::new(Versioned::new(value)))
    }
}

impl From<mina_rs::SignedCommand2> for SignedCommand {
    fn from(value: mina_rs::SignedCommand2) -> Self {
        Self::V2(Versioned {
            t: Versioned::new(value),
            version: 2,
        })
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
            SignedCommand::V2(v2) => {
                let mut json = serde_json::Map::new();

                json.insert("payload".into(), payload_json_v2(&v2.t.t));
                json.insert("signer".into(), signer_v2(&v2.t.t));
                json.insert("signature".into(), signature_v2(&v2.t.t));

                serde_json::Value::Object(json)
            }
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

fn signer_v2(value: &mina_rs::SignedCommand2) -> serde_json::Value {
    let pk: PublicKey = value.signer.0.t.to_owned().into();
    serde_json::Value::String(pk.0)
}

fn signature_v1(value: &mina_rs::SignedCommand1) -> serde_json::Value {
    let sig: Signature = value.signature.to_owned().into();
    serde_json::Value::String(sig.to_string())
}

fn signature_v2(value: &mina_rs::SignedCommand2) -> serde_json::Value {
    let sig: Signature = value.signature.to_owned().into();
    serde_json::Value::String(sig.to_string())
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
    common.insert("memo".into(), Value::String(decode_memo(&memo.t.0)));

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
            let mina_rs::StakeDelegation::SetDelegate {
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

fn payload_json_v2(value: &mina_rs::SignedCommand2) -> serde_json::Value {
    use serde_json::*;

    let mut payload_obj = Map::new();
    let mina_rs::SignedCommand2 { ref payload, .. } = value;

    let mut common = Map::new();
    let mina_rs::SignedCommandPayloadCommon2 {
        fee,
        fee_payer_pk,
        nonce,
        valid_until,
        memo,
    } = &payload.t.t.common.t.t.t;

    common.insert("fee".into(), Value::Number(Number::from(fee.t.t)));
    common.insert(
        "fee_payer_pk".into(),
        Value::String(PublicKey::from(fee_payer_pk.to_owned()).to_address()),
    );
    common.insert("nonce".into(), Value::Number(Number::from(nonce.t.t)));
    common.insert(
        "valid_until".into(),
        Value::Number(Number::from(valid_until.t.t as u32)),
    );
    common.insert("memo".into(), Value::String(decode_memo(&memo.t.0)));

    use mina_rs::SignedCommandPayloadBody2::*;
    let body = match &payload.t.t.body.t.t {
        PaymentPayload(payment_payload) => {
            let mut body_obj = Map::new();
            let mina_rs::PaymentPayload2 {
                receiver_pk,
                amount,
            } = &payment_payload.t.t;

            body_obj.insert(
                "receiver_pk".into(),
                Value::String(PublicKey::from(receiver_pk.to_owned()).to_address()),
            );
            body_obj.insert("amount".into(), Value::Number(Number::from(amount.t.t)));
            body_obj.insert("kind".into(), Value::String("Payment".into()));

            Value::Object(body_obj)
        }
        StakeDelegation(stake_delegation) => {
            let mut body_obj = Map::new();
            let mina_rs::StakeDelegation::SetDelegate {
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

impl From<String> for TxnHash {
    fn from(value: String) -> Self {
        Self::new(value).expect("transaction hash")
    }
}

impl std::fmt::Display for TxnHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ref_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::path::PathBuf;

    #[test]
    fn transaction_hash_v1() -> anyhow::Result<()> {
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

    #[ignore]
    #[test]
    fn transaction_hash_v2() -> anyhow::Result<()> {
        let block_file = PathBuf::from("./tests/data/hardfork/mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let hashes = precomputed_block.command_hashes();

        // see https://minaexplorer.com/block/3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg
        assert_eq!(
            hashes,
            vec![TxnHash::V2(
                "5JuJ1eRNWdE8jSMmCDoHnAdBGhLyBnCk2gkcvkfCZ7WvrKtGuWHB".to_string()
            )]
        );
        Ok(())
    }

    #[test]
    fn signed_command_json() -> anyhow::Result<()> {
        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V1).unwrap();
        let signed_commands = precomputed_block
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

        assert_eq!(signed_commands, vec![expect0, expect1]);
        Ok(())
    }
}
