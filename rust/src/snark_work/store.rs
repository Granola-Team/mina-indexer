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
    fn get_pk_num_prover_blocks(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

    /// Update snark work prover fees
    fn update_snark_prover_fees(&self, snarks: Vec<SnarkWorkSummary>) -> anyhow::Result<()>;

    /// Get top `n` SNARK provers by accumulated fees
    fn get_top_snark_provers_by_total_fees(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>>;

    /// Set the SNARK for the prover in `block_height` at `index`
    fn set_snark_by_prover_block_height(
        &self,
        snark: &SnarkWorkSummary,
        block_height: u32,
        index: u32,
    ) -> anyhow::Result<()>;

    /// Set the SNARK for the prover in `global_slot` at `index`
    fn set_snark_by_prover_global_slot(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> anyhow::Result<()>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator over SNARKs sorted by fee & block height
    fn snark_fees_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARKs sorted by fee & block height
    fn snark_fees_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers by max fee
    fn snark_prover_max_fee_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers by accumulated fees
    fn snark_prover_total_fees_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARKs by prover, sorted by block height & index
    fn snark_prover_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARKs by prover, sorted by global slot & index
    fn snark_prover_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    //////////////////
    // SNARK counts //
    //////////////////

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

    /// Set SNARK count for a block
    fn set_block_snarks_count(&self, state_hash: &BlockHash, count: u32) -> anyhow::Result<()>;

    /// Get num SNARKs per block
    fn get_block_snarks_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

    /// Increment snarks counts given `snark` in `epoch`
    fn increment_snarks_counts(&self, snark: &SnarkWorkSummary, epoch: u32) -> anyhow::Result<()>;
}
