use crate::{
    block::precomputed::PrecomputedBlock,
    state::ledger::{public_key::PublicKey, Amount},
};
use mina_serialization_types::{
    staged_ledger_diff::{
        SignedCommandPayloadBody, SignedCommandPayloadCommon, StakeDelegation,
        TransactionStatusBalanceData, UserCommand,
    },
    v1::{PaymentPayloadV1, PublicKeyV1, SignedCommandV1, UserCommandWithStatusV1},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TransactionType {
    Payment,
    Delegation,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub source: PublicKeyV1,
    pub receiver: PublicKeyV1,
    pub amount: Amount,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: PublicKeyV1,
    pub delegate: PublicKeyV1,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Command {
    Payment(Payment),
    Delegation(Delegation),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SignedCommand(pub SignedCommandV1);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CommandStatusData {
    Applied {
        balance_data: TransactionStatusBalanceData,
    },
    Failed,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UserCommandWithStatus(pub UserCommandWithStatusV1);

impl UserCommandWithStatus {
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
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PaymentPayload(pub PaymentPayloadV1);

impl Command {
    pub fn from_precomputed_block(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block
            .commands()
            .iter()
            .map(
                |command| match UserCommandWithStatus(command.clone()).data() {
                    UserCommand::SignedCommand(signed_command) => {
                        match SignedCommand(signed_command).payload_body() {
                            SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                                let source = payment_payload.clone().inner().inner().source_pk;
                                let receiver = payment_payload.clone().inner().inner().receiver_pk;
                                let amount = payment_payload.inner().inner().amount.inner().inner();
                                Self::Payment(Payment {
                                    source,
                                    receiver,
                                    amount: amount.into(),
                                })
                            }
                            SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                                match delegation_payload.inner() {
                                    StakeDelegation::SetDelegate {
                                        delegator,
                                        new_delegate,
                                    } => Self::Delegation(Delegation {
                                        delegate: new_delegate,
                                        delegator,
                                    }),
                                }
                            }
                        }
                    }
                },
            )
            .collect()
    }
}

impl SignedCommand {
    pub fn payload_body(&self) -> SignedCommandPayloadBody {
        self.0
            .clone()
            .inner()
            .inner()
            .payload
            .inner()
            .inner()
            .body
            .inner()
            .inner()
    }

    pub fn payload_common(&self) -> SignedCommandPayloadCommon {
        self.0
            .clone()
            .inner()
            .inner()
            .payload
            .inner()
            .inner()
            .common
            .inner()
            .inner()
            .inner()
    }

    pub fn fee_payer_pk(&self) -> PublicKey {
        self.payload_common().fee_payer_pk.into()
    }

    pub fn signer(&self) -> PublicKey {
        self.0.clone().inner().inner().signer.0.inner().into()
    }
}

impl UserCommandWithStatus {
    pub fn data(self) -> UserCommand {
        self.0.inner().data.inner().inner()
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

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::{Command, Delegation, Payment};
    use crate::{block::parser::BlockParser, state::ledger::PublicKey};

    #[tokio::test]
    async fn from_precomputed() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw
        let log_dir = PathBuf::from("./tests/data/non_sequential_blocks");
        let mut bp = BlockParser::new(&log_dir).unwrap();
        let block = bp
            .get_precomputed_block("3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw")
            .await
            .unwrap();

        let mut payments = Vec::new();
        let mut delegations = Vec::new();
        for command in Command::from_precomputed_block(&block) {
            match command {
                Command::Payment(Payment {
                    source,
                    receiver,
                    amount,
                }) => {
                    let source = PublicKey::from(source);
                    let receiver = PublicKey::from(receiver);
                    println!("s: {source:?}");
                    println!("r: {receiver:?}");
                    println!("a: {}", amount.0);
                    payments.push((source, receiver, amount));
                }
                Command::Delegation(Delegation {
                    delegate,
                    delegator,
                }) => {
                    let delegate = PublicKey::from(delegate);
                    let delegator = PublicKey::from(delegator);
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
                    .map(|(s, r, a)| (s.to_address(), r.to_address(), (*a).0))
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
