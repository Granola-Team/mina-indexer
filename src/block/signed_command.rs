use crate::state::ledger::{command::UserCommandWithStatus, public_key::PublicKey};
use blake2::digest::VariableOutput;
use mina_serialization_types::staged_ledger_diff::{SignedCommandPayload, UserCommand};
use std::io::Write;
use versioned::Versioned;

pub struct SignedCommand(pub mina_serialization_types::staged_ledger_diff::SignedCommandV1);

impl SignedCommand {
    pub fn payload(&self) -> &SignedCommandPayload {
        &self.0.t.t.payload.t.t
    }

    pub fn from_user_command(uc: UserCommandWithStatus) -> Self {
        match uc.0.t.data.t.t {
            UserCommand::SignedCommand(signed_command) => signed_command.into(),
        }
    }

    pub fn source_nonce(&self) -> i32 {
        self.0.t.t.payload.t.t.common.t.t.t.nonce.t.t
    }

    pub fn fee_payer(&self) -> PublicKey {
        self.0
            .t
            .t
            .payload
            .t
            .t
            .common
            .t
            .t
            .t
            .fee_payer_pk
            .clone()
            .into()
    }

    pub fn receiver_pk(&self) -> PublicKey {
        match self.0.t.t.payload.t.t.body.t.t.clone() {
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
        match self.0.t.t.payload.t.t.body.t.t.clone() {
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
        match self.0.t.t.payload.t.t.body.t.t.clone() {
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::PaymentPayload(_payment_payload)
                => false,
            mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBody::StakeDelegation(_delegation_payload)
                => true,
        }
    }

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

        Ok(bs58::encode(hash).with_check_version(0x12).into_string())
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
        SignedCommand(value)
    }
}

#[cfg(test)]
mod tests {
    use super::SignedCommand;
    use crate::block::parse_file;
    use mina_serialization_types::staged_ledger_diff::UserCommand;
    use std::path::PathBuf;

    #[tokio::test]
    async fn transaction_hash() {
        // refer to the hashes on Minascan
        // https://minascan.io/mainnet/tx/CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj
        // https://minascan.io/mainnet/tx/CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX

        let block_file = PathBuf::from("./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json");
        let precomputed_block = parse_file(&block_file).await.unwrap();
        let commands = precomputed_block.commands();
        let hashes: Vec<String> = commands
            .iter()
            .map(|commandv1| {
                let UserCommand::SignedCommand(signed_commandv1) = commandv1.t.data.t.t.clone();
                SignedCommand(signed_commandv1)
                    .hash_signed_command()
                    .unwrap()
            })
            .collect();
        let expect = vec![
            "CkpZZsSm9hQpGkGzMi8rcsQEWPZwGJXktiqGYADNwLoBeeamhzqnX".to_string(),
            "CkpZDcqGWQVpckXjcg99hh4EzmCrnPzMM8VzHaLAYxPU5tMubuLaj".to_string(),
        ];

        assert_eq!(hashes, expect);
    }
}
