use mina_serialization_types::{
    staged_ledger_diff::{SignedCommandPayloadBody, StakeDelegation, UserCommand},
    v1::PublicKeyV1,
};

use crate::precomputed_block::PrecomputedBlock;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TransactionType {
    Payment,
    Delegation,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Payment {
    pub source: PublicKeyV1,
    pub receiver: PublicKeyV1,
    pub amount: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Delegation {
    pub delegator: PublicKeyV1,
    pub delegate: PublicKeyV1,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Payment(Payment),
    Delegation(Delegation),
}

impl Command {
    // i say i say now this is a thiccy
    pub fn from_precomputed_block(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block
            .staged_ledger_diff
            .diff
            .clone()
            .inner()
            .0
            .inner()
            .inner()
            .commands
            .iter()
            .map(
                |command| match command.clone().inner().data.inner().inner() {
                    UserCommand::SignedCommand(signed_command) => match signed_command
                        .inner()
                        .inner()
                        .payload
                        .inner()
                        .inner()
                        .body
                        .inner()
                        .inner()
                    {
                        SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                            let source = payment_payload.clone().inner().inner().source_pk;
                            let receiver = payment_payload.clone().inner().inner().receiver_pk;
                            let amount = payment_payload.inner().inner().amount.inner().inner();
                            Self::Payment(Payment {
                                source,
                                receiver,
                                amount,
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
                    },
                },
            )
            .collect()
    }
}
