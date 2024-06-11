use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    event::{db::*, store::EventStore, IndexerEvent},
    snark_work::store::SnarkStore,
    store::{from_be_bytes, to_be_bytes, IndexerStore},
};
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

        // update canonical chain length
        self.set_max_canonical_blockchain_length(height)?;

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

    fn get_max_canonical_blockchain_length(&self) -> anyhow::Result<Option<u32>> {
        trace!("Getting max canonical blockchain length");
        Ok(self
            .database
            .get(Self::MAX_CANONICAL_KEY)?
            .map(from_be_bytes))
    }

    fn set_max_canonical_blockchain_length(&self, height: u32) -> anyhow::Result<()> {
        trace!("Setting max canonical blockchain length to {height}");
        self.database
            .put(Self::MAX_CANONICAL_KEY, to_be_bytes(height))?;
        Ok(())
    }

    fn get_block_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        trace!("Getting canonicity of block with hash {}", state_hash.0);

        if let (Ok(Some(best_tip)), Ok(Some(blockchain_length))) =
            (self.get_best_block(), self.get_block_height(state_hash))
        {
            if blockchain_length > best_tip.blockchain_length() {
                return Ok(None);
            } else if let Some(max_canonical_length) = self.get_max_canonical_blockchain_length()? {
                if blockchain_length > max_canonical_length {
                    // follow best chain back from tip to given block
                    let mut curr_block = best_tip;
                    while curr_block.state_hash() != *state_hash
                        && curr_block.blockchain_length() > max_canonical_length
                    {
                        if let Some(parent) = self.get_block(&curr_block.previous_state_hash())? {
                            curr_block = parent;
                        } else {
                            break;
                        }
                    }

                    return Ok(Some(
                        if curr_block.state_hash() == *state_hash
                            && curr_block.blockchain_length() > max_canonical_length
                        {
                            Canonicity::Canonical
                        } else {
                            Canonicity::Orphaned
                        },
                    ));
                } else {
                    return Ok(Some(
                        if self.get_canonical_hash_at_height(blockchain_length)?
                            == Some(state_hash.clone())
                        {
                            Canonicity::Canonical
                        } else {
                            Canonicity::Orphaned
                        },
                    ));
                }
            }
        }
        Ok(None)
    }
}
