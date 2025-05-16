//! SNARK store impl

use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, DbUpdate, IndexerStore};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{
        precomputed::PrecomputedBlock,
        store::{BlockStore, BlockUpdate, DbBlockUpdate},
    },
    canonicity::store::CanonicityStore,
    snark_work::{
        store::{DbSnarkUpdate, SnarkApplication, SnarkProverFees, SnarkStore, SnarkUpdate},
        SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal,
    },
    store::Result,
    utility::store::{
        common::{
            block_index_key, pk_index_key, u32_from_be_bytes, u64_from_be_bytes, u64_prefix_key,
            U32_LEN, U64_LEN,
        },
        snarks::*,
    },
};
use log::trace;
use serde::{Deserialize, Serialize};
use speedb::{DBIterator, Direction, IteratorMode};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnarkAllTimeFees {
    pub total: u64,
    pub max: u64,
    pub min: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnarkEpochFees {
    pub total: u64,
    pub max: u64,
    pub min: u64,
}

impl SnarkStore for IndexerStore {
    fn add_snark_work(&self, block: &PrecomputedBlock) -> Result<()> {
        trace!("Adding SNARK work block {}", block.summary());

        let state_hash = block.state_hash();
        let epoch = block.epoch_count();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let genesis_state_hash = block.genesis_state_hash();
        let completed_works = SnarkWorkSummary::from_precomputed(block);

        // add snark works & fee data
        let mut num_prover_works: HashMap<PublicKey, u32> = HashMap::new();
        let num_snarks = completed_works.len() as u32;

        self.set_block_snarks_count(&state_hash, num_snarks)?;
        for (index, snark) in completed_works.iter().enumerate() {
            // add snark
            self.database.put_cf(
                self.snarks_cf(),
                block_index_key(&state_hash, index as u32),
                serde_json::to_vec(snark)?,
            )?;

            // add fee data
            let prover_index = num_prover_works
                .get(&snark.prover)
                .copied()
                .unwrap_or_default();
            self.database.put_cf(
                self.snark_work_fees_block_height_sort_cf(),
                snark_fee_sort_key(
                    snark.fee.0,
                    block_height,
                    &snark.prover,
                    &state_hash,
                    prover_index,
                ),
                b"",
            )?;
            self.database.put_cf(
                self.snark_work_fees_global_slot_sort_cf(),
                snark_fee_sort_key(
                    snark.fee.0,
                    global_slot,
                    &snark.prover,
                    &state_hash,
                    prover_index,
                ),
                b"",
            )?;

            // build the block's fee table
            if num_prover_works.contains_key(&snark.prover) {
                *num_prover_works.get_mut(&snark.prover).unwrap() += 1;
            } else {
                num_prover_works.insert(snark.prover.clone(), 1);
            }
        }

        // add: "pk -> linked list of SNARK work summaries with state hash"
        let completed_works_state_hash: Vec<_> = completed_works
            .into_iter()
            .map(|snark| SnarkWorkSummaryWithStateHash::from(snark, state_hash.clone()))
            .collect();

        for pk in block.prover_keys() {
            // get pk's next index
            let n = self.get_snarks_pk_total_count(&pk)?;
            let block_pk_snarks: Vec<_> = completed_works_state_hash
                .iter()
                .filter(|snark| snark.contains_pk(&pk))
                .collect();

            // increment SNARK counts
            for (index, snark) in block_pk_snarks.into_iter().enumerate() {
                if self
                    .database
                    .get_cf(
                        self.snark_prover_block_height_sort_cf(),
                        snark_prover_sort_key(&pk, block_height, index as u32),
                    )?
                    .is_none()
                {
                    self.database.put_cf(
                        self.snarks_prover_cf(),
                        pk_index_key(&pk, n + index as u32),
                        serde_json::to_vec(snark)?,
                    )?;

                    let snark: SnarkWorkSummary = snark.clone().into();
                    self.set_snark_by_prover_block_height(&snark, block_height, index as u32)?;
                    self.set_snark_by_prover_global_slot(&snark, global_slot, index as u32)?;
                    self.increment_snarks_counts(&snark, epoch, &genesis_state_hash)?;
                }
            }
        }

        Ok(())
    }

    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> Result<Vec<SnarkWorkSummaryWithStateHash>> {
        trace!("Getting SNARK work pk {}", pk);

        let mut snarks = vec![];
        for index in 0..self.get_snarks_pk_total_count(pk)? {
            snarks.push(
                self.database
                    .get_cf(self.snarks_prover_cf(), pk_index_key(pk, index))?
                    .map(|bytes| {
                        serde_json::from_slice(&bytes).expect("SNARK work with state hash")
                    })
                    .expect("prover SNARK work"),
            );
        }

        Ok(snarks)
    }

    fn get_block_snark_work(
        &self,
        state_hash: &StateHash,
    ) -> Result<Option<Vec<SnarkWorkSummary>>> {
        trace!("Getting SNARK work block {}", state_hash);

        let mut snarks: Vec<SnarkWorkSummary> = vec![];
        if let Some(num) = self.get_block_snarks_count(state_hash)? {
            for index in 0..num {
                let snark = self
                    .database
                    .get_cf(self.snarks_cf(), block_index_key(state_hash, index))?
                    .map(|bytes| serde_json::from_slice(&bytes).expect("SNARK work"))
                    .expect("SNARK");
                snarks.push(snark);
            }

            return Ok(Some(snarks));
        }

        Ok(None)
    }

    fn snark_prover_fees(
        snarks: &[SnarkWorkSummary],
    ) -> Result<HashMap<PublicKey, SnarkProverFees>> {
        trace!("Calculating SNARK fees");

        let mut prover_fees = <HashMap<PublicKey, SnarkProverFees>>::new();
        for snark in snarks.iter() {
            if let Some(fees) = prover_fees.get_mut(&snark.prover) {
                // update total, max & min fees
                fees.total += snark.fee.0;

                if snark.fee.0 > fees.max {
                    fees.max = snark.fee.0;
                }

                if snark.fee.0 < fees.min {
                    fees.min = snark.fee.0;
                }
            } else {
                prover_fees.insert(
                    snark.prover.clone(),
                    SnarkProverFees {
                        total: snark.fee.0,
                        max: snark.fee.0,
                        min: snark.fee.0,
                    },
                );
            }
        }

        Ok(prover_fees)
    }

    fn update_snark_prover_fees(
        &self,
        epoch: u32,
        block_height: u32,
        genesis_state_hash: &StateHash,
        snarks: &[SnarkWorkSummary],
        apply: SnarkApplication,
    ) -> Result<()> {
        trace!(
            "Updating SNARK fees epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        let block_height_opt = match apply {
            SnarkApplication::Apply => None,
            SnarkApplication::Unapply => Some(block_height),
        };

        let mut old_prover_fees = HashMap::new();
        for snark in snarks.iter() {
            if !old_prover_fees.contains_key(&snark.prover) {
                // delete old all-time fees
                let old_total = self
                    .get_snark_prover_total_fees(&snark.prover, None)?
                    .unwrap_or_default();
                let old_max = self
                    .get_snark_prover_max_fee(&snark.prover, None)?
                    .unwrap_or_default();
                let old_min = self
                    .get_snark_prover_min_fee(&snark.prover, None)?
                    .unwrap_or(u64::MAX);

                self.delete_old_all_time_snark_fees(&snark.prover, old_total, old_max, old_min)?;

                // delete old epoch fees
                let old_epoch_total = self
                    .get_snark_prover_epoch_fees(
                        &snark.prover,
                        Some(epoch),
                        Some(genesis_state_hash),
                        block_height_opt,
                    )?
                    .unwrap_or_default();
                let old_epoch_max = self
                    .get_snark_prover_epoch_max_fee(
                        &snark.prover,
                        Some(epoch),
                        Some(genesis_state_hash),
                        block_height_opt,
                    )?
                    .unwrap_or_default();
                let old_epoch_min = self
                    .get_snark_prover_epoch_min_fee(
                        &snark.prover,
                        Some(epoch),
                        Some(genesis_state_hash),
                        block_height_opt,
                    )?
                    .unwrap_or(u64::MAX);

                self.delete_old_epoch_snark_fees(
                    epoch,
                    genesis_state_hash,
                    &snark.prover,
                    old_epoch_total,
                    old_epoch_max,
                    old_epoch_min,
                )?;

                // update the old SNARK fee table
                old_prover_fees.insert(
                    snark.prover.clone(),
                    (
                        SnarkAllTimeFees {
                            total: old_total,
                            max: old_max,
                            min: old_min,
                        },
                        SnarkEpochFees {
                            total: old_epoch_total,
                            max: old_epoch_max,
                            min: old_epoch_min,
                        },
                    ),
                );
            }
        }

        // update fees from given snarks
        for (prover, SnarkProverFees { total, max, min }) in Self::snark_prover_fees(snarks)?.iter()
        {
            match old_prover_fees.get(prover) {
                None => unreachable!(),
                Some((
                    SnarkAllTimeFees {
                        total: old_total,
                        max: old_max,
                        min: old_min,
                    },
                    SnarkEpochFees {
                        total: old_epoch_total,
                        max: old_epoch_max,
                        min: old_epoch_min,
                    },
                )) => {
                    // all-time fees
                    let total_fees = *old_total + *total;
                    let max_fee = *old_max.max(max);
                    let min_fee = *old_min.min(min);

                    self.delete_old_all_time_snark_fees(prover, *old_total, *old_max, *old_min)?;
                    self.store_all_time_snark_fees(prover, total_fees, max_fee, min_fee)?;
                    self.sort_all_time_snark_fees(prover, total_fees, max_fee, min_fee)?;

                    // epoch fees
                    let epoch_total_fees = *old_epoch_total + *total;
                    let epoch_max_fee = *old_epoch_max.max(max);
                    let epoch_min_fee = *old_epoch_min.min(min);

                    self.delete_old_epoch_snark_fees(
                        epoch,
                        genesis_state_hash,
                        prover,
                        *old_epoch_total,
                        *old_epoch_max,
                        *old_epoch_min,
                    )?;
                    self.store_epoch_snark_fees(
                        epoch,
                        genesis_state_hash,
                        prover,
                        epoch_total_fees,
                        epoch_max_fee,
                        epoch_min_fee,
                    )?;
                    self.sort_epoch_snark_fees(
                        epoch,
                        genesis_state_hash,
                        prover,
                        epoch_total_fees,
                        epoch_max_fee,
                        epoch_min_fee,
                    )?;

                    match apply {
                        // record new historical fees on apply
                        SnarkApplication::Apply => {
                            self.database.put_cf(
                                self.snark_prover_fees_historical_cf(),
                                pk_index_key(prover, block_height),
                                serde_json::to_vec(&SnarkAllTimeFees {
                                    total: total_fees,
                                    max: max_fee,
                                    min: min_fee,
                                })?,
                            )?;
                            self.database.put_cf(
                                self.snark_prover_fees_epoch_historical_cf(),
                                snarks_epoch_pk_height_key(
                                    genesis_state_hash,
                                    epoch,
                                    prover,
                                    block_height,
                                ),
                                serde_json::to_vec(&SnarkEpochFees {
                                    total: epoch_total_fees,
                                    max: epoch_max_fee,
                                    min: epoch_min_fee,
                                })?,
                            )?;
                        }

                        // delete old historical fees on unapply
                        SnarkApplication::Unapply => {
                            self.database.delete_cf(
                                self.snark_prover_fees_historical_cf(),
                                pk_index_key(prover, block_height),
                            )?;
                            self.database.delete_cf(
                                self.snark_prover_fees_epoch_historical_cf(),
                                snarks_epoch_pk_height_key(
                                    genesis_state_hash,
                                    epoch,
                                    prover,
                                    block_height,
                                ),
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn store_all_time_snark_fees(
        &self,
        prover: &PublicKey,
        total_fees: u64,
        max_fee: u64,
        min_fee: u64,
    ) -> Result<()> {
        trace!(
            "Storing all-time SNARK fees pk {} fees ({}, {}, {})",
            prover,
            total_fees,
            max_fee,
            min_fee,
        );

        self.database.put_cf(
            self.snark_prover_fees_cf(),
            prover.0.as_bytes(),
            total_fees.to_be_bytes(),
        )?;
        self.database.put_cf(
            self.snark_prover_max_fee_cf(),
            prover.0.as_bytes(),
            max_fee.to_be_bytes(),
        )?;
        self.database.put_cf(
            self.snark_prover_min_fee_cf(),
            prover.0.as_bytes(),
            min_fee.to_be_bytes(),
        )?;

        Ok(())
    }

    fn sort_all_time_snark_fees(
        &self,
        prover: &PublicKey,
        total_fees: u64,
        max_fee: u64,
        min_fee: u64,
    ) -> Result<()> {
        trace!(
            "Sorting all-time SNARK fees pk {} fees ({}, {}, {})",
            prover,
            total_fees,
            max_fee,
            min_fee,
        );

        self.database.put_cf(
            self.snark_prover_total_fees_sort_cf(),
            u64_prefix_key(total_fees, prover),
            b"",
        )?;
        self.database.put_cf(
            self.snark_prover_max_fee_sort_cf(),
            u64_prefix_key(max_fee, prover),
            b"",
        )?;
        self.database.put_cf(
            self.snark_prover_min_fee_sort_cf(),
            u64_prefix_key(min_fee, prover),
            b"",
        )?;

        Ok(())
    }

    fn delete_old_all_time_snark_fees(
        &self,
        prover: &PublicKey,
        old_total: u64,
        old_max: u64,
        old_min: u64,
    ) -> Result<()> {
        trace!("Deleting all-time SNARK fees pk {}", prover);

        // stored values
        let prover_bytes = prover.0.as_bytes();
        self.database
            .delete_cf(self.snark_prover_fees_cf(), prover_bytes)?;
        self.database
            .delete_cf(self.snark_prover_max_fee_cf(), prover_bytes)?;
        self.database
            .delete_cf(self.snark_prover_min_fee_cf(), prover_bytes)?;

        // sort values
        self.database.delete_cf(
            self.snark_prover_total_fees_sort_cf(),
            u64_prefix_key(old_total, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_max_fee_sort_cf(),
            u64_prefix_key(old_max, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_min_fee_sort_cf(),
            u64_prefix_key(old_min, prover),
        )?;

        Ok(())
    }

    fn store_epoch_snark_fees(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        prover: &PublicKey,
        epoch_total_fees: u64,
        epoch_max_fee: u64,
        epoch_min_fee: u64,
    ) -> Result<()> {
        trace!(
            "Storing epoch SNARK fees pk {} epoch {} genesis {} fees ({}, {}, {})",
            prover,
            epoch,
            genesis_state_hash,
            epoch_total_fees,
            epoch_max_fee,
            epoch_min_fee,
        );

        self.database.put_cf(
            self.snark_prover_fees_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
            epoch_total_fees.to_be_bytes(),
        )?;
        self.database.put_cf(
            self.snark_prover_max_fee_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
            epoch_max_fee.to_be_bytes(),
        )?;
        self.database.put_cf(
            self.snark_prover_min_fee_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
            epoch_min_fee.to_be_bytes(),
        )?;

        Ok(())
    }

    fn sort_epoch_snark_fees(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        prover: &PublicKey,
        epoch_total_fees: u64,
        epoch_max_fee: u64,
        epoch_min_fee: u64,
    ) -> Result<()> {
        trace!(
            "Sorting epoch SNARK fees pk {} epoch {} genesis {}",
            prover,
            epoch,
            genesis_state_hash,
        );

        self.database.put_cf(
            self.snark_prover_total_fees_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, epoch_total_fees, prover),
            b"",
        )?;
        self.database.put_cf(
            self.snark_prover_max_fee_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, epoch_max_fee, prover),
            b"",
        )?;
        self.database.put_cf(
            self.snark_prover_min_fee_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, epoch_min_fee, prover),
            b"",
        )?;

        Ok(())
    }

    fn delete_old_epoch_snark_fees(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        prover: &PublicKey,
        old_epoch_total: u64,
        old_epoch_max: u64,
        old_epoch_min: u64,
    ) -> Result<()> {
        trace!(
            "Deleting old epoch SNARK fees pk {} epoch {} genesis {}",
            prover,
            epoch,
            genesis_state_hash,
        );

        // stored values
        self.database.delete_cf(
            self.snark_prover_fees_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_max_fee_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_min_fee_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, prover),
        )?;

        // sort values
        self.database.delete_cf(
            self.snark_prover_total_fees_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, old_epoch_total, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_max_fee_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, old_epoch_max, prover),
        )?;
        self.database.delete_cf(
            self.snark_prover_min_fee_epoch_sort_cf(),
            snark_fee_epoch_sort_key(genesis_state_hash, epoch, old_epoch_min, prover),
        )?;

        Ok(())
    }

    fn get_top_snark_provers_by_total_fees(&self, n: usize) -> Result<Vec<SnarkWorkTotal>> {
        trace!("Getting top {} SNARK workers by fees", n);

        Ok(self
            .snark_prover_total_fees_iterator(IteratorMode::End)
            .take(n)
            .map(|res| {
                res.map(|(bytes, _)| SnarkWorkTotal {
                    total_fees: u64_from_be_bytes(&bytes[..U64_LEN])
                        .expect("total fees")
                        .into(),
                    prover: PublicKey::from_bytes(&bytes[U64_LEN..]).expect("public key"),
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
    ) -> Result<()> {
        trace!(
            "Setting SNARK block height {} index {} prover {}",
            block_height,
            index,
            snark.prover,
        );

        Ok(self.database.put_cf(
            self.snark_prover_block_height_sort_cf(),
            snark_prover_sort_key(&snark.prover, block_height, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    fn set_snark_by_prover_global_slot(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> Result<()> {
        trace!(
            "Setting SNARK global slot {} at index {} for prover {}",
            global_slot,
            index,
            snark.prover,
        );

        Ok(self.database.put_cf(
            self.snark_prover_global_slot_sort_cf(),
            snark_prover_sort_key(&snark.prover, global_slot, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    fn get_snark_prover_total_fees(
        &self,
        pk: &PublicKey,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        trace!(
            "Getting SNARK total fees pk {} max block height {:?}",
            pk,
            block_height,
        );

        Ok(match block_height {
            None => self
                .database
                .get_cf(self.snark_prover_fees_cf(), pk.0.as_bytes())?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK total fees")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(pk, block_height, None, None)?
                .map(|bytes| {
                    let fees: SnarkAllTimeFees =
                        serde_json::from_slice(&bytes).expect("SNARK all-time fees");
                    fees.total
                }),
        })
    }

    fn get_snark_prover_epoch_fees(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        let epoch = epoch.unwrap_or_else(|| self.get_current_epoch().expect("current epoch"));
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting SNARK fees epoch {} pk {} genesis {} max height {:?}",
            epoch,
            pk,
            genesis_state_hash,
            block_height,
        );

        Ok(match block_height {
            None => self
                .database
                .get_cf(
                    self.snark_prover_fees_epoch_cf(),
                    snarks_pk_epoch_key(genesis_state_hash, epoch, pk),
                )?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK epoch fees")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(
                    pk,
                    block_height,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?
                .map(|bytes| {
                    let fees: SnarkEpochFees =
                        serde_json::from_slice(&bytes).expect("SNARK epoch fees");
                    fees.total
                }),
        })
    }

    fn get_snark_prover_max_fee(
        &self,
        pk: &PublicKey,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        trace!("Getting SNARK max fee for {pk}");

        Ok(match block_height {
            None => self
                .database
                .get_cf(self.snark_prover_max_fee_cf(), pk.0.as_bytes())?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK max fee")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(pk, block_height, None, None)?
                .map(|bytes| {
                    let fees: SnarkAllTimeFees =
                        serde_json::from_slice(&bytes).expect("SNARK all-time fees");
                    fees.max
                }),
        })
    }

    fn get_snark_prover_epoch_max_fee(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        let epoch = epoch.unwrap_or_else(|| self.get_current_epoch().expect("current epoch"));
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting SNARK fees pk {} epoch {} genesis {} max height {:?}",
            pk,
            epoch,
            genesis_state_hash,
            block_height,
        );

        Ok(match block_height {
            None => self
                .database
                .get_cf(
                    self.snark_prover_max_fee_epoch_cf(),
                    snarks_pk_epoch_key(genesis_state_hash, epoch, pk),
                )?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK epoch max fee")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(
                    pk,
                    block_height,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?
                .map(|bytes| {
                    let fees: SnarkEpochFees =
                        serde_json::from_slice(&bytes).expect("SNARK epoch fees");
                    fees.max
                }),
        })
    }

    fn get_snark_prover_min_fee(
        &self,
        pk: &PublicKey,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        trace!("Getting SNARK min fee pk {}", pk);

        Ok(match block_height {
            None => self
                .database
                .get_cf(self.snark_prover_min_fee_cf(), pk.0.as_bytes())?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK min fee")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(pk, block_height, None, None)?
                .map(|bytes| {
                    let fees: SnarkAllTimeFees =
                        serde_json::from_slice(&bytes).expect("SNARK all-time fees");
                    fees.min
                }),
        })
    }

    fn get_snark_prover_epoch_min_fee(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
        block_height: Option<u32>,
    ) -> Result<Option<u64>> {
        let epoch = epoch.unwrap_or_else(|| self.get_current_epoch().expect("current epoch"));
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting SNARK epoch fees pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        Ok(match block_height {
            None => self
                .database
                .get_cf(
                    self.snark_prover_min_fee_epoch_cf(),
                    snarks_pk_epoch_key(genesis_state_hash, epoch, pk),
                )?
                .map(|bytes| u64_from_be_bytes(&bytes).expect("SNARK epoch min fee")),
            Some(block_height) => self
                .get_snark_prover_prev_fees(
                    pk,
                    block_height,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?
                .map(|bytes| {
                    let fees: SnarkEpochFees =
                        serde_json::from_slice(&bytes).expect("SNARK epoch fees");
                    fees.min
                }),
        })
    }

    fn update_block_snarks(&self, blocks: &DbBlockUpdate) -> Result<()> {
        let snark_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .map(
                    |BlockUpdate {
                         epoch,
                         state_hash: a,
                         global_slot_since_genesis,
                         blockchain_length,
                     }| {
                        let block_snarks = self.get_block_snark_work(a).ok().flatten().unwrap();
                        SnarkUpdate {
                            epoch: *epoch,
                            state_hash: a.clone(),
                            blockchain_length: *blockchain_length,
                            global_slot_since_genesis: *global_slot_since_genesis,
                            works: block_snarks,
                        }
                    },
                )
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .map(
                    |BlockUpdate {
                         epoch,
                         state_hash: u,
                         blockchain_length,
                         global_slot_since_genesis,
                     }| {
                        let block_snarks = self.get_block_snark_work(u).ok().flatten().unwrap();
                        SnarkUpdate {
                            epoch: *epoch,
                            state_hash: u.clone(),
                            blockchain_length: *blockchain_length,
                            global_slot_since_genesis: *global_slot_since_genesis,
                            works: block_snarks,
                        }
                    },
                )
                .collect(),
        };

        self.update_snarks(snark_updates)
    }

    fn update_snarks(&self, update: DbSnarkUpdate) -> Result<()> {
        trace!("Updating SNARKs");

        // unapply
        for snark_update in update.unapply {
            let genesis_state_hash = self
                .get_block_genesis_state_hash(&snark_update.state_hash)?
                .expect("block genesis state hash");

            self.decrement_snarks_total_canonical_count(snark_update.works.len() as u32)?;
            self.update_snark_prover_fees(
                snark_update.epoch,
                snark_update.blockchain_length,
                &genesis_state_hash,
                &snark_update.works,
                SnarkApplication::Unapply,
            )?;
        }

        // apply
        for snark_update in update.apply {
            let genesis_state_hash = self
                .get_block_genesis_state_hash(&snark_update.state_hash)?
                .expect("block genesis state hash");

            self.increment_snarks_total_canonical_count(snark_update.works.len() as u32)?;
            self.update_snark_prover_fees(
                snark_update.epoch,
                snark_update.blockchain_length,
                &genesis_state_hash,
                &snark_update.works,
                SnarkApplication::Apply,
            )?;
        }

        Ok(())
    }

    fn get_snark_prover_prev_fees(
        &self,
        prover: &PublicKey,
        block_height: u32,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<Option<Vec<u8>>> {
        // use appropriate CF for iteration
        match (epoch, genesis_state_hash) {
            (None, _) | (_, None) => {
                if let Some((key, value)) = self
                    .database
                    .iterator_cf(
                        self.snark_prover_fees_historical_cf(),
                        IteratorMode::From(
                            &pk_index_key(prover, block_height - 1),
                            Direction::Reverse,
                        ),
                    )
                    .flatten()
                    .next()
                {
                    if key[..PublicKey::LEN] != *prover.0.as_bytes() {
                        // gone beyond desired prover
                        return Ok(None);
                    }

                    let block_height =
                        u32_from_be_bytes(&key[PublicKey::LEN..]).expect("u32 block height");
                    if let Some(epoch) = epoch {
                        if let Some(state_hash) = self.get_canonical_hash_at_height(block_height)? {
                            if let Some(block_epoch) = self.get_block_epoch(&state_hash)? {
                                if block_epoch != epoch {
                                    return Ok(None);
                                }
                            } else {
                                return Ok(None);
                            }
                        } else {
                            return Ok(None);
                        }
                    }

                    return Ok(Some(value.to_vec()));
                }
            }
            (Some(epoch), Some(genesis_state_hash)) => {
                if let Some((key, value)) = self
                    .database
                    .iterator_cf(
                        self.snark_prover_fees_epoch_historical_cf(),
                        IteratorMode::From(
                            &snarks_epoch_pk_height_key(
                                genesis_state_hash,
                                epoch,
                                prover,
                                block_height - 1,
                            ),
                            Direction::Reverse,
                        ),
                    )
                    .flatten()
                    .next()
                {
                    if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                        || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                        || key[StateHash::LEN..][U32_LEN..][..PublicKey::LEN]
                            != *prover.0.as_bytes()
                    {
                        // gone beyond desired epoch/prover
                        return Ok(None);
                    }

                    let block_height =
                        u32_from_be_bytes(&key[StateHash::LEN..][U32_LEN..][PublicKey::LEN..])
                            .expect("u32 block height");
                    if let Some(state_hash) = self.get_canonical_hash_at_height(block_height)? {
                        if let Some(block_epoch) = self.get_block_epoch(&state_hash)? {
                            if block_epoch != epoch {
                                return Ok(None);
                            }
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }

                    return Ok(Some(value.to_vec()));
                }
            }
        }

        Ok(None)
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

    fn snark_prover_max_fee_epoch_iterator(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        direction: Direction,
    ) -> DBIterator<'_> {
        self.database.iterator_cf(
            self.snark_prover_max_fee_epoch_sort_cf(),
            IteratorMode::From(&start_key(genesis_state_hash, epoch, direction), direction),
        )
    }

    fn snark_prover_min_fee_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_min_fee_sort_cf(), mode)
    }

    fn snark_prover_min_fee_epoch_iterator(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        direction: Direction,
    ) -> DBIterator<'_> {
        self.database.iterator_cf(
            self.snark_prover_min_fee_epoch_sort_cf(),
            IteratorMode::From(&start_key(genesis_state_hash, epoch, direction), direction),
        )
    }

    fn snark_prover_total_fees_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.snark_prover_total_fees_sort_cf(), mode)
    }

    fn snark_prover_total_fees_epoch_iterator(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
        direction: Direction,
    ) -> DBIterator<'_> {
        self.database.iterator_cf(
            self.snark_prover_total_fees_epoch_sort_cf(),
            IteratorMode::From(&start_key(genesis_state_hash, epoch, direction), direction),
        )
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

    fn get_snarks_epoch_count(
        &self,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32> {
        let epoch = epoch.unwrap_or_else(|| self.get_current_epoch().expect("current epoch"));
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting SNARK count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.snarks_epoch_cf(),
                snarks_epoch_key(genesis_state_hash, epoch),
            )?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("epoch SNARK count")
            }))
    }

    fn increment_snarks_epoch_count(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing SNARK count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        let old = self.get_snarks_epoch_count(Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.snarks_epoch_cf(),
            snarks_epoch_key(genesis_state_hash, epoch),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_snarks_total_count(&self) -> Result<u32> {
        trace!("Getting total SNARKs count");
        Ok(self
            .database
            .get_pinned(Self::TOTAL_NUM_SNARKS_KEY)?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("total SNARK count")
            }))
    }

    fn get_snarks_total_canonical_count(&self) -> Result<u32> {
        trace!("Getting total canonical SNARKs count");
        Ok(self
            .database
            .get_pinned(Self::TOTAL_NUM_CANONICAL_SNARKS_KEY)?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("total canonical SNARK count")
            }))
    }

    fn increment_snarks_total_canonical_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing total canonical SNARKs count");
        let old = self
            .get_snarks_total_canonical_count()
            .ok()
            .unwrap_or_default();
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_SNARKS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn decrement_snarks_total_canonical_count(&self, decr: u32) -> Result<()> {
        trace!("Incrementing total canonical SNARKs count");
        let old = self
            .get_snarks_total_canonical_count()
            .ok()
            .unwrap_or_default();
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_SNARKS_KEY,
            (old.saturating_sub(decr)).to_be_bytes(),
        )?)
    }

    fn increment_snarks_total_count(&self) -> Result<()> {
        trace!("Incrementing total SNARKs count");
        let old = self.get_snarks_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_SNARKS_KEY, (old + 1).to_be_bytes())?)
    }

    fn get_snarks_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32> {
        let epoch = epoch.unwrap_or_else(|| self.get_current_epoch().expect("current epoch"));
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting SNARK count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.snarks_pk_epoch_cf(),
                snarks_pk_epoch_key(genesis_state_hash, epoch, pk),
            )?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("pk epoch SNARK count")
            }))
    }

    fn increment_snarks_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing SNARK count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        let old = self.get_snarks_pk_epoch_count(pk, Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.snarks_pk_epoch_cf(),
            snarks_pk_epoch_key(genesis_state_hash, epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_snarks_pk_total_count(&self, pk: &PublicKey) -> Result<u32> {
        trace!("Getting pk total SNARKs count {pk}");

        Ok(self
            .database
            .get_cf(self.snarks_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, |bytes| {
                u32_from_be_bytes(&bytes).expect("pk total SNARK count")
            }))
    }

    fn increment_snarks_pk_total_count(&self, pk: &PublicKey) -> Result<()> {
        trace!("Incrementing pk total SNARKs count {pk}");

        let old = self.get_snarks_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.snarks_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_block_snarks_count(&self, state_hash: &StateHash) -> Result<Option<u32>> {
        trace!("Getting block SNARKs count {state_hash}");

        Ok(self
            .database
            .get_cf(self.block_snark_counts_cf(), state_hash.0.as_bytes())?
            .map(|bytes| u32_from_be_bytes(&bytes).expect("block SNARK count")))
    }

    fn set_block_snarks_count(&self, state_hash: &StateHash, count: u32) -> Result<()> {
        trace!("Setting block SNARKs count {state_hash} -> {count}");

        Ok(self.database.put_cf(
            self.block_snark_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        )?)
    }

    fn increment_snarks_counts(
        &self,
        snark: &SnarkWorkSummary,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing SNARK count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        // prover epoch & total
        self.increment_snarks_pk_epoch_count(&snark.prover, epoch, genesis_state_hash)?;
        self.increment_snarks_pk_total_count(&snark.prover)?;

        // epoch & total counts
        self.increment_snarks_epoch_count(epoch, genesis_state_hash)?;
        self.increment_snarks_total_count()
    }
}

fn start_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    direction: Direction,
) -> [u8; StateHash::LEN + U32_LEN + U64_LEN + PublicKey::LEN] {
    let mut start = [0; StateHash::LEN + U32_LEN + U64_LEN + PublicKey::LEN];

    // start in the desired epoch
    start[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    start[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());

    if let Direction::Reverse = direction {
        start[StateHash::LEN..][U32_LEN..][..U64_LEN].copy_from_slice(&u64::MAX.to_be_bytes());
        start[StateHash::LEN..][U32_LEN..][U64_LEN..]
            .copy_from_slice(PublicKey::upper_bound().0.as_bytes());
    }

    start
}

#[cfg(all(test, feature = "tier2"))]
mod tests {
    use super::SnarkStore;
    use crate::store::IndexerStore;
    use std::env;
    use tempfile::TempDir;

    fn create_indexer_store() -> anyhow::Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(env::current_dir()?)?;
        IndexerStore::new(temp_dir.path(), true)
    }

    #[test]
    fn test_incr_dec_snarks_total_canonical_count() -> anyhow::Result<()> {
        let store = create_indexer_store()?;

        store.increment_snarks_total_canonical_count(1)?;
        assert_eq!(store.get_snarks_total_canonical_count()?, 1);

        store.increment_snarks_total_canonical_count(1)?;
        assert_eq!(store.get_snarks_total_canonical_count()?, 2);

        store.decrement_snarks_total_canonical_count(1)?;
        assert_eq!(store.get_snarks_total_canonical_count()?, 1);

        store.decrement_snarks_total_canonical_count(1)?;
        assert_eq!(store.get_snarks_total_canonical_count()?, 0);

        store.decrement_snarks_total_canonical_count(1)?;
        assert_eq!(store.get_snarks_total_canonical_count()?, 0);

        Ok(())
    }
}
