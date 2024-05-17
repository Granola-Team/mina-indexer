use crate::{
    command::*,
    protocol::{
        bin_prot,
        serialization_types::{staged_ledger_diff as mina_rs, version_bytes::V1_TXN_HASH},
    },
};
use blake2::digest::VariableOutput;
use mina_serialization_versioned::Versioned2;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommand(pub mina_rs::SignedCommandV1);

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithStateHash {
    pub command: SignedCommand,
    pub state_hash: BlockHash,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedCommandWithData {
    pub command: SignedCommand,
    pub state_hash: BlockHash,
    pub status: CommandStatusData,
    pub tx_hash: String,
    pub blockchain_length: u32,
    pub date_time: u64,
    pub global_slot_since_genesis: u32,
}

impl SignedCommand {
    pub fn fee(&self) -> u64 {
        self.payload_common().fee.inner().inner()
    }

    pub fn fee_payer_pk(&self) -> PublicKey {
        self.payload_common().fee_payer_pk.into()
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                payment_payload.t.t.receiver_pk.into()
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                match delegation_payload.t {
                    mina_rs::StakeDelegation::SetDelegate {
                        delegator: _,
                        new_delegate,
                    } => new_delegate.into(),
                }
            }
        }
    }

    pub fn source_pk(&self) -> PublicKey {
        match self.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                payment_payload.t.t.source_pk.into()
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                match delegation_payload.t {
                    mina_rs::StakeDelegation::SetDelegate {
                        delegator,
                        new_delegate: _,
                    } => delegator.into(),
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

    pub fn is_delegation(&self) -> bool {
        matches!(
            self.payload_body(),
            mina_rs::SignedCommandPayloadBody::StakeDelegation(_)
        )
    }

    pub fn payload(&self) -> &mina_rs::SignedCommandPayload {
        &self.0.t.t.payload.t.t
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc.0.inner().data.inner().inner() {
            mina_rs::UserCommand::SignedCommand(signed_command) => signed_command.into(),
        }
    }

    pub fn source_nonce(&self) -> u32 {
        self.payload_common().nonce.t.t as u32
    }

    pub fn payload_body(&self) -> mina_rs::SignedCommandPayloadBody {
        self.payload().body.clone().inner().inner()
    }

    pub fn payload_common(&self) -> mina_rs::SignedCommandPayloadCommon {
        self.payload().common.clone().inner().inner().inner()
    }

    /// This returns a user command (transaction) hash that starts with
    /// [TXN_HASH_START]
    pub fn hash_signed_command(&self) -> anyhow::Result<String> {
        let mut binprot_bytes = Vec::new();
        bin_prot::to_writer(&mut binprot_bytes, &self.0).map_err(anyhow::Error::from)?;

        let binprot_bytes_bs58 = bs58::encode(&binprot_bytes[..])
            .with_check_version(0x13)
            .into_string();
        let mut hasher = blake2::Blake2bVar::new(32).unwrap();

        hasher.write_all(binprot_bytes_bs58.as_bytes()).unwrap();

        let mut hash = hasher.finalize_boxed().to_vec();
        hash.insert(0, hash.len() as u8);
        hash.insert(0, 1);

        Ok(bs58::encode(hash)
            .with_check_version(V1_TXN_HASH)
            .into_string())
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block.commands().into_iter().map(Self::from).collect()
    }
}

impl SignedCommandWithStateHash {
    pub fn from(signed_cmd: &SignedCommand, state_hash: &str) -> Self {
        Self {
            command: signed_cmd.clone(),
            state_hash: state_hash.into(),
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

impl From<mina_rs::UserCommand> for SignedCommand {
    fn from(value: mina_rs::UserCommand) -> Self {
        let mina_rs::UserCommand::SignedCommand(v1) = value;
        Self(v1)
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

impl From<SignedCommand> for Command {
    fn from(value: SignedCommand) -> Command {
        match value.payload_body() {
            mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload_v1) => {
                let mina_rs::PaymentPayload {
                    source_pk,
                    receiver_pk,
                    amount,
                    ..
                } = payment_payload_v1.inner().inner();
                Command::Payment(Payment {
                    source: source_pk.into(),
                    receiver: receiver_pk.into(),
                    amount: amount.inner().inner().into(),
                    nonce: value.source_nonce(),
                })
            }
            mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation_v1) => {
                let mina_rs::StakeDelegation::SetDelegate {
                    delegator,
                    new_delegate,
                } = stake_delegation_v1.inner();
                Command::Delegation(Delegation {
                    delegate: new_delegate.into(),
                    delegator: delegator.into(),
                    nonce: value.source_nonce(),
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
        value.command.into()
    }
}

impl From<SignedCommandWithStateHash> for CommandWithStateHash {
    fn from(value: SignedCommandWithStateHash) -> Self {
        Self {
            command: value.command.into(),
            state_hash: value.state_hash,
        }
    }
}

impl From<Versioned2<mina_rs::SignedCommand, 1, 1>> for SignedCommand {
    fn from(value: Versioned2<mina_rs::SignedCommand, 1, 1>) -> Self {
        SignedCommand(value)
    }
}

impl From<SignedCommand> for serde_json::Value {
    fn from(value: SignedCommand) -> Self {
        use serde_json::*;

        let mut object = Map::new();
        let payload = payload_json(value.0.clone());
        let signer = signer(value.0.clone());
        let signature = signature(value.0);

        object.insert("payload".into(), payload);
        object.insert("signer".into(), signer);
        object.insert("signature".into(), signature);
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
        let tx_hash = Value::String(value.tx_hash);
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
        value.command.into()
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

fn signer(value: mina_rs::SignedCommandV1) -> serde_json::Value {
    use serde_json::*;
    let pk: PublicKey = value.inner().inner().signer.0.inner().into();
    Value::String(pk.0)
}

fn signature(_value: mina_rs::SignedCommandV1) -> serde_json::Value {
    use serde_json::*;

    Value::String("signature".into())
}

fn payload_json(value: mina_rs::SignedCommandV1) -> serde_json::Value {
    use serde_json::*;

    let mut payload_obj = Map::new();
    let mina_rs::SignedCommand { payload, .. } = value.inner().inner();

    let mut common = Map::new();
    let mina_rs::SignedCommandPayloadCommon {
        fee,
        fee_token,
        fee_payer_pk,
        nonce,
        valid_until,
        memo,
    } = payload.t.t.common.t.t.t.clone();
    common.insert(
        "fee".into(),
        Value::Number(Number::from(fee.inner().inner())),
    );
    common.insert(
        "fee_token".into(),
        Value::Number(Number::from(fee_token.inner().inner().inner())),
    );
    common.insert(
        "fee_payer_pk".into(),
        Value::String(PublicKey::from(fee_payer_pk).to_address()),
    );
    common.insert(
        "nonce".into(),
        Value::Number(Number::from(nonce.inner().inner())),
    );
    common.insert(
        "valid_until".into(),
        Value::Number(Number::from(valid_until.inner().inner() as u32)),
    );
    common.insert(
        "memo".into(),
        Value::String(String::from_utf8_lossy(&memo.inner().0).to_string()),
    );

    let body = match payload.inner().inner().body.inner().inner() {
        mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
            let mut body_obj = Map::new();
            let mina_rs::PaymentPayload {
                source_pk,
                receiver_pk,
                token_id,
                amount,
            } = payment_payload.inner().inner();

            body_obj.insert(
                "source_pk".into(),
                Value::String(PublicKey::from(source_pk).to_address()),
            );
            body_obj.insert(
                "receiver_pk".into(),
                Value::String(PublicKey::from(receiver_pk).to_address()),
            );
            body_obj.insert(
                "token_id".into(),
                Value::Number(Number::from(token_id.inner().inner().inner())),
            );
            body_obj.insert(
                "amount".into(),
                Value::Number(Number::from(amount.inner().inner())),
            );
            body_obj.insert("kind".into(), Value::String("Payment".into()));
            Value::Object(body_obj)
        }
        mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation) => {
            let mut body_obj = Map::new();
            let mina_rs::StakeDelegation::SetDelegate {
                delegator,
                new_delegate,
            } = stake_delegation.inner();

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

pub const TXN_HASH_LEN: usize = 53;
pub const TXN_HASH_START: &str = "Ckp";

pub fn is_valid_tx_hash(input: &str) -> bool {
    input.starts_with(TXN_HASH_START) && input.len() == TXN_HASH_LEN
}

#[cfg(test)]
mod tests {
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::path::PathBuf;

    #[test]
    fn transaction_hash() {
        // refer to the hashes on Minascan
        // https://minascan.io/mainnet/tx/CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj
        // https://minascan.io/mainnet/tx/CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX

        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V1).unwrap();
        let hashes = precomputed_block.command_hashes();
        let expect = vec![
            "CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX".to_string(),
            "CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj".to_string(),
        ];

        assert_eq!(hashes, expect);
    }
}
