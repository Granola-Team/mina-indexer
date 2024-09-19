use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    ledger::public_key::PublicKey,
    snark_work::{
        store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal,
    },
    utility::store::{
        from_be_bytes, pk_index_key, snarks::*, u32_from_be_bytes, u32_prefix_key,
        u64_from_be_bytes, u64_prefix_key, U64_LEN,
    },
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

impl SnarkStore for IndexerStore {
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding SNARK work from block {}", block.summary());
        let state_hash = block.state_hash();
        let epoch = block.epoch_count();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let completed_works = SnarkWorkSummary::from_precomputed(block);
        let completed_works_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(block);

        // add: state hash -> snark works
        self.database.put_cf(
            self.snarks_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&completed_works)?,
        )?;

        // per block SNARK count
        self.set_block_snarks_count(&state_hash, completed_works.len() as u32)?;

        // store fee info
        let mut num_prover_works: HashMap<PublicKey, u32> = HashMap::new();
        for snark in completed_works.iter() {
            let index = num_prover_works.get(&snark.prover).unwrap_or(&0);
            self.database.put_cf(
                self.snark_work_fees_block_height_sort_cf(),
                snark_fee_sort_key(snark.fee, block_height, &snark.prover, &state_hash, *index),
                b"",
            )?;
            self.database.put_cf(
                self.snark_work_fees_global_slot_sort_cf(),
                snark_fee_sort_key(snark.fee, global_slot, &snark.prover, &state_hash, *index),
                b"",
            )?;

            // build the block's fee table
            if num_prover_works.contains_key(&snark.prover) {
                *num_prover_works.get_mut(&snark.prover).unwrap() += 1;
            } else {
                num_prover_works.insert(snark.prover, 1);
            }
        }

        // add: "pk -> linked list of SNARK work summaries with state hash"
        for pk in block.prover_keys() {
            trace!("Adding SNARK work for pk {pk}");

            // get pk's next index
            let n = self.get_pk_num_prover_blocks(&pk)?.unwrap_or(0);
            let block_pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = completed_works_state_hash
                .into_iter()
                .filter(|snark| snark.contains_pk(&pk))
                .collect();

            if !block_pk_snarks.is_empty() {
                // write these SNARKs to the next key for pk
                self.database.put_cf(
                    self.snarks_cf(),
                    pk_index_key(&pk, n),
                    serde_json::to_vec(&block_pk_snarks)?,
                )?;

                // update pk's next index
                self.database
                    .put_cf(self.snarks_cf(), pk.0.as_bytes(), (n + 1).to_be_bytes())?;

                // increment SNARK counts
                for (index, snark) in block_pk_snarks.iter().enumerate() {
                    if self
                        .database
                        .get_pinned_cf(
                            self.snark_prover_block_height_sort_cf(),
                            snark_prover_sort_key(&pk, block_height, index as u32),
                        )?
                        .is_none()
                    {
                        let snark: SnarkWorkSummary = snark.clone().into();
                        self.set_snark_by_prover_block_height(&snark, block_height, index as u32)?;
                        self.set_snark_by_prover_global_slot(&snark, global_slot, index as u32)?;
                        self.increment_snarks_counts(&snark, epoch)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn get_pk_num_prover_blocks(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        Ok(self
            .database
            .get_cf(self.snarks_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummaryWithStateHash>>> {
        trace!("Getting SNARK work for public key {pk}");
        let mut all_snarks = None;

        if let Some(n) = self.get_pk_num_prover_blocks(pk)? {
            for m in 0..n {
                if let Some(mut block_m_snarks) = self
                    .database
                    .get_pinned_cf(self.snarks_cf(), pk_index_key(pk, m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<SnarkWorkSummaryWithStateHash>>(&bytes)
                            .expect("snark work with state hash")
                    })
                {
                    let mut snarks = all_snarks.unwrap_or(vec![]);
                    snarks.append(&mut block_m_snarks);
                    all_snarks = Some(snarks);
                } else {
                    all_snarks = None;
                    break;
                }
            }
        }
        Ok(all_snarks)
    }

    fn get_snark_work_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummary>>> {
        trace!("Getting SNARK work in block {state_hash}");
        if let Some(snarks_bytes) = self
            .database
            .get_pinned_cf(self.snarks_cf(), state_hash.0.as_bytes())?
        {
            return Ok(Some(serde_json::from_slice(&snarks_bytes)?));
        }
        Ok(None)
    }

    fn update_snark_prover_fees(&self, snarks: Vec<SnarkWorkSummary>) -> anyhow::Result<()> {
        trace!("Updating SNARK prover fees");

        let mut prover_fees: HashMap<PublicKey, (u64, u64)> = HashMap::new();
        for snark in snarks {
            if prover_fees.contains_key(&snark.prover) {
                // update total
                prover_fees.get_mut(&snark.prover).unwrap().0 += snark.fee;

                // update max
                let max_fee = prover_fees.get(&snark.prover).unwrap().1;
                if snark.fee > max_fee {
                    prover_fees.get_mut(&snark.prover).unwrap().1 = snark.fee;
                }
            } else {
                let old_max = self
                    .database
                    .get_pinned_cf(self.snark_prover_max_fee_cf(), snark.prover.0.as_bytes())?
                    .map_or(0, |bytes| {
                        u64_from_be_bytes(&bytes).expect("snark prover max fee")
                    });
                let old_total = self
                    .database
                    .get_pinned_cf(self.snark_prover_fees_cf(), snark.prover.0.as_bytes())?
                    .map_or(0, |bytes| {
                        u64_from_be_bytes(&bytes).expect("snark prover total fees")
                    });

                // delete the stale data
                self.database.delete_cf(
                    self.snark_prover_max_fee_sort_cf(),
                    u64_prefix_key(old_max, &snark.prover),
                )?;
                self.database.delete_cf(
                    self.snark_prover_total_fees_sort_cf(),
                    u64_prefix_key(old_total, &snark.prover),
                )?;
                prover_fees.insert(
                    snark.prover,
                    (old_total + snark.fee, snark.fee.max(old_max)),
                );
            }
        }

        // replace stale data with updated
        for (prover, (total_fees, max_fee)) in prover_fees.iter() {
            self.database.put_cf(
                self.snark_prover_fees_cf(),
                prover.0.as_bytes(),
                max_fee.to_be_bytes(),
            )?;
            self.database.put_cf(
                self.snark_prover_max_fee_cf(),
                prover.0.as_bytes(),
                total_fees.to_be_bytes(),
            )?;

            // sort data
            self.database.put_cf(
                self.snark_prover_max_fee_sort_cf(),
                u64_prefix_key(*max_fee, prover),
                b"",
            )?;
            self.database.put_cf(
                self.snark_prover_total_fees_sort_cf(),
                u64_prefix_key(*total_fees, prover),
                b"",
            )?;
        }
        Ok(())
    }

    fn get_top_snark_provers_by_total_fees(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>> {
        trace!("Getting top {n} SNARK workers by fees");
        Ok(self
            .snark_prover_total_fees_iterator(IteratorMode::End)
            .take(n)
            .map(|res| {
                res.map(|(bytes, _)| {
                    let mut total_fees_bytes = [0; U64_LEN];
                    total_fees_bytes.copy_from_slice(&bytes[..U64_LEN]);
                    SnarkWorkTotal {
                        prover: PublicKey::from_bytes(&bytes[U64_LEN..]).expect("public key"),
                        total_fees: u64::from_be_bytes(total_fees_bytes),
                    }
                })
                .expect("snark work iterator")
            })
            .collect())
    }

    fn set_snark_by_prover_block_height(
        &self,
        snark: &SnarkWorkSummary,
        block_height: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        let prover = snark.prover;
        trace!("Setting snark block height {block_height} at index {index} for prover {prover}");
        Ok(self.database.put_cf(
            self.snark_prover_block_height_sort_cf(),
            snark_prover_sort_key(&prover, block_height, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    fn set_snark_by_prover_global_slot(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        let prover = snark.prover;
        trace!("Setting snark global slot {global_slot} at index {index} for prover {prover}");
        Ok(self.database.put_cf(
            self.snark_prover_global_slot_sort_cf(),
            snark_prover_sort_key(&prover, global_slot, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    ///////////////
    // Iterators //
    ///////////////

    fn snark_fees_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_work_fees_block_height_sort_cf(), mode)
    }

    fn snark_fees_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_work_fees_global_slot_sort_cf(), mode)
    }

    fn snark_prover_max_fee_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_max_fee_sort_cf(), mode)
    }

    fn snark_prover_total_fees_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_total_fees_sort_cf(), mode)
    }

    fn snark_prover_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_block_height_sort_cf(), mode)
    }

    fn snark_prover_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_global_slot_sort_cf(), mode)
    }

    //////////////////
    // SNARK counts //
    //////////////////

    fn get_snarks_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch {epoch} SNARKs count");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_epoch_cf(), epoch.to_be_bytes())?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("epoch SNARK count")
            }))
    }

    fn increment_snarks_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing epoch {epoch} SNARKs count");
        let old = self.get_snarks_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.snarks_epoch_cf(),
            epoch.to_be_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_snarks_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total SNARKs count");
        Ok(self
            .database
            .get_pinned(Self::TOTAL_NUM_SNARKS_KEY)?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("total SNARK count")
            }))
    }

    fn increment_snarks_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing total SNARKs count");
        let old = self.get_snarks_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_SNARKS_KEY, (old + 1).to_be_bytes())?)
    }

    fn get_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} SNARKs count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_pk_epoch_cf(), u32_prefix_key(epoch, pk))?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("pk epoch SNARK count")
            }))
    }

    fn increment_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} SNARKs count {pk}");
        let old = self.get_snarks_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.database.put_cf(
            self.snarks_pk_epoch_cf(),
            u32_prefix_key(epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total SNARKs count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("pk total SNARK count")
            }))
    }

    fn increment_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing pk total SNARKs count {pk}");
        let old = self.get_snarks_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.snarks_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_block_snarks_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block SNARKs count {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.block_snark_counts_cf(), state_hash.0.as_bytes())?
            .map(|bytes| u32_from_be_bytes(&bytes).expect("block SNARK count")))
    }

    fn set_block_snarks_count(&self, state_hash: &BlockHash, count: u32) -> anyhow::Result<()> {
        trace!("Setting block SNARKs count {state_hash} -> {count}");
        Ok(self.database.put_cf(
            self.block_snark_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        )?)
    }

    fn increment_snarks_counts(&self, snark: &SnarkWorkSummary, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing SNARKs count {snark:?}");

        // prover epoch & total
        let prover = snark.prover;
        self.increment_snarks_pk_epoch_count(&prover, epoch)?;
        self.increment_snarks_pk_total_count(&prover)?;

        // epoch & total counts
        self.increment_snarks_epoch_count(epoch)?;
        self.increment_snarks_total_count()
    }
}
