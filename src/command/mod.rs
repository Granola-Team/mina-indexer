pub mod internal;
pub mod signed;
pub mod store;

use crate::{
    block::precomputed::PrecomputedBlock,
    command::signed::SignedCommand,
    ledger::{account::Amount, post_balances::PostBalance, public_key::PublicKey},
};
use mina_serialization_types::{
    staged_ledger_diff as mina_rs,
    v1::{PaymentPayloadV1, UserCommandWithStatusV1},
};
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
    Delegation(Delegation),
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
        balance_data: mina_rs::TransactionStatusBalanceData,
    },
    Failed,
}

impl CommandStatusData {
    pub fn fee_payer_balance(data: &mina_rs::TransactionStatusBalanceData) -> Option<u64> {
        data.fee_payer_balance.as_ref().map(|balance| balance.t.t.t)
    }

    pub fn receiver_balance(balance_data: &mina_rs::TransactionStatusBalanceData) -> Option<u64> {
        balance_data
            .receiver_balance
            .as_ref()
            .map(|balance| balance.t.t.t)
    }

    pub fn source_balance(balance_data: &mina_rs::TransactionStatusBalanceData) -> Option<u64> {
        balance_data
            .source_balance
            .as_ref()
            .map(|balance| balance.t.t.t)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UserCommandWithStatus(pub UserCommandWithStatusV1);

impl UserCommandWithStatus {
    pub fn is_applied(&self) -> bool {
        if let CommandStatusData::Applied { .. } = self.status_data() {
            return true;
        }
        false
    }

    pub fn status_data(&self) -> CommandStatusData {
        match self.0.t.status.t.clone() {
            mina_serialization_types::staged_ledger_diff::TransactionStatus::Applied(
                _,
                balance_data,
            ) => CommandStatusData::Applied {
                balance_data: balance_data.t,
            },
            mina_serialization_types::staged_ledger_diff::TransactionStatus::Failed(_, _) => {
                CommandStatusData::Failed
            }
        }
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
pub struct PaymentPayload(pub PaymentPayloadV1);

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

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use serde_json::*;

        let mut json = Map::new();
        match self {
            Self::Payment(Payment {
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
            Self::Delegation(Delegation {
                delegate,
                delegator,
            }) => {
                let mut delegation = Map::new();
                delegation.insert("delegate".into(), Value::String(delegate.to_address()));
                delegation.insert("delegator".into(), Value::String(delegator.to_address()));
                json.insert("StakeDelegation".into(), Value::Object(delegation));
            }
        };
        write!(f, "{}", to_string(&json).unwrap())
    }
}

impl CommandUpdate {
    pub fn is_delegation(&self) -> bool {
        matches!(self.command_type, CommandType::Delegation)
    }
}

#[cfg(test)]
mod test {
    use super::{Command, Delegation, Payment};
    use crate::{block::parser::BlockParser, MAINNET_CANONICAL_THRESHOLD};
    use std::path::PathBuf;

    #[tokio::test]
    async fn from_precomputed() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw
        let log_dir = PathBuf::from("./tests/data/non_sequential_blocks");
        let mut bp = BlockParser::new(&log_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();
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
}
