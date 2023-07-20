use mina_serialization_types::staged_ledger_diff::{SignedCommandPayload, UserCommand};
use versioned::Versioned;

use crate::state::ledger::{command::UserCommandWithStatus, public_key::PublicKey};

pub struct SignedCommand(mina_serialization_types::staged_ledger_diff::SignedCommand);

impl SignedCommand {
    pub fn payload(&self) -> &SignedCommandPayload {
        &self.0.payload.t.t
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc.0.t.data.t.t {
            UserCommand::SignedCommand(signed_command) => signed_command.into(),
        }
    }

    pub fn source_nonce(&self) -> i32 {
        self.0.payload.t.t.common.t.t.t.nonce.t.t
    }

    pub fn fee_payer(&self) -> PublicKey {
        self.0.payload.t.t.common.t.t.t.fee_payer_pk.clone().into()
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self.0.payload.t.t.body.t.t.clone() {
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::PaymentPayload(payment_payload)
                => payment_payload.t.t.receiver_pk.into(),
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::StakeDelegation(delegation_payload)
                => match delegation_payload.t {
                    mina_serialization_types::staged_ledger_diff::StakeDelegation::SetDelegate { delegator: _, new_delegate }
                        => new_delegate.into(),
                },
        }
    }

    pub fn source_pk(&self) -> PublicKey {
        match self.0.payload.t.t.body.t.t.clone() {
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::PaymentPayload(payment_payload)
                => payment_payload.t.t.source_pk.into(),
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::StakeDelegation(delegation_payload)
                => match delegation_payload.t {
                    mina_serialization_types::staged_ledger_diff::StakeDelegation::SetDelegate { delegator, new_delegate: _ }
                        => delegator.into(),
                },
        }
    }

    pub fn is_delegation(&self) -> bool {
        match self.0.payload.t.t.body.t.t.clone() {
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::PaymentPayload(_payment_payload)
                => false,
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::StakeDelegation(_delegation_payload)
                => true,
        }
    }
}

impl From<Versioned<Versioned<mina_serialization_types::staged_ledger_diff::SignedCommand, 1>, 1>>
    for SignedCommand
{
    fn from(
        value: Versioned<
            Versioned<mina_serialization_types::staged_ledger_diff::SignedCommand, 1>,
            1,
        >,
    ) -> Self {
        SignedCommand(value.t.t)
    }
}
