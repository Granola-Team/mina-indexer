use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, DbUpdate};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity, CanonicityDiff, CanonicityUpdate},
    event::{db::*, store::EventStore, IndexerEvent},
    snark_work::store::SnarkStore,
    store::{to_be_bytes, IndexerStore},
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

    fn update_block_canonicities(&self, blocks: &DbUpdate<(BlockHash, u32)>) -> anyhow::Result<()> {
        let canonicity_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .map(|(a, h)| CanonicityDiff {
                    state_hash: a.clone(),
                    blockchain_length: *h,
                    global_slot: self
                        .get_block_global_slot(a)
                        .unwrap()
                        .with_context(|| format!("(length {h}): {a}"))
                        .expect("block global slot exists"),
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .map(|(u, h)| CanonicityDiff {
                    state_hash: u.clone(),
                    blockchain_length: *h,
                    global_slot: self
                        .get_block_global_slot(u)
                        .unwrap()
                        .with_context(|| format!("(length {h}): {u}"))
                        .expect("block global slot exists"),
                })
                .collect(),
        };
        self.update_canonicity(canonicity_updates)
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
            // put into canonicity sets
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
