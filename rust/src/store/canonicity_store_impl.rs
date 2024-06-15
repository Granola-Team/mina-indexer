use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity, CanonicityDiff, CanonicityUpdate},
    event::{db::*, store::EventStore, IndexerEvent},
    snark_work::store::SnarkStore,
    store::{to_be_bytes, DBUpdate, IndexerStore},
};
use anyhow::Context;
use log::trace;

impl CanonicityStore for IndexerStore {
    fn add_canonical_block(
        &self,
        height: u32,
        global_slot: u32,
        state_hash: &BlockHash,
        genesis_state_hash: &BlockHash,
        genesis_prev_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<()> {
        if state_hash == genesis_state_hash && genesis_prev_state_hash.is_some() {
            trace!("Adding new genesis block (length {height}): {state_hash}");
        } else {
            trace!("Adding canonical block (length {height}): {state_hash}");
        }

        // height -> state hash
        self.database.put_cf(
            self.canonicity_length_cf(),
            to_be_bytes(height),
            state_hash.0.as_bytes(),
        )?;

        // slot -> state hash
        self.database.put_cf(
            self.canonicity_slot_cf(),
            to_be_bytes(global_slot),
            state_hash.0.as_bytes(),
        )?;

        // update top snarkers based on the incoming canonical block
        if let Some(completed_works) = self.get_snark_work_in_block(state_hash)? {
            self.update_top_snarkers(completed_works)?;
        }

        // record new genesis/prev state hashes
        if let Some(genesis_prev_state_hash) = genesis_prev_state_hash {
            let (mut genesis_state_hashes, mut genesis_prev_state_hashes) = (
                self.get_known_genesis_state_hashes()?,
                self.get_known_genesis_prev_state_hashes()?,
            );

            // check if genesis hash is present
            if !genesis_state_hashes.contains(genesis_state_hash) {
                // if not
                // add genesis state hash
                genesis_state_hashes.push(genesis_state_hash.clone());
                self.database.put(
                    Self::KNOWN_GENESIS_STATE_HASHES_KEY,
                    serde_json::to_vec(&genesis_state_hashes)?,
                )?;

                // add genesis prev state hash
                genesis_prev_state_hashes.push(genesis_prev_state_hash.clone());
                self.database.put(
                    Self::KNOWN_GENESIS_PREV_STATE_HASHES_KEY,
                    serde_json::to_vec(&genesis_prev_state_hashes)?,
                )?;
            }
        }

        // record new canonical block event
        self.add_event(&IndexerEvent::Db(DbEvent::Canonicity(
            DbCanonicityEvent::NewCanonicalBlock {
                blockchain_length: height,
                state_hash: state_hash.0.clone().into(),
            },
        )))?;
        Ok(())
    }

    fn get_known_genesis_state_hashes(&self) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting known genesis state hashes");
        Ok(self
            .database
            .get_pinned(Self::KNOWN_GENESIS_STATE_HASHES_KEY)?
            .map_or(vec![], |bytes| {
                serde_json::from_slice(&bytes).expect("known genesis state hashes")
            }))
    }

    fn get_known_genesis_prev_state_hashes(&self) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting known genesis prev state hashes");
        Ok(self
            .database
            .get_pinned(Self::KNOWN_GENESIS_PREV_STATE_HASHES_KEY)?
            .map_or(vec![], |bytes| {
                serde_json::from_slice(&bytes).expect("known genesis prev state hashes")
            }))
    }

    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting canonical state hash at height {height}");
        Ok(self
            .database
            .get_pinned_cf(&self.canonicity_length_cf(), to_be_bytes(height))?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn get_canonical_hash_at_slot(&self, global_slot: u32) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting canonical state hash at slot {global_slot}");
        Ok(self
            .database
            .get_pinned_cf(&self.canonicity_slot_cf(), to_be_bytes(global_slot))?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn get_block_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        trace!("Getting canonicity of block {state_hash}");
        if let Ok(Some(height)) = self.get_block_height(state_hash) {
            return Ok(self
                .get_canonical_hash_at_height(height)?
                .map(|canonical_hash| {
                    if *state_hash == canonical_hash {
                        Canonicity::Canonical
                    } else {
                        Canonicity::Orphaned
                    }
                }));
        }
        Ok(None)
    }

    fn reorg_canonicity_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<CanonicityUpdate> {
        trace!("Getting reorg canonicity updates:\n  old: {old_best_tip}\n  new: {new_best_tip}");

        // follows the old best tip back to the common ancestor
        let mut a = old_best_tip.clone();
        let mut unapply = vec![];

        // follows the new best tip back to the common ancestor
        let mut b = new_best_tip.clone();
        let mut apply = vec![];

        let a_length = self.get_block_height(&a)?.expect("a has a length");
        let b_length = self.get_block_height(&b)?.expect("b has a length");

        // bring b back to the same height as a
        let genesis_state_hashes: Vec<BlockHash> = self.get_known_genesis_state_hashes()?;
        for _ in 0..(b_length - a_length) {
            // check if there's a previous block
            if genesis_state_hashes.contains(&b) {
                break;
            }

            apply.push(CanonicityDiff {
                state_hash: b.clone(),
                blockchain_length: b_length,
                global_slot: self
                    .get_block_global_slot(&b)?
                    .with_context(|| format!("(length {b_length}): {b}"))
                    .unwrap(),
            });
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && !genesis_state_hashes.contains(&a) {
            // collect canonicity diffs
            unapply.push(CanonicityDiff {
                state_hash: a.clone(),
                blockchain_length: self.get_block_height(&a)?.unwrap(),
                global_slot: self.get_block_global_slot(&a)?.unwrap(),
            });
            apply.push(CanonicityDiff {
                state_hash: b.clone(),
                blockchain_length: self.get_block_height(&b)?.unwrap(),
                global_slot: self.get_block_global_slot(&b)?.unwrap(),
            });

            // descend
            a = a_prev;
            b = b_prev;

            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }
        Ok(DBUpdate { apply, unapply })
    }

    fn update_canonicity(&self, updates: CanonicityUpdate) -> anyhow::Result<()> {
        trace!("Updating block canonicities: {updates:?}");
        // unapply canonicities
        for unapply in updates.unapply.iter() {
            // remove from canonicity sets
            self.database.delete_cf(
                self.canonicity_length_cf(),
                to_be_bytes(unapply.blockchain_length),
            )?;
            self.database
                .delete_cf(self.canonicity_slot_cf(), to_be_bytes(unapply.global_slot))?;
        }

        // apply canonicities
        for apply in updates.apply.iter() {
            // remove from canonicity sets
            self.database.put_cf(
                self.canonicity_length_cf(),
                to_be_bytes(apply.blockchain_length),
                apply.state_hash.0.as_bytes(),
            )?;
            self.database.put_cf(
                self.canonicity_slot_cf(),
                to_be_bytes(apply.global_slot),
                apply.state_hash.0.as_bytes(),
            )?;
        }
        Ok(())
    }
}
