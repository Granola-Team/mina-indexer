pub mod internal;
pub mod signed;
pub mod store;

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::signed::{SignedCommand, SignedCommandWithKind},
    ledger::{account::Amount, post_balances::PostBalance, public_key::PublicKey},
};
use mina_serialization_types::{staged_ledger_diff as mina_rs, v1 as mina_v1};
use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum CommandType {
    Payment,
    Delegation,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Command {
    Payment(Payment),
    #[serde(rename = "Stake_delegation")]
    Delegation(Delegation),
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CommandWithStateHash {
    pub command: Command,
    pub state_hash: BlockHash,
}

pub struct CommandUpdate {
    pub source_nonce: u32,
    pub command_type: CommandType,
    pub fee_payer: PostBalance,
    pub source: PostBalance,
    pub receiver: PostBalance,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub source: PublicKey,
    pub receiver: PublicKey,
    pub amount: Amount,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CommandStatusData {
    Applied {
        auxiliary_data: mina_rs::TransactionStatusAuxiliaryData,
        balance_data: mina_rs::TransactionStatusBalanceData,
    },
    Failed(
        Vec<mina_rs::TransactionStatusFailedType>,
        mina_rs::TransactionStatusBalanceData,
    ),
}

impl CommandStatusData {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    fn balance_data(&self) -> Option<&mina_rs::TransactionStatusBalanceData> {
        if let Self::Applied { balance_data, .. } = self {
            return Some(balance_data);
        }
        None
    }

    fn auxiliary_data(&self) -> Option<&mina_rs::TransactionStatusAuxiliaryData> {
        if let Self::Applied { auxiliary_data, .. } = self {
            return Some(auxiliary_data);
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

    pub fn from_transaction_status(data: &mina_rs::TransactionStatus) -> Self {
        use mina_rs::TransactionStatus as TS;

        match data {
            TS::Applied(auxiliary_data, balance_data) => Self::Applied {
                auxiliary_data: auxiliary_data.clone().inner(),
                balance_data: balance_data.clone().inner(),
            },
            TS::Failed(fails, balance_data) => Self::Failed(
                fails.iter().map(|reason| reason.clone().inner()).collect(),
                balance_data.clone().inner(),
            ),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UserCommandWithStatus(pub mina_v1::UserCommandWithStatusV1);

impl UserCommandWithStatus {
    pub fn is_applied(&self) -> bool {
        self.status_data().is_applied()
    }

    pub fn status_data(&self) -> CommandStatusData {
        match self.0.t.status.t.clone() {
            mina_rs::TransactionStatus::Applied(auxiliary_data, balance_data) => {
                CommandStatusData::Applied {
                    auxiliary_data: auxiliary_data.inner(),
                    balance_data: balance_data.inner(),
                }
            }
            mina_rs::TransactionStatus::Failed(reason, balance_data) => CommandStatusData::Failed(
                reason.iter().map(|r| r.clone().inner()).collect(),
                balance_data.inner(),
            ),
        }
    }

    pub fn contains_public_key(&self, pk: &PublicKey) -> bool {
        let signed = SignedCommand::from(self.clone());
        signed.all_command_public_keys().contains(pk)
    }

    pub fn data(&self) -> mina_rs::UserCommand {
        self.0.clone().inner().data.inner().inner()
    }

    pub fn to_command(&self) -> Command {
        match self.data() {
            mina_rs::UserCommand::SignedCommand(v1) => match v1
                .inner()
                .inner()
                .payload
                .inner()
                .inner()
                .body
                .inner()
                .inner()
            {
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
                    })
                }
                mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation_v1) => {
                    let mina_rs::StakeDelegation::SetDelegate {
                        delegator,
                        new_delegate,
                    } = stake_delegation_v1.inner();
                    Command::Delegation(Delegation {
                        delegator: delegator.into(),
                        delegate: new_delegate.into(),
                    })
                }
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PaymentPayload(pub mina_v1::PaymentPayloadV1);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct StakeDelegation(pub mina_v1::StakeDelegationV1);

impl Command {
    /// Get the list of commands from the precomputed block
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block
            .commands()
            .iter()
            .filter(|&command| command.is_applied())
            .map(|command| match command.clone().data() {
                mina_rs::UserCommand::SignedCommand(signed_command) => {
                    match SignedCommand(signed_command).payload_body() {
                        mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                            let source: PublicKey =
                                payment_payload.clone().inner().inner().source_pk.into();
                            let receiver: PublicKey =
                                payment_payload.clone().inner().inner().receiver_pk.into();
                            let amount = payment_payload.inner().inner().amount.inner().inner();
                            trace!(
                                "Payment {{ source: {}, receiver: {}, amount: {amount} }}",
                                source.to_address(),
                                receiver.to_address()
                            );
                            Self::Payment(Payment {
                                source,
                                receiver,
                                amount: amount.into(),
                            })
                        }
                        mina_rs::SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                            match delegation_payload.inner() {
                                mina_rs::StakeDelegation::SetDelegate {
                                    delegator,
                                    new_delegate,
                                } => {
                                    let delegator: PublicKey = delegator.into();
                                    let new_delegate: PublicKey = new_delegate.into();
                                    trace!(
                                        "Delegation {{ delegator: {}, new_delegate: {} }}",
                                        delegator.to_address(),
                                        new_delegate.to_address()
                                    );
                                    Self::Delegation(Delegation {
                                        delegate: new_delegate,
                                        delegator,
                                    })
                                }
                            }
                        }
                    }
                }
            })
            .collect()
    }
}

impl PaymentPayload {
    pub fn source_pk(&self) -> PublicKey {
        self.0.clone().inner().inner().source_pk.into()
    }

    pub fn receiver_pk(&self) -> PublicKey {
        self.0.clone().inner().inner().receiver_pk.into()
    }
}

impl StakeDelegation {
    pub fn delegator(&self) -> PublicKey {
        let mina_rs::StakeDelegation::SetDelegate { delegator, .. } = self.0.clone().inner();
        delegator.into()
    }

    pub fn new_delegate(&self) -> PublicKey {
        let mina_rs::StakeDelegation::SetDelegate { new_delegate, .. } = self.0.clone().inner();
        new_delegate.into()
    }
}

impl CommandUpdate {
    pub fn is_delegation(&self) -> bool {
        matches!(self.command_type, CommandType::Delegation)
    }
}

impl From<mina_rs::TransactionStatus> for CommandStatusData {
    fn from(value: mina_rs::TransactionStatus) -> Self {
        Self::from_transaction_status(&value)
    }
}

impl From<mina_rs::UserCommandWithStatus> for UserCommandWithStatus {
    fn from(value: mina_rs::UserCommandWithStatus) -> Self {
        Self(versioned::Versioned {
            version: 1,
            t: value,
        })
    }
}

impl From<UserCommandWithStatus> for mina_rs::UserCommandWithStatus {
    fn from(value: UserCommandWithStatus) -> Self {
        value.0.inner()
    }
}

impl From<UserCommandWithStatus> for Command {
    fn from(value: UserCommandWithStatus) -> Self {
        value.data().into()
    }
}

impl From<mina_rs::UserCommand> for Command {
    fn from(value: mina_rs::UserCommand) -> Self {
        let value: SignedCommand = value.into();
        value.into()
    }
}

impl From<UserCommandWithStatus> for CommandStatusData {
    fn from(value: UserCommandWithStatus) -> Self {
        value.status_data()
    }
}

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
                amount,
            }) => {
                let mut payment = Map::new();
                payment.insert("source".into(), Value::String(source.to_address()));
                payment.insert("receiver".into(), Value::String(receiver.to_address()));
                payment.insert("amount".into(), Value::Number(Number::from(amount.0)));
                json.insert("Payment".into(), Value::Object(payment));
            }
            Command::Delegation(Delegation {
                delegate,
                delegator,
            }) => {
                let mut delegation = Map::new();
                delegation.insert("delegate".into(), Value::String(delegate.to_address()));
                delegation.insert("delegator".into(), Value::String(delegator.to_address()));
                json.insert("Stake_delegation".into(), Value::Object(delegation));
            }
        };
        Value::Object(json)
    }
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

        let mut object = Map::new();
        let user_cmd: UserCommandWithStatus = value.0.inner().into();
        let status: CommandStatusData = user_cmd.clone().into();
        let data: SignedCommandWithKind = user_cmd.into();

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

#[cfg(test)]
mod test {
    use super::{Command, Delegation, Payment};
    use crate::{block::parser::BlockParser, constants::*, ledger::account::nanomina_to_mina};
    use std::path::PathBuf;

    #[tokio::test]
    async fn from_precomputed() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw
        let log_dir = PathBuf::from("./tests/data/non_sequential_blocks");
        let mut bp = BlockParser::new_with_canonical_chain_discovery(
            &log_dir,
            MAINNET_CANONICAL_THRESHOLD,
            BLOCK_REPORTING_FREQ_NUM,
        )
        .unwrap();
        let block = bp
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
                    amount,
                }) => {
                    println!("s: {source:?}");
                    println!("r: {receiver:?}");
                    println!("a: {}", amount.0);
                    payments.push((source, receiver, amount));
                }
                Command::Delegation(Delegation {
                    delegate,
                    delegator,
                }) => {
                    println!("d: {delegate:?}");
                    println!("t: {delegator:?}");
                    delegations.push((delegate, delegator));
                }
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
    fn user_command_with_status_json() -> anyhow::Result<()> {
        use crate::block::precomputed::PrecomputedBlock;
        use serde_json::*;

        fn convert(value: serde_json::Value) -> serde_json::Value {
            match value {
                Value::Number(n) => Value::String(n.to_string()),
                Value::Object(mut obj) => {
                    obj.iter_mut().for_each(|(key, x)| {
                        if *key == json!("memo") || *key == json!("signature") {
                            *x = Value::Null
                        } else {
                            *x = convert(x.clone())
                        }
                    });
                    Value::Object(obj)
                }
                Value::Array(arr) => Value::Array(arr.into_iter().map(convert).collect()),
                x => x,
            }
        }
        fn fee_convert(value: serde_json::Value) -> serde_json::Value {
            match value {
                Value::Object(mut obj) => {
                    obj.iter_mut().for_each(|(key, x)| {
                        if *key == json!("fee") {
                            *x = {
                                let nanomina = x.clone().to_string().parse::<u64>().unwrap();
                                Value::String(nanomina_to_mina(nanomina))
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
        fn to_mina_format(json: serde_json::Value) -> serde_json::Value {
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
                            if kind == Value::String("Payment".into())
                                || kind == Value::String("Stake_delegation".into())
                            {
                                body.remove("kind");
                                obj["body"] = Value::Array(vec![kind, Value::Object(body)]);
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
        fn to_mina_json(json: serde_json::Value) -> serde_json::Value {
            to_mina_format(convert(fee_convert(json)))
        }

        let path: PathBuf = "./tests/data/non_sequential_blocks/mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw.json".into();
        let contents = std::fs::read(path.clone())?;
        let mina_json: Value =
            from_slice::<Value>(&contents)?["staged_ledger_diff"]["diff"][0]["commands"][0].clone();
        let block = PrecomputedBlock::parse_file(&path)?;
        let user_cmd_with_status = block.commands()[0].clone();
        let user_cmd_with_status: Value = user_cmd_with_status.into();

        assert_eq!(convert(mina_json), to_mina_json(user_cmd_with_status));
        Ok(())
    }
}

fn to_auxiliary_json(
    auxiliary_data: &mina_rs::TransactionStatusAuxiliaryData,
) -> serde_json::Value {
    use serde_json::*;

    let mut auxiliary_obj = Map::new();
    let fee_payer_account_creation_fee_paid = auxiliary_data
        .fee_payer_account_creation_fee_paid
        .clone()
        .map(|amt| Value::Number(Number::from(amt.inner().inner())))
        .unwrap_or(Value::Null);
    let receiver_account_creation_fee_paid = auxiliary_data
        .receiver_account_creation_fee_paid
        .clone()
        .map(|amt| Value::Number(Number::from(amt.inner().inner())))
        .unwrap_or(Value::Null);
    let created_token = auxiliary_data
        .created_token
        .clone()
        .map(|id| Value::Number(Number::from(id.inner().inner().inner())))
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

fn to_balance_json(balance_data: &mina_rs::TransactionStatusBalanceData) -> serde_json::Value {
    use serde_json::*;

    let mut balance_obj = Map::new();
    let fee_payer_balance = balance_data
        .fee_payer_balance
        .clone()
        .map(|amt| Value::Number(Number::from(amt.inner().inner().inner())))
        .unwrap_or(Value::Null);
    let receiver_balance = balance_data
        .receiver_balance
        .clone()
        .map(|amt| Value::Number(Number::from(amt.inner().inner().inner())))
        .unwrap_or(Value::Null);
    let source_balance = balance_data
        .source_balance
        .clone()
        .map(|amt| Value::Number(Number::from(amt.inner().inner().inner())))
        .unwrap_or(Value::Null);

    balance_obj.insert("fee_payer_balance".into(), fee_payer_balance);
    balance_obj.insert("receiver_balance".into(), receiver_balance);
    balance_obj.insert("source_balance".into(), source_balance);
    Value::Object(balance_obj)
}
