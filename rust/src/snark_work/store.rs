use super::{SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal};
use crate::{
    block::{precomputed::PrecomputedBlock, store::DbBlockUpdate, BlockHash},
    ledger::public_key::PublicKey,
    store::DbUpdate,
};
use speedb::{DBIterator, Direction, IteratorMode};

pub struct SnarkUpdate {
    pub state_hash: BlockHash,
    pub global_slot_since_genesis: u32,
    pub works: Vec<SnarkWorkSummary>,
}

pub type DbSnarkUpdate = DbUpdate<SnarkUpdate>;

pub trait SnarkStore {
    /// Add SNARK work in a precomputed block
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get SNARK work in a given block
    fn get_block_snark_work(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummary>>>;

    /// Get SNARK work associated with a prover key
    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<SnarkWorkSummaryWithStateHash>>;

    /// Update SNARK work prover fees
    fn update_snark_prover_fees(
        &self,
        epoch: u32,
        snarks: &[SnarkWorkSummary],
    ) -> anyhow::Result<()>;

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

    /// Get the SNARK prover's total fees for all SNARKs sold
    fn get_snark_prover_total_fees(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

    /// Get the SNARK prover's total fees for all SNARKs sold in the given epoch
    /// (default: current epoch)
    fn get_snark_prover_epoch_fees(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<Option<u64>>;

    /// Get the SNARK prover's max fee for all SNARKs sold
    fn get_snark_prover_max_fee(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

    /// Get the SNARK prover's max fee for all SNARKs sold in the given epoch
    /// (default: current epoch)
    fn get_snark_prover_epoch_max_fee(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<Option<u64>>;

    /// Get the SNARK prover's min fee for all SNARKs sold
    fn get_snark_prover_min_fee(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

    /// Get the SNARK prover's min fee for all SNARKs sold in the given epoch
    /// (default: current epoch)
    fn get_snark_prover_epoch_min_fee(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<Option<u64>>;

    /// Update SNARK work from the applied & unapplied blocks
    fn update_block_snarks(&self, blocks: &DbBlockUpdate) -> anyhow::Result<()>;

    /// Update SNARK work for each update
    fn update_snarks(&self, update: DbSnarkUpdate) -> anyhow::Result<()>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator over SNARKs sorted by fee & block height
    fn snark_fees_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARKs sorted by fee & block height
    fn snark_fees_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers by max fee
    fn snark_prover_max_fee_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers per epoch by max fee
    fn snark_prover_max_fee_epoch_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_>;

    /// Iterator over SNARK provers by min fee
    fn snark_prover_min_fee_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers per epoch by min fee
    fn snark_prover_min_fee_epoch_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_>;

    /// Iterator over SNARK provers by accumulated fees
    fn snark_prover_total_fees_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator over SNARK provers per epoch by accumulated fees
    fn snark_prover_total_fees_epoch_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_>;

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
