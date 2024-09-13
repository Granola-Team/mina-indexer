use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    ledger::public_key::PublicKey,
    snark_work::{
        store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal,
    },
    utility::db::{from_be_bytes, to_be_bytes, u32_prefix_key, u64_prefix_key},
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

/// **Key format:** `{fee}{slot}{pk}{hash}{num}`
/// ```
/// fee:  8 BE bytes
/// slot: 4 BE bytes
/// pk:   [PublicKey::LEN] bytes
/// hash: [BlockHash::LEN] bytes
/// num:  4 BE bytes
pub fn snark_fee_prefix_key(
    fee: u64,
    global_slot: u32,
    pk: PublicKey,
    state_hash: BlockHash,
    num: u32,
) -> [u8; 4 * 2 + 8 + PublicKey::LEN + BlockHash::LEN] {
    const SIZE_OF_U32: usize = 4; // u32 is always 4 bytes
    const SIZE_OF_U64: usize = 8; // u64 is always 8 bytes

    let mut bytes = [0u8; SIZE_OF_U32 * 2 + SIZE_OF_U64 + PublicKey::LEN + BlockHash::LEN];

    let mut start_index = 0;

    // Copy fee (u64) - 8 bytes
    bytes[start_index..start_index + SIZE_OF_U64].copy_from_slice(&fee.to_be_bytes());
    start_index += SIZE_OF_U64;

    // Copy global_slot (u32) - 4 bytes
    bytes[start_index..start_index + SIZE_OF_U32].copy_from_slice(&global_slot.to_be_bytes());
    start_index += SIZE_OF_U32;

    // Copy pk (PublicKey) - PublicKey::LEN bytes
    bytes[start_index..start_index + PublicKey::LEN].copy_from_slice(&pk.to_bytes());
    start_index += PublicKey::LEN;

    // Copy state_hash (BlockHash) - BlockHash::LEN bytes
    bytes[start_index..start_index + BlockHash::LEN].copy_from_slice(&state_hash.to_bytes());
    start_index += BlockHash::LEN;

    // Copy num (u32) - 4 bytes
    bytes[start_index..start_index + SIZE_OF_U32].copy_from_slice(&num.to_be_bytes());

    bytes
}

/// **Key format:** `{prover}{slot}{index}`
/// ```
/// - prover: [PublicKey::LEN] bytes
/// - slot:   4 BE bytes
/// - index:  4 BE bytes
fn snark_prover_prefix_key(prover: &PublicKey, global_slot: u32, index: u32) -> Vec<u8> {
    let mut bytes = prover.0.as_bytes().to_vec();
    bytes.append(&mut to_be_bytes(global_slot).to_vec());
    bytes.append(&mut to_be_bytes(index).to_vec());
    bytes
}

impl SnarkStore for IndexerStore {
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding SNARK work from block {}", block.summary());

        let epoch = block.epoch_count();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let completed_works = SnarkWorkSummary::from_precomputed(block);
        let completed_works_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(block);

        // add: state hash -> snark works
        let state_hash = block.state_hash().0;
        let key = state_hash.as_bytes();
        let value = serde_json::to_vec(&completed_works)?;
        self.database.put_cf(self.snarks_cf(), key, value)?;

        // per block SNARK count
        self.set_block_snarks_count(&block.state_hash(), completed_works.len() as u32)?;

        // store fee info
        let mut num_prover_works: HashMap<PublicKey, u32> = HashMap::new();
        for snark in completed_works {
            let num = num_prover_works.get(&snark.prover).copied().unwrap_or(0);
            self.database.put_cf(
                self.snark_work_fees_cf(),
                snark_fee_prefix_key(
                    snark.fee,
                    block.global_slot_since_genesis(),
                    snark.prover.clone(),
                    block.state_hash(),
                    num,
                ),
                b"",
            )?;

            // build the block's fee table
            if num_prover_works.get(&snark.prover).is_some() {
                *num_prover_works.get_mut(&snark.prover).unwrap() += 1;
            } else {
                num_prover_works.insert(snark.prover.clone(), 1);
            }
        }

        // add: "pk -> linked list of SNARK work summaries with state hash"
        for pk in block.prover_keys() {
            let pk_str = pk.to_address();
            trace!("Adding SNARK work for pk {pk}");

            // get pk's next index
            let n = self.get_pk_num_prover_blocks(&pk_str)?.unwrap_or(0);

            let block_pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = completed_works_state_hash
                .clone()
                .into_iter()
                .filter(|snark| snark.contains_pk(&pk))
                .collect();

            if !block_pk_snarks.is_empty() {
                // write these SNARKs to the next key for pk
                let key = format!("{pk_str}{n}").as_bytes().to_vec();
                let value = serde_json::to_vec(&block_pk_snarks)?;
                self.database.put_cf(self.snarks_cf(), key, value)?;

                // update pk's next index
                let key = pk_str.as_bytes();
                let next_n = (n + 1).to_string();
                let value = next_n.as_bytes();
                self.database.put_cf(self.snarks_cf(), key, value)?;

                // increment SNARK counts
                for (index, snark) in block_pk_snarks.iter().enumerate() {
                    if self
                        .database
                        .get_pinned_cf(
                            self.snark_work_prover_cf(),
                            snark_prover_prefix_key(&pk, global_slot, index as u32),
                        )?
                        .is_none()
                    {
                        let snark: SnarkWorkSummary = snark.clone().into();
                        self.set_snark_by_prover(&snark, global_slot, index as u32)?;
                        self.set_snark_by_prover_height(&snark, block_height, index as u32)?;
                        self.increment_snarks_counts(&snark, epoch)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn get_pk_num_prover_blocks(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        let key = pk.as_bytes();
        Ok(self
            .database
            .get_pinned_cf(self.snarks_cf(), key)?
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }))
    }

    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummaryWithStateHash>>> {
        let pk = pk.to_address();
        trace!("Getting SNARK work for public key {pk}");

        let snarks_cf = self.snarks_cf();
        let mut all_snarks = None;
        fn key_n(pk: String, n: u32) -> Vec<u8> {
            format!("{pk}{n}").as_bytes().to_vec()
        }

        if let Some(n) = self.get_pk_num_prover_blocks(&pk)? {
            for m in 0..n {
                if let Some(mut block_m_snarks) = self
                    .database
                    .get_pinned_cf(snarks_cf, key_n(pk.clone(), m))?
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
        trace!("Getting SNARK work in block {}", state_hash.0);

        let key = state_hash.0.as_bytes();
        if let Some(snarks_bytes) = self.database.get_pinned_cf(self.snarks_cf(), key)? {
            return Ok(Some(serde_json::from_slice(&snarks_bytes)?));
        }
        Ok(None)
    }

    fn update_top_snarkers(&self, snarks: Vec<SnarkWorkSummary>) -> anyhow::Result<()> {
        trace!("Updating top SNARK workers");

        let mut prover_fees: HashMap<PublicKey, (u64, u64)> = HashMap::new();
        for snark in snarks {
            let key = snark.prover.0.as_bytes();
            if prover_fees.get(&snark.prover).is_some() {
                prover_fees.get_mut(&snark.prover).unwrap().1 += snark.fee;
            } else {
                let old_total = self
                    .database
                    .get_pinned_cf(self.snark_top_producers_cf(), key)?
                    .map_or(0, |fee_bytes| {
                        serde_json::from_slice::<u64>(&fee_bytes).expect("fee is u64")
                    });
                prover_fees.insert(snark.prover.clone(), (old_total, snark.fee));

                // delete the stale data
                self.database.delete_cf(
                    self.snark_top_producers_sort_cf(),
                    u64_prefix_key(old_total, &snark.prover),
                )?
            }
        }

        // replace stale data with updated
        for (prover, (old_total, new_fees)) in prover_fees.iter() {
            let total_fees = old_total + new_fees;
            let key = u64_prefix_key(total_fees, prover);
            self.database
                .put_cf(self.snark_top_producers_sort_cf(), key, b"")?
        }

        Ok(())
    }

    fn get_top_snark_workers_by_fees(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>> {
        trace!("Getting top {n} SNARK workers by fees");
        Ok(self
            .top_snark_workers_iterator(IteratorMode::End)
            .take(n)
            .map(|res| {
                res.map(|(bytes, _)| {
                    let mut total_fees_bytes = [0; 8];
                    total_fees_bytes.copy_from_slice(&bytes[..8]);
                    SnarkWorkTotal {
                        prover: PublicKey::from_bytes(&bytes[8..]).expect("public key"),
                        total_fees: u64::from_be_bytes(total_fees_bytes),
                    }
                })
                .expect("snark work iterator")
            })
            .collect())
    }

    fn top_snark_workers_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_top_producers_sort_cf(), mode)
    }

    fn snark_fees_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database.iterator_cf(self.snark_work_fees_cf(), mode)
    }

    fn set_snark_by_prover(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting snark slot {global_slot} at index {index} for prover {}",
            snark.prover
        );
        Ok(self.database.put_cf(
            self.snark_work_prover_cf(),
            snark_prover_prefix_key(&snark.prover, global_slot, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    /// `{prover}{slot}{index} -> snark`
    /// - prover: 55 pk bytes
    /// - slot:   4 BE bytes
    /// - index:  4 BE bytes
    /// - snark:  serde_json encoded
    fn snark_prover_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database.iterator_cf(self.snark_work_prover_cf(), mode)
    }

    fn set_snark_by_prover_height(
        &self,
        snark: &SnarkWorkSummary,
        block_height: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting snark slot {block_height} at index {index} for prover {}",
            snark.prover
        );
        Ok(self.database.put_cf(
            self.snark_work_prover_height_cf(),
            snark_prover_prefix_key(&snark.prover, block_height, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    /// `{prover}{height}{index} -> snark`
    /// - prover:         55 pk bytes
    /// - block height:   4 BE bytes
    /// - index:          4 BE bytes
    /// - snark:          serde_json encoded
    fn snark_prover_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_work_prover_height_cf(), mode)
    }

    fn get_snarks_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch {epoch} SNARKs count");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_epoch_cf(), to_be_bytes(epoch))?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_snarks_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing epoch {epoch} SNARKs count");
        let old = self.get_snarks_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.snarks_epoch_cf(),
            to_be_bytes(epoch),
            to_be_bytes(old + 1),
        )?)
    }

    fn get_snarks_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total SNARKs count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_SNARKS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_snarks_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing total SNARKs count");

        let old = self.get_snarks_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_SNARKS_KEY, to_be_bytes(old + 1))?)
    }

    fn get_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} SNARKs count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_pk_epoch_cf(), u32_prefix_key(epoch, pk))?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} SNARKs count {pk}");

        let old = self.get_snarks_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.database.put_cf(
            self.snarks_pk_epoch_cf(),
            u32_prefix_key(epoch, pk),
            to_be_bytes(old + 1),
        )?)
    }

    fn get_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total SNARKs count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.snarks_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing pk total SNARKs count {pk}");

        let old = self.get_snarks_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.snarks_pk_total_cf(),
            pk.0.as_bytes(),
            to_be_bytes(old + 1),
        )?)
    }

    fn get_block_snarks_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block SNARKs count {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.block_snark_counts_cf(), state_hash.0.as_bytes())?
            .map(|bytes| from_be_bytes(bytes.to_vec())))
    }

    fn set_block_snarks_count(&self, state_hash: &BlockHash, count: u32) -> anyhow::Result<()> {
        trace!("Setting block SNARKs count {state_hash} -> {count}");
        Ok(self.database.put_cf(
            self.block_snark_counts_cf(),
            state_hash.0.as_bytes(),
            to_be_bytes(count),
        )?)
    }

    fn increment_snarks_counts(&self, snark: &SnarkWorkSummary, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing SNARKs count {snark:?}");

        // prover epoch & total
        let prover = snark.prover.clone();
        self.increment_snarks_pk_epoch_count(&prover, epoch)?;
        self.increment_snarks_pk_total_count(&prover)?;

        // epoch & total counts
        self.increment_snarks_epoch_count(epoch)?;
        self.increment_snarks_total_count()
    }
}

#[cfg(test)]
mod snark_store_impl_tests {
    use super::*;

    #[test]
    fn test_snark_fee_prefix_key_length() {
        // Mock values for PublicKey and BlockHash
        let pk = PublicKey::default();
        let state_hash = BlockHash::default();

        // Invoke the function
        let result = snark_fee_prefix_key(100u64, 50u32, pk, state_hash, 25u32);

        // Expected size of the result array
        assert_eq!(result.len(), 4 * 2 + 8 + PublicKey::LEN + BlockHash::LEN);
    }

    #[test]
    fn test_snark_fee_prefix_key_content() {
        // Mock values for PublicKey and BlockHash
        let pk = PublicKey::default(); // All 1s
        let state_hash = BlockHash::default(); // All 2s

        // Values for fee, global_slot, and num
        let fee = 100u64;
        let global_slot = 50u32;
        let num = 25u32;

        // Invoke the function
        let result = snark_fee_prefix_key(fee, global_slot, pk.clone(), state_hash.clone(), num);

        // Assert the fee is correctly copied (big-endian u64)
        assert_eq!(&result[0..8], &fee.to_be_bytes());

        // Assert the global_slot is correctly copied (big-endian u32)
        assert_eq!(&result[8..12], &global_slot.to_be_bytes());

        // Assert the PublicKey is correctly copied
        assert_eq!(&result[12..12 + PublicKey::LEN], &pk.to_bytes());

        // Assert the BlockHash is correctly copied
        assert_eq!(
            &result[12 + PublicKey::LEN..12 + PublicKey::LEN + BlockHash::LEN],
            &state_hash.to_bytes()
        );

        // Assert the num is correctly copied (big-endian u32)
        assert_eq!(&result[result.len() - 4..], &num.to_be_bytes());
    }
}
