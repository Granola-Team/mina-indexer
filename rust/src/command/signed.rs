use crate::{
    command::*,
    proof_systems::signer::signature::Signature,
    protocol::{
        bin_prot,
        serialization_types::{
            staged_ledger_diff as mina_rs,
            version_bytes::{USER_COMMAND, V1_TXN_HASH},
        },
    },
};
use anyhow::bail;
use blake2::digest::VariableOutput;
use mina_serialization_versioned::Versioned2;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct TxnHash(pub String);

impl TxnHash {
    pub const LEN: usize = 53;
    pub const PREFIX: &'static str = "Ckp";

    pub fn new(txn_hash: &str) -> anyhow::Result<Self> {
        let res = TxnHash(txn_hash.to_string());
        if Self::is_valid(&res) {
            return Ok(res);
        }
        bail!("Invalid txn hash {txn_hash}")
    }

    pub fn is_valid(&self) -> bool {
        self.0.starts_with(TxnHash::PREFIX) && self.0.len() == TxnHash::LEN
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        Ok(Self(String::from_utf8(bytes)?))
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommand(pub mina_rs::SignedCommandV1);

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
    pub fn amount(&self) -> u64 {
        use mina_rs::SignedCommandPayloadBody::*;
        match self.payload_body() {
            PaymentPayload(payload) => payload.t.t.amount.t.t,
            StakeDelegation(_) => 0,
        }
    }

    pub fn memo(&self) -> String {
        decode_memo(&self.payload_common().memo.t.0)
    }

    pub fn fee(&self) -> u64 {
        self.payload_common().fee.t.t
    }

    pub fn fee_payer_pk(&self) -> PublicKey {
        self.payload_common().fee_payer_pk.to_owned().into()
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                payment_payload.t.t.receiver_pk.to_owned().into()
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                match delegation_payload.t {
                    mina_rs::StakeDelegation::SetDelegate {
                        delegator: _,
                        ref new_delegate,
                    } => new_delegate.to_owned().into(),
                }
            }
        }
    }

    pub fn source_pk(&self) -> PublicKey {
        match self.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                payment_payload.t.t.source_pk.to_owned().into()
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                match delegation_payload.t {
                    mina_rs::StakeDelegation::SetDelegate {
                        ref delegator,
                        new_delegate: _,
                    } => delegator.to_owned().into(),
                }
            }
        }
    }

    pub fn signer(&self) -> PublicKey {
        self.0.clone().inner().inner().signer.0.inner().into()
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
        match self.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(_) => CommandType::Payment,
            mina_rs::SignedCommandPayloadBody::StakeDelegation(_) => CommandType::Delegation,
        }
    }

    pub fn is_delegation(&self) -> bool {
        matches!(self.kind(), CommandType::Delegation)
    }

    pub fn payload(&self) -> &mina_rs::SignedCommandPayload {
        &self.0.t.t.payload.t.t
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc.0.inner().data.inner().inner() {
            mina_rs::UserCommand::SignedCommand(signed_command) => signed_command.into(),
        }
    }

    pub fn source_nonce(&self) -> Nonce {
        Nonce(self.payload_common().nonce.t.t as u32)
    }

    pub fn payload_body(&self) -> &mina_rs::SignedCommandPayloadBody {
        &self.payload().body.t.t
    }

    pub fn payload_common(&self) -> &mina_rs::SignedCommandPayloadCommon {
        &self.payload().common.t.t.t
    }

    /// This returns a user command (transaction) hash that starts with
    /// [TxnHash::PREFIX]
    pub fn hash_signed_command(&self) -> anyhow::Result<TxnHash> {
        let mut binprot_bytes = Vec::new();
        bin_prot::to_writer(&mut binprot_bytes, &self.0).map_err(anyhow::Error::from)?;

        let binprot_bytes_bs58 = bs58::encode(&binprot_bytes[..])
            .with_check_version(USER_COMMAND)
            .into_string();
        let mut hasher = blake2::Blake2bVar::new(32).unwrap();
        hasher.write_all(binprot_bytes_bs58.as_bytes()).unwrap();

        let mut hash = hasher.finalize_boxed().to_vec();
        hash.insert(0, hash.len() as u8);
        hash.insert(0, 1);
        Ok(TxnHash(
            bs58::encode(hash)
                .with_check_version(V1_TXN_HASH)
                .into_string(),
        ))
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
            nonce: command.source_nonce(),
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

impl From<(mina_rs::UserCommand, bool)> for SignedCommandWithCreationData {
    fn from(value: (mina_rs::UserCommand, bool)) -> Self {
        let mina_rs::UserCommand::SignedCommand(v1) = value.0;
        Self {
            signed_command: v1.into(),
            is_new_receiver_account: value.1,
        }
    }
}

impl From<mina_rs::UserCommandWithStatus> for SignedCommand {
    fn from(value: mina_rs::UserCommandWithStatus) -> Self {
        Self::from_user_command(value.into())
    }
}

impl From<UserCommandWithStatus> for SignedCommand {
    fn from(value: UserCommandWithStatus) -> Self {
        let value: mina_rs::UserCommandWithStatus = value.into();
        value.into()
    }
}

impl From<SignedCommandWithCreationData> for Command {
    fn from(value: SignedCommandWithCreationData) -> Command {
        let signed = value.signed_command;
        match signed.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload_v1) => {
                let mina_rs::PaymentPayload {
                    source_pk,
                    receiver_pk,
                    amount,
                    ..
                } = &payment_payload_v1.t.t;
                Command::Payment(Payment {
                    source: source_pk.to_owned().into(),
                    receiver: receiver_pk.to_owned().into(),
                    amount: amount.t.t.into(),
                    nonce: signed.source_nonce(),
                    is_new_receiver_account: value.is_new_receiver_account,
                })
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation_v1) => {
                let mina_rs::StakeDelegation::SetDelegate {
                    delegator,
                    new_delegate,
                } = stake_delegation_v1.t.to_owned();
                Command::Delegation(Delegation {
                    delegate: new_delegate.into(),
                    delegator: delegator.into(),
                    nonce: signed.source_nonce(),
                })
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

impl From<Versioned2<mina_rs::SignedCommand, 1, 1>> for SignedCommand {
    fn from(value: Versioned2<mina_rs::SignedCommand, 1, 1>) -> Self {
        Self(value)
    }
}

impl From<SignedCommand> for serde_json::Value {
    fn from(value: SignedCommand) -> Self {
        use serde_json::*;

        let mut object = Map::new();
        object.insert("payload".into(), payload_json(&value.0));
        object.insert("signer".into(), signer(&value.0));
        object.insert("signature".into(), signature(&value.0));

        Value::Object(object)
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
        let tx_hash = Value::String(value.tx_hash.0);
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

fn signer(value: &mina_rs::SignedCommandV1) -> serde_json::Value {
    let pk: PublicKey = value.t.t.signer.0.t.to_owned().into();
    serde_json::Value::String(pk.0)
}

fn signature(value: &mina_rs::SignedCommandV1) -> serde_json::Value {
    let sig: Signature = value.t.t.signature.to_owned().into();
    serde_json::Value::String(sig.to_string())
}

fn payload_json(value: &mina_rs::SignedCommandV1) -> serde_json::Value {
    use serde_json::*;

    let mut payload_obj = Map::new();
    let mina_rs::SignedCommand { payload, .. } = &value.t.t;

    let mut common = Map::new();
    let mina_rs::SignedCommandPayloadCommon {
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

    let body = match &payload.t.t.body.t.t {
        mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
            let mut body_obj = Map::new();
            let mina_rs::PaymentPayload {
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
        mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation) => {
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
        Self(value)
    }
}

impl std::fmt::Display for TxnHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SignedCommand;
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::signed::TxnHash,
    };
    use std::path::PathBuf;

    #[test]
    fn transaction_hash() -> anyhow::Result<()> {
        // refer to the hashes on Minascan
        // https://minascan.io/mainnet/tx/CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj
        // https://minascan.io/mainnet/tx/CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX

        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V1).unwrap();
        let hashes = precomputed_block.command_hashes();
        let expect = vec![
            TxnHash::new("CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX").unwrap(),
            TxnHash::new("CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj").unwrap(),
        ];

        assert_eq!(hashes, expect);
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
