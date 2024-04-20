use super::{SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal};
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::public_key::PublicKey,
};

pub trait SnarkStore {
    /// Add snark work in a precomputed block
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get snark work in a given block
    fn get_snark_work_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummary>>>;

    /// Get snark work associated with a prover key
    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummaryWithStateHash>>>;

    /// Get number of blocks which pk is a SNARK work prover
    fn get_pk_num_prover_blocks(&self, pk: &str) -> anyhow::Result<Option<u32>>;

    /// Update top snark work producers
    fn update_top_snarkers(&self, snarks: Vec<SnarkWorkSummary>) -> anyhow::Result<()>;

    /// Get top snark work producers
    fn get_top_snarkers(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>>;
}
