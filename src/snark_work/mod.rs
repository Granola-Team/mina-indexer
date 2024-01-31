use crate::{block::precomputed::PrecomputedBlock, ledger::public_key::PublicKey};
use mina_serialization_types::snark_work as mina_rs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnarkWork {
    pub fee: u64,
    pub prover: PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnarkWorkWithData {
    pub fee: u64,
    pub prover: PublicKey,
    pub state_hash: String,
}

impl SnarkWork {
    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .completed_works()
            .into_iter()
            .map(Self::from)
            .collect()
    }
}

impl From<mina_rs::TransactionSnarkWork> for SnarkWork {
    fn from(value: mina_rs::TransactionSnarkWork) -> Self {
        Self {
            fee: value.fee.inner().inner(),
            prover: value.prover.into(),
        }
    }
}
