use super::{SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal};
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::public_key::PublicKey,
};
use speedb::{DBIterator, IteratorMode};

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

    /// Top snarker by fees terator
    fn top_snarkers_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Set the SNARK for the prover in `global_slot` at `index`
    fn set_snark_by_prover(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> anyhow::Result<()>;

    /// Iterator over SNARKs by prover, sorted by global slot & index
    fn snark_prover_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Increment snarks per epoch count
    fn increment_snarks_epoch_count(&self, epoch: u32) -> anyhow::Result<()>;

    /// Get snarks per epoch count
    fn get_snarks_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32>;

    /// Increment snarks total count
    fn increment_snarks_total_count(&self) -> anyhow::Result<()>;

    /// Get snarks total count
    fn get_snarks_total_count(&self) -> anyhow::Result<u32>;

    /// Increment snarks per epoch per account count
    fn increment_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> anyhow::Result<()>;

    /// Get snarks per epoch per account count
    fn get_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> anyhow::Result<u32>;

    /// Increment snarks per account total
    fn increment_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()>;

    /// Get snarks per account total
    fn get_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Increment snarks counts given `snark` in `epoch`
    fn increment_snarks_counts(&self, snark: &SnarkWorkSummary, epoch: u32) -> anyhow::Result<()>;
}
