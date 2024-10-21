use super::{
    column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, username::UsernameStore, DbUpdate,
    IndexerStore,
};
use crate::{
    block::{
        precomputed::{PcbVersion, PrecomputedBlock},
        store::{BlockStore, BlockUpdate, DbBlockUpdate},
        BlockComparison, BlockHash,
    },
    canonicity::{store::CanonicityStore, Canonicity},
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::{MAINNET_EPOCH_SLOT_COUNT, MAINNET_GENESIS_HASH, MAINNET_GENESIS_PREV_STATE_HASH},
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        coinbase::Coinbase,
        diff::{account::AccountDiff, LedgerDiff},
        public_key::PublicKey,
        store::{best::BestLedgerStore, staged::StagedLedgerStore},
    },
    snark_work::store::SnarkStore,
    utility::store::{
        block::*, block_u32_prefix_from_key, from_be_bytes, i64_from_be_bytes, pk_index_key,
        state_hash_suffix, u32_from_be_bytes, u32_prefix_key, u64_from_be_bytes, U32_LEN, U64_LEN,
    },
};
use anyhow::{bail, Context};
use log::{error, trace};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};

impl BlockStore for IndexerStore {
    /// Add the given block at its indices and record a db event
    fn add_block(
        &self,
        block: &PrecomputedBlock,
        num_block_bytes: u64,
    ) -> anyhow::Result<Option<DbEvent>> {
        trace!("Adding block {}", block.summary());

        // add block to db - prefix with num bytes (u64) BE bytes
        let state_hash = block.state_hash();
        let mut value = num_block_bytes.to_be_bytes().to_vec();
        value.append(&mut serde_json::to_vec(block)?);

        if matches!(
            self.database
                .get_cf(self.blocks_state_hash_cf(), state_hash.0.as_bytes()),
            Ok(Some(_))
        ) {
            trace!("Block already present {}", block.summary());
            return Ok(None);
        }

        let mut batch = WriteBatch::default();
        batch.put_cf(self.blocks_state_hash_cf(), state_hash.0.as_bytes(), b"");
        batch.put_cf(self.blocks_cf(), state_hash.0.as_bytes(), value);

        // add to ledger diff index
        self.set_block_ledger_diff_batch(
            &state_hash,
            &LedgerDiff::from_precomputed(block),
            &mut batch,
        )?;

        // add to epoch index before setting other indices
        self.set_block_epoch_batch(&state_hash, block.epoch_count(), &mut batch)?;

        // increment block production counts
        self.increment_block_production_count_batch(block, &mut batch)?;

        // add comparison data before user commands, SNARKs, and internal commands
        self.set_block_comparison_batch(&state_hash, &BlockComparison::from(block))?;

        // add to blockchain length index
        self.set_block_height_batch(&state_hash, block.blockchain_length(), &mut batch)?;

        // add to block global slot index
        self.set_block_global_slot_batch(
            &state_hash,
            block.global_slot_since_genesis(),
            &mut batch,
        )?;

        // add to parent hash index
        self.set_block_parent_hash_batch(&state_hash, &block.previous_state_hash(), &mut batch)?;

        // add to date time index
        self.set_block_date_time_batch(&state_hash, block.timestamp() as i64, &mut batch)?;

        // add to staged ledger hash index
        self.set_block_staged_ledger_hash_batch(
            &state_hash,
            &block.staged_ledger_hash(),
            &mut batch,
        )?;

        // add to genesis state hash index
        let genesis_state_hash = block.genesis_state_hash();
        if genesis_state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH {
            self.set_block_genesis_state_hash_batch(
                &state_hash,
                &MAINNET_GENESIS_HASH.into(),
                &mut batch,
            )?;
        } else {
            self.set_block_genesis_state_hash_batch(&state_hash, &genesis_state_hash, &mut batch)?;
        }

        // add block height/global slot index
        self.set_block_height_global_slot_pair_batch(
            block.blockchain_length(),
            block.global_slot_since_genesis(),
            &mut batch,
        )?;

        // add to block creator index
        self.set_block_creator_batch(block, &mut batch)?;

        // add to coinbase receiver index
        self.set_coinbase_receiver_batch(block, &mut batch)?;

        // add block height/global slot for sorting
        batch.put_cf(self.blocks_height_sort_cf(), block_height_key(block), b"");
        batch.put_cf(
            self.blocks_global_slot_sort_cf(),
            block_global_slot_key(block),
            b"",
        );

        // add block for each public key
        for pk in block.all_public_keys() {
            self.add_block_at_public_key_batch(&pk, &state_hash, &mut batch)?;
        }

        // add block to height list
        self.add_block_at_height_batch(&state_hash, block.blockchain_length(), &mut batch)?;

        // add block to slots list
        self.add_block_at_slot_batch(&state_hash, block.global_slot_since_genesis(), &mut batch)?;

        // add pcb's version
        self.set_block_version_batch(&state_hash, block.version(), &mut batch)?;

        // add block user commands
        self.add_user_commands_batch(block, &mut batch)?;

        // add block internal commands
        self.add_internal_commands_batch(block, &mut batch)?;

        // write the batch
        trace!(
            "Writing {} bytes to database from batch",
            batch.size_in_bytes()
        );
        self.database.write(batch)?;

        // add epoch produced slot
        self.add_epoch_slots_produced(
            block.epoch_count(),
            block.global_slot_since_genesis() % MAINNET_EPOCH_SLOT_COUNT,
            &block.block_creator(),
        )?;

        // add block SNARK work
        self.add_snark_work(block)?;

        // increment bytes processed
        let bytes_processed = self
            .database
            .get(Self::NUM_BLOCK_BYTES_PROCESSED)?
            .map_or(0, |bytes| {
                u64_from_be_bytes(&bytes).expect("bytes processed u64 BE bytes")
            });
        self.database.put(
            Self::NUM_BLOCK_BYTES_PROCESSED,
            (bytes_processed + num_block_bytes).to_be_bytes(),
        )?;

        // add new block db event only after all other data is added
        let db_event = DbEvent::Block(DbBlockEvent::NewBlock {
            state_hash: block.state_hash(),
            blockchain_length: block.blockchain_length(),
        });
        self.add_event(&IndexerEvent::Db(db_event.clone()))?;
        Ok(Some(db_event))
    }

    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<(PrecomputedBlock, u64)>> {
        trace!("Getting block {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.blocks_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| {
                serde_json::from_slice::<PrecomputedBlock>(&bytes[U64_LEN..])
                    .with_context(|| format!("{:?}", bytes.to_vec()))
                    .ok()
                    .map(|block| {
                        (
                            block,
                            u64_from_be_bytes(&bytes[..U64_LEN]).expect("block bytes u64 BE bytes"),
                        )
                    })
            }))
    }

    //////////////////////////
    // Best block functions //
    //////////////////////////

    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting best block");
        match self.get_best_block_hash()? {
            None => Ok(None),
            Some(state_hash) => Ok(self.get_block(&state_hash)?.map(|b| b.0)),
        }
    }

    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting best block hash");
        Ok(self
            .database
            .get(Self::BEST_TIP_STATE_HASH_KEY)?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn get_best_block_height(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_height(&state_hash).ok().flatten()))
    }

    fn get_best_block_global_slot(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_global_slot(&state_hash).ok().flatten()))
    }

    fn get_best_block_genesis_hash(&self) -> anyhow::Result<Option<BlockHash>> {
        Ok(self.get_best_block_hash()?.and_then(|state_hash| {
            self.get_block_genesis_state_hash(&state_hash)
                .ok()
                .flatten()
        }))
    }

    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!("Setting best block {state_hash}");
        if let Some(old) = self.get_best_block_hash()? {
            if old == *state_hash {
                return Ok(());
            }

            // reorg updates
            let reorg_blocks = self.reorg_blocks(&old, state_hash)?;
            self.update_block_canonicities(&reorg_blocks)?;
            self.update_block_best_accounts(state_hash, &reorg_blocks)?;
            self.update_block_snarks(&reorg_blocks)?;
            self.update_block_usernames(&reorg_blocks)?;
            self.update_internal_commands(&reorg_blocks)?;
        }

        // set new best tip
        self.database
            .put(Self::BEST_TIP_STATE_HASH_KEY, state_hash.0.as_bytes())?;

        // record new best tip event
        match self.get_block_height(state_hash)? {
            Some(blockchain_length) => {
                self.add_event(&IndexerEvent::Db(DbEvent::Block(
                    DbBlockEvent::NewBestTip {
                        state_hash: state_hash.clone(),
                        blockchain_length,
                    },
                )))?;
            }
            None => error!("Block missing from store: {state_hash}"),
        }
        Ok(())
    }

    fn reorg_blocks(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<DbBlockUpdate> {
        trace!(
            "Getting common ancestor account balance updates:\n  old: {}\n  new: {}",
            old_best_tip,
            new_best_tip
        );

        // follows the old best tip back to the common ancestor
        let mut a = old_best_tip.clone();
        let mut unapply = vec![];

        // follows the new best tip back to the common ancestor
        let mut b = new_best_tip.clone();
        let mut apply = vec![];
        let b_length = self.get_block_height(&b)?.expect("b has length");

        // bring b back to the same height as a
        for _ in 0..b_length.saturating_sub(self.get_block_height(&a)?.expect("a has length")) {
            // check if there's a previous block
            if b.0 == MAINNET_GENESIS_HASH {
                break;
            }
            apply.push(BlockUpdate {
                state_hash: b.clone(),
                blockchain_length: b_length,
                global_slot_since_genesis: self
                    .get_block_global_slot(&b)?
                    .expect("b has global slot"),
            });
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && a.0 != MAINNET_GENESIS_HASH {
            // add blocks to appropriate collection
            apply.push(BlockUpdate {
                state_hash: b.clone(),
                blockchain_length: self.get_block_height(&b)?.expect("b has length"),
                global_slot_since_genesis: self
                    .get_block_global_slot(&b)?
                    .expect("b has global slot"),
            });
            unapply.push(BlockUpdate {
                state_hash: a.clone(),
                blockchain_length: self.get_block_height(&a)?.expect("a has length"),
                global_slot_since_genesis: self
                    .get_block_global_slot(&a)?
                    .expect("a has global slot"),
            });

            // descend
            a = a_prev;
            b = b_prev;
            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        apply.reverse();
        Ok(DbUpdate { apply, unapply })
    }

    fn get_current_epoch(&self) -> anyhow::Result<u32> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_epoch(&state_hash).ok().flatten())
            .unwrap_or_default())
    }

    /////////////////////////////
    // General block functions //
    /////////////////////////////

    fn get_block_account_diffs(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<AccountDiff>>> {
        trace!("Getting block account diffs for {state_hash}");
        Ok(self
            .get_block_ledger_diff(state_hash)?
            .map(|diff| diff.account_diffs.into_iter().flatten().collect::<Vec<_>>()))
    }

    fn get_block_ledger_diff(&self, state_hash: &BlockHash) -> anyhow::Result<Option<LedgerDiff>> {
        trace!("Getting block ledger diff {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.block_ledger_diff_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_block_parent_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting block's parent hash {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_parent_hash_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn set_block_parent_hash_batch(
        &self,
        state_hash: &BlockHash,
        previous_state_hash: &BlockHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block parent hash {state_hash}: {previous_state_hash}");
        batch.put_cf(
            self.block_parent_hash_cf(),
            state_hash.0.as_bytes(),
            previous_state_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_block_date_time(&self, state_hash: &BlockHash) -> anyhow::Result<Option<i64>> {
        trace!("Getting block date time {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_date_time_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| i64_from_be_bytes(&bytes).ok()))
    }

    fn set_block_date_time_batch(
        &self,
        state_hash: &BlockHash,
        date_time: i64,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block date time {state_hash}");
        batch.put_cf(
            self.block_date_time_cf(),
            state_hash.0.as_bytes(),
            date_time.to_be_bytes(),
        );
        Ok(())
    }

    fn get_block_height(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block height {state_hash}");
        if state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH {
            return Ok(Some(0));
        }
        Ok(self
            .database
            .get_cf(self.block_height_cf(), state_hash.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn set_block_height_batch(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block height {state_hash}: {blockchain_length}");
        batch.put_cf(
            self.block_height_cf(),
            state_hash.0.as_bytes(),
            blockchain_length.to_be_bytes(),
        );
        Ok(())
    }

    fn get_block_global_slot(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block global slot {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_global_slot_cf(), state_hash.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn set_block_global_slot_batch(
        &self,
        state_hash: &BlockHash,
        global_slot: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block global slot {state_hash}: {global_slot}");
        batch.put_cf(
            self.block_global_slot_cf(),
            state_hash.0.as_bytes(),
            global_slot.to_be_bytes(),
        );
        Ok(())
    }

    fn get_block_creator(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting block creator {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_creator_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn set_block_creator_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        let block_creator = block.block_creator();
        trace!("Setting block creator: {state_hash} -> {block_creator}");

        // index
        batch.put_cf(
            self.block_creator_cf(),
            state_hash.0.as_bytes(),
            block_creator.0.as_bytes(),
        );

        // block height sort
        batch.put_cf(
            self.block_creator_height_sort_cf(),
            pk_block_sort_key(&block_creator, block.blockchain_length(), &state_hash),
            b"",
        );

        // global slot sort
        batch.put_cf(
            self.block_creator_slot_sort_cf(),
            pk_block_sort_key(
                &block_creator,
                block.global_slot_since_genesis(),
                &state_hash,
            ),
            b"",
        );
        Ok(())
    }

    fn get_coinbase_receiver(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting coinbase receiver for {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_coinbase_receiver_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn set_coinbase_receiver_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        let coinbase_receiver = block.coinbase_receiver();
        trace!("Setting coinbase receiver: {state_hash} -> {coinbase_receiver}");

        // index
        batch.put_cf(
            self.block_coinbase_receiver_cf(),
            state_hash.0.as_bytes(),
            coinbase_receiver.0.as_bytes(),
        );

        // block height sort
        batch.put_cf(
            self.block_coinbase_height_sort_cf(),
            pk_block_sort_key(&coinbase_receiver, block.blockchain_length(), &state_hash),
            b"",
        );

        // global slot sort
        batch.put_cf(
            self.block_coinbase_slot_sort_cf(),
            pk_block_sort_key(
                &coinbase_receiver,
                block.global_slot_since_genesis(),
                &state_hash,
            ),
            b"",
        );
        Ok(())
    }

    fn get_num_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at height {blockchain_length}");
        Ok(self
            .database
            .get_cf(self.blocks_at_height_cf(), blockchain_length.to_be_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn add_block_at_height_batch(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at height {blockchain_length}");

        // increment num blocks at height
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        batch.put_cf(
            self.blocks_at_height_cf(),
            blockchain_length.to_be_bytes(),
            (num_blocks_at_height + 1).to_be_bytes(),
        );

        // add the new key-value pair
        batch.put_cf(
            self.blocks_at_height_cf(),
            block_num_key(blockchain_length, num_blocks_at_height),
            state_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<Vec<BlockHash>> {
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_height {
            match self.database.get_cf(
                self.blocks_at_height_cf(),
                block_num_key(blockchain_length, n),
            )? {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }
        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_num_blocks_at_slot(&self, slot: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at slot {slot}");
        Ok(self
            .database
            .get_cf(self.blocks_at_global_slot_cf(), slot.to_be_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn add_block_at_slot_batch(
        &self,
        state_hash: &BlockHash,
        slot: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at slot {slot}");

        // increment num blocks at slot
        let num_blocks_at_slot = self.get_num_blocks_at_slot(slot)?;
        batch.put_cf(
            self.blocks_at_global_slot_cf(),
            slot.to_be_bytes(),
            (num_blocks_at_slot + 1).to_be_bytes(),
        );

        // add the new key-value pair
        batch.put_cf(
            self.blocks_at_global_slot_cf(),
            block_num_key(slot, num_blocks_at_slot),
            state_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_blocks_at_slot(&self, slot: u32) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting blocks at slot {slot}");
        let mut blocks = vec![];

        for n in 0..self.get_num_blocks_at_slot(slot)? {
            match self
                .database
                .get_cf(self.blocks_at_global_slot_cf(), block_num_key(slot, n))?
            {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }

        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at public key {pk}");
        Ok(self
            .database
            .get_cf(self.blocks_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn add_block_at_public_key_batch(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at public key {pk}");

        // increment num blocks at public key
        let num_blocks_at_pk = self.get_num_blocks_at_public_key(pk)?;
        batch.put_cf(
            self.blocks_cf(),
            pk.0.as_bytes(),
            (num_blocks_at_pk + 1).to_be_bytes(),
        );

        // add the new key-value pair
        batch.put_cf(
            self.blocks_cf(),
            pk_index_key(pk, num_blocks_at_pk),
            state_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting blocks at public key {pk}");
        let mut blocks = vec![];

        for n in 0..self.get_num_blocks_at_public_key(pk)? {
            match self
                .database
                .get_cf(self.blocks_cf(), pk_index_key(pk, n))?
            {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }

        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting children of block {state_hash}");
        if let Some(height) = self.get_block_height(state_hash)? {
            let blocks_at_next_height = self.get_blocks_at_height(height + 1)?;
            let mut children: Vec<BlockHash> = blocks_at_next_height
                .into_iter()
                .filter(|b| {
                    self.get_block_parent_hash(b).ok().flatten() == Some(state_hash.clone())
                })
                .collect();
            children.sort_by(|a, b| block_cmp(self, a, b));
            return Ok(children);
        }
        bail!("Block missing from store {state_hash}")
    }

    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>> {
        trace!("Getting block version {state_hash}");
        let key = state_hash.0.as_bytes();
        Ok(self
            .database
            .get_pinned_cf(self.block_version_cf(), key)?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn set_block_version_batch(
        &self,
        state_hash: &BlockHash,
        version: PcbVersion,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block {state_hash} version to {version}");
        batch.put_cf(
            self.block_version_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&version)?,
        );
        Ok(())
    }

    fn set_block_height_global_slot_pair_batch(
        &self,
        blockchain_length: u32,
        global_slot: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block height {blockchain_length} <-> slot {global_slot}");

        // add height to slot's "height collection"
        let mut heights = self
            .get_block_heights_from_global_slot(global_slot)?
            .unwrap_or_default();
        if !heights.contains(&blockchain_length) {
            heights.push(blockchain_length);
            heights.sort();
            batch.put_cf(
                self.block_global_slot_to_heights_cf(),
                global_slot.to_be_bytes(),
                serde_json::to_vec(&heights)?,
            );
        }

        // add slot to height's "slot collection"
        let mut slots = self
            .get_block_global_slots_from_height(blockchain_length)?
            .unwrap_or_default();
        if !slots.contains(&global_slot) {
            slots.push(global_slot);
            slots.sort();
            batch.put_cf(
                self.block_height_to_global_slots_cf(),
                blockchain_length.to_be_bytes(),
                serde_json::to_vec(&slots)?,
            );
        }
        Ok(())
    }

    fn get_block_global_slots_from_height(
        &self,
        blockchain_length: u32,
    ) -> anyhow::Result<Option<Vec<u32>>> {
        trace!("Getting global slot for height {blockchain_length}");
        Ok(self
            .database
            .get_pinned_cf(
                self.block_height_to_global_slots_cf(),
                blockchain_length.to_be_bytes(),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_block_heights_from_global_slot(
        &self,
        global_slot: u32,
    ) -> anyhow::Result<Option<Vec<u32>>> {
        trace!("Getting height for global slot {global_slot}");
        Ok(self
            .database
            .get_pinned_cf(
                self.block_global_slot_to_heights_cf(),
                global_slot.to_be_bytes(),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn set_block_epoch_batch(
        &self,
        state_hash: &BlockHash,
        epoch: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block epoch {epoch}: {state_hash}");
        batch.put_cf(
            self.block_epoch_cf(),
            state_hash.0.as_bytes(),
            epoch.to_be_bytes(),
        );
        Ok(())
    }

    fn get_block_epoch(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block epoch {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_epoch_cf(), state_hash.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn set_block_genesis_state_hash_batch(
        &self,
        state_hash: &BlockHash,
        genesis_state_hash: &BlockHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block genesis state hash {state_hash}: {genesis_state_hash}");
        batch.put_cf(
            self.block_genesis_state_hash_cf(),
            state_hash.0.as_bytes(),
            genesis_state_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_block_genesis_state_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting block genesis state hash {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_genesis_state_hash_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn add_epoch_slots_produced(
        &self,
        epoch: u32,
        epoch_slot: u32,
        pk: &PublicKey,
    ) -> anyhow::Result<()> {
        trace!("Adding epoch {epoch} slot {epoch_slot} produced");

        // add to total
        let key = block_num_key(epoch, epoch_slot);
        if self
            .database
            .get_cf(self.block_epoch_slots_produced_cf(), key)?
            .is_none()
        {
            // add the epoch slot
            self.database
                .put_cf(self.block_epoch_slots_produced_cf(), key, b"")?;

            // increment epoch slots produced count
            let acc = self.get_epoch_slots_produced_count(Some(epoch))?;
            self.database.put_cf(
                self.block_epoch_slots_produced_count_cf(),
                epoch.to_be_bytes(),
                (acc + 1).to_be_bytes(),
            )?;
        }

        // add to account
        let key = epoch_pk_num_key(epoch, pk, epoch_slot);
        if self
            .database
            .get_cf(self.block_pk_epoch_slots_produced_cf(), key)?
            .is_none()
        {
            // add the epoch slot
            self.database
                .put_cf(self.block_pk_epoch_slots_produced_cf(), key, b"")?;

            // increment epoch slots produced count
            let acc = self.get_pk_epoch_slots_produced_count(pk, Some(epoch))?;
            self.database.put_cf(
                self.block_pk_epoch_slots_produced_count_cf(),
                epoch_pk_key(epoch, pk),
                (acc + 1).to_be_bytes(),
            )?;
        }
        Ok(())
    }

    fn get_next_global_slot_produced(&self, global_slot: u32) -> anyhow::Result<Option<u32>> {
        trace!("Getting next slot produced at or above {global_slot}");
        let epoch = global_slot / MAINNET_EPOCH_SLOT_COUNT;
        let epoch_slot = global_slot % MAINNET_EPOCH_SLOT_COUNT;

        if let Some((key, _)) = self
            .database
            .iterator_cf(
                self.block_epoch_slots_produced_cf(),
                IteratorMode::From(&block_num_key(epoch, epoch_slot), Direction::Forward),
            )
            .flatten()
            .next()
        {
            let epoch = u32_from_be_bytes(&key[..U32_LEN]).expect("epoch u32 bytes");
            let epoch_slot = u32_from_be_bytes(&key[U32_LEN..]).expect("epoch slot u32 bytes");
            return Ok(Some(epoch * MAINNET_EPOCH_SLOT_COUNT + epoch_slot));
        }
        Ok(None)
    }

    fn get_prev_global_slot_produced(&self, global_slot: u32) -> anyhow::Result<u32> {
        trace!("Getting previous slot produced at or below {global_slot}");
        let epoch = global_slot / MAINNET_EPOCH_SLOT_COUNT;
        let epoch_slot = global_slot % MAINNET_EPOCH_SLOT_COUNT;

        if let Some((key, _)) = self
            .database
            .iterator_cf(
                self.block_epoch_slots_produced_cf(),
                IteratorMode::From(&block_num_key(epoch, epoch_slot), Direction::Reverse),
            )
            .flatten()
            .next()
        {
            let epoch = u32_from_be_bytes(&key[..U32_LEN]).expect("epoch u32 bytes");
            let epoch_slot = u32_from_be_bytes(&key[U32_LEN..]).expect("epoch slot u32 bytes");
            return Ok(epoch * MAINNET_EPOCH_SLOT_COUNT + epoch_slot);
        }
        Ok(0)
    }

    ///////////////
    // Iterators //
    ///////////////

    fn blocks_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.blocks_height_sort_cf(), mode)
    }

    fn blocks_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.blocks_global_slot_sort_cf(), mode)
    }

    fn block_creator_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.block_creator_height_sort_cf(), mode)
    }

    fn block_creator_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.block_creator_slot_sort_cf(), mode)
    }

    fn coinbase_receiver_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.block_coinbase_height_sort_cf(), mode)
    }

    fn coinbase_receiver_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.block_coinbase_slot_sort_cf(), mode)
    }

    fn canonical_epoch_blocks_produced_iterator(
        &self,
        epoch: Option<u32>,
        direction: Direction,
    ) -> DBIterator<'_> {
        let epoch = epoch.unwrap_or(self.get_current_epoch().expect("current epoch"));
        let epoch_be_bytes = epoch.to_be_bytes();
        let mut start = [0; U32_LEN + U32_LEN + PublicKey::LEN];
        match direction {
            Direction::Forward => {
                // start at the beginning of the epoch
                start[..U32_LEN].copy_from_slice(&epoch_be_bytes);
                start[U32_LEN..][..U32_LEN].copy_from_slice(&0u32.to_be_bytes());
                start[U32_LEN..][U32_LEN..].copy_from_slice(PublicKey::lower_bound().0.as_bytes());
            }
            Direction::Reverse => {
                // start at the end of the epoch
                start[..U32_LEN].copy_from_slice(&epoch_be_bytes);
                start[U32_LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
                start[U32_LEN..][U32_LEN..].copy_from_slice(PublicKey::upper_bound().0.as_bytes());
            }
        }
        self.database.iterator_cf(
            self.block_production_pk_canonical_epoch_sort_cf(),
            IteratorMode::From(start.as_slice(), direction),
        )
    }

    //////////////////
    // Block counts //
    //////////////////

    fn increment_block_production_count_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Incrementing block production count {}", block.summary());
        let creator = block.block_creator();
        let epoch = block.epoch_count();

        // increment pk epoch count
        let acc = self.get_block_production_pk_epoch_count(&creator, Some(epoch))?;
        batch.put_cf(
            self.block_production_pk_epoch_cf(),
            u32_prefix_key(epoch, &creator),
            (acc + 1).to_be_bytes(),
        );

        // increment pk total count
        let acc = self.get_block_production_pk_total_count(&creator)?;
        batch.put_cf(
            self.block_production_pk_total_cf(),
            creator.0.as_bytes(),
            (acc + 1).to_be_bytes(),
        );

        // increment epoch count
        let acc = self.get_block_production_epoch_count(Some(epoch))?;
        batch.put_cf(
            self.block_production_epoch_cf(),
            epoch.to_be_bytes(),
            (acc + 1).to_be_bytes(),
        );

        // increment total count
        let acc = self.get_block_production_total_count()?;
        batch.put(Self::TOTAL_NUM_BLOCKS_KEY, (acc + 1).to_be_bytes());

        // supercharged counts
        if Coinbase::from_precomputed(block).supercharge {
            // pk epoch supercharged
            let acc =
                self.get_block_production_pk_supercharged_epoch_count(&creator, Some(epoch))?;
            batch.put_cf(
                self.block_production_pk_supercharged_epoch_cf(),
                u32_prefix_key(epoch, &creator),
                (acc + 1).to_be_bytes(),
            );

            // pk total supercharged
            let acc = self.get_block_production_pk_supercharged_total_count(&creator)?;
            batch.put_cf(
                self.block_production_pk_supercharged_total_cf(),
                creator.0.as_bytes(),
                (acc + 1).to_be_bytes(),
            );

            // epoch supercharged
            let acc = self.get_block_production_supercharged_epoch_count(Some(epoch))?;
            batch.put_cf(
                self.block_production_supercharged_epoch_cf(),
                epoch.to_be_bytes(),
                (acc + 1).to_be_bytes(),
            );

            // total supercharged
            let acc = self.get_block_production_supercharged_total_count()?;
            batch.put(
                Self::TOTAL_NUM_BLOCKS_SUPERCHARGED_KEY,
                (acc + 1).to_be_bytes(),
            );
        }
        Ok(())
    }

    fn increment_block_production_count(
        &self,
        state_hash: &BlockHash,
        creator: &PublicKey,
        supercharged: bool,
    ) -> anyhow::Result<()> {
        trace!("Incrementing block production count {state_hash}");

        // increment pk epoch count
        let acc = self.get_block_production_pk_epoch_count(creator, Some(0))?;
        self.database.put_cf(
            self.block_production_pk_epoch_cf(),
            u32_prefix_key(0, creator),
            (acc + 1).to_be_bytes(),
        )?;

        // increment pk total count
        let acc = self.get_block_production_pk_total_count(creator)?;
        self.database.put_cf(
            self.block_production_pk_total_cf(),
            creator.0.as_bytes(),
            (acc + 1).to_be_bytes(),
        )?;

        // increment epoch count
        let acc = self.get_block_production_epoch_count(Some(0))?;
        self.database.put_cf(
            self.block_production_epoch_cf(),
            0u32.to_be_bytes(),
            (acc + 1).to_be_bytes(),
        )?;

        // increment total count
        let acc = self.get_block_production_total_count()?;
        self.database
            .put(Self::TOTAL_NUM_BLOCKS_KEY, (acc + 1).to_be_bytes())?;

        // supercharged counts
        if supercharged {
            // pk epoch supercharged
            let acc = self.get_block_production_pk_supercharged_epoch_count(creator, Some(0))?;
            self.database.put_cf(
                self.block_production_pk_supercharged_epoch_cf(),
                u32_prefix_key(0, creator),
                (acc + 1).to_be_bytes(),
            )?;

            // pk total supercharged
            let acc = self.get_block_production_pk_supercharged_total_count(creator)?;
            self.database.put_cf(
                self.block_production_pk_supercharged_total_cf(),
                creator.0.as_bytes(),
                (acc + 1).to_be_bytes(),
            )?;

            // epoch supercharged
            let acc = self.get_block_production_supercharged_epoch_count(Some(0))?;
            self.database.put_cf(
                self.block_production_supercharged_epoch_cf(),
                0u32.to_be_bytes(),
                (acc + 1).to_be_bytes(),
            )?;

            // total supercharged
            let acc = self.get_block_production_supercharged_total_count()?;
            self.database.put(
                Self::TOTAL_NUM_BLOCKS_SUPERCHARGED_KEY,
                (acc + 1).to_be_bytes(),
            )?;
        }
        Ok(())
    }

    fn increment_block_canonical_production_count(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Incrementing canonical block production count {state_hash}");
        let creator = self.get_block_creator(state_hash)?.expect("block creator");
        let epoch = self.get_block_epoch(state_hash)?.expect("block epoch");

        // increment pk epoch count
        let acc = self.get_block_production_pk_canonical_epoch_count(&creator, Some(epoch))?;
        self.database.put_cf(
            self.block_production_pk_canonical_epoch_cf(),
            u32_prefix_key(epoch, &creator),
            (acc + 1).to_be_bytes(),
        )?;
        self.increment_block_canonical_production_count_sort(epoch, acc, &creator)?;

        // increment pk total count
        let acc = self.get_block_production_pk_canonical_total_count(&creator)?;
        self.database.put_cf(
            self.block_production_pk_canonical_total_cf(),
            creator.0.as_bytes(),
            (acc + 1).to_be_bytes(),
        )?;

        // increment epoch count
        let acc = self.get_block_production_canonical_epoch_count(Some(epoch))?;
        self.database.put_cf(
            self.block_production_canonical_epoch_cf(),
            epoch.to_be_bytes(),
            (acc + 1).to_be_bytes(),
        )?;
        Ok(())
    }

    fn increment_block_canonical_production_count_sort(
        &self,
        epoch: u32,
        num: u32,
        pk: &PublicKey,
    ) -> anyhow::Result<()> {
        trace!(
            "Incrementing epoch {epoch} pk {pk} canonical block sort: {num} -> {}",
            num + 1
        );
        self.database.delete_cf(
            self.block_production_pk_canonical_epoch_sort_cf(),
            epoch_block_num_key(epoch, num, pk),
        )?;
        self.database.put_cf(
            self.block_production_pk_canonical_epoch_sort_cf(),
            epoch_block_num_key(epoch, num + 1, pk),
            b"",
        )?;
        Ok(())
    }

    fn decrement_block_canonical_production_count(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Decrementing canonical block production count {state_hash}");
        let creator = self.get_block_creator(state_hash)?.expect("block creator");
        let epoch = self.get_block_epoch(state_hash)?.expect("block epoch");

        // decrement pk epoch count
        let acc = self.get_block_production_pk_canonical_epoch_count(&creator, Some(epoch))?;
        assert!(acc > 0);
        self.database.put_cf(
            self.block_production_pk_canonical_epoch_cf(),
            u32_prefix_key(epoch, &creator),
            (acc - 1).to_be_bytes(),
        )?;
        self.decrement_block_canonical_production_count_sort(epoch, acc, &creator)?;

        // decrement pk total count
        let acc = self.get_block_production_pk_canonical_total_count(&creator)?;
        assert!(acc > 0);
        self.database.put_cf(
            self.block_production_pk_canonical_total_cf(),
            creator.0.as_bytes(),
            (acc - 1).to_be_bytes(),
        )?;

        // decrement epoch count
        let acc = self.get_block_production_canonical_epoch_count(Some(epoch))?;
        assert!(acc > 0);
        self.database.put_cf(
            self.block_production_canonical_epoch_cf(),
            epoch.to_be_bytes(),
            (acc - 1).to_be_bytes(),
        )?;
        Ok(())
    }

    fn decrement_block_canonical_production_count_sort(
        &self,
        epoch: u32,
        num: u32,
        pk: &PublicKey,
    ) -> anyhow::Result<()> {
        assert!(num > 0);
        trace!(
            "Decrementing epoch {epoch} pk {pk} canonical block sort: {num} -> {}",
            num - 1
        );
        self.database.delete_cf(
            self.block_production_pk_canonical_epoch_sort_cf(),
            epoch_block_num_key(epoch, num, pk),
        )?;
        self.database.put_cf(
            self.block_production_pk_canonical_epoch_sort_cf(),
            epoch_block_num_key(epoch, num - 1, pk),
            b"",
        )?;
        Ok(())
    }

    fn get_block_production_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} block production count {pk}");
        Ok(self
            .database
            .get_cf(
                self.block_production_pk_epoch_cf(),
                u32_prefix_key(epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_canonical_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} canonical block production count {pk}");
        Ok(self
            .database
            .get_cf(
                self.block_production_pk_canonical_epoch_cf(),
                u32_prefix_key(epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_supercharged_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} supercharged block production count {pk}");
        Ok(self
            .database
            .get_cf(
                self.block_production_pk_supercharged_epoch_cf(),
                u32_prefix_key(epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total block production count {pk}");
        Ok(self
            .database
            .get_cf(self.block_production_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_canonical_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total canonical block production count {pk}");
        Ok(self
            .database
            .get_cf(
                self.block_production_pk_canonical_total_cf(),
                pk.0.as_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_supercharged_total_count(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<u32> {
        trace!("Getting pk total supercharged block production count {pk}");
        Ok(self
            .database
            .get_cf(
                self.block_production_pk_supercharged_total_cf(),
                pk.0.as_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch block production count {epoch}");
        Ok(self
            .database
            .get_cf(self.block_production_epoch_cf(), epoch.to_be_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_canonical_epoch_count(
        &self,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch canonical block production count {epoch}");
        Ok(self
            .database
            .get_cf(
                self.block_production_canonical_epoch_cf(),
                epoch.to_be_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_supercharged_epoch_count(
        &self,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch supercharged block production count {epoch}");
        Ok(self
            .database
            .get_cf(
                self.block_production_supercharged_epoch_cf(),
                epoch.to_be_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total block production count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_BLOCKS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_canonical_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total canonical block production count");
        self.get_best_block_height()
            .map(|res| res.unwrap_or_default())
    }

    fn get_block_production_supercharged_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total supercharged block production count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_BLOCKS_SUPERCHARGED_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_pk_epoch_slots_produced_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch {epoch} pk {pk} slots produced count");
        Ok(self
            .database
            .get_cf(
                self.block_pk_epoch_slots_produced_count_cf(),
                epoch_pk_key(epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_epoch_slots_produced_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch {epoch} slots produced count");
        Ok(self
            .database
            .get_cf(
                self.block_epoch_slots_produced_count_cf(),
                epoch.to_be_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn set_block_comparison_batch(
        &self,
        state_hash: &BlockHash,
        comparison: &BlockComparison,
    ) -> anyhow::Result<()> {
        trace!("Setting block comparison {state_hash}");
        Ok(self.database.put_cf(
            self.block_comparison_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(comparison)?,
        )?)
    }

    fn get_block_comparison(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockComparison>> {
        trace!("Getting block comparison {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.block_comparison_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn block_cmp(
        &self,
        block: &BlockHash,
        other: &BlockHash,
    ) -> anyhow::Result<Option<std::cmp::Ordering>> {
        // get stored block comparisons
        let res1 = self
            .database
            .get_cf(self.block_comparison_cf(), block.0.as_bytes());
        let res2 = self
            .database
            .get_cf(self.block_comparison_cf(), other.0.as_bytes());

        // compare stored block comparisons
        if let (Ok(Some(bytes1)), Ok(Some(bytes2))) = (res1, res2) {
            let bc1: BlockComparison = serde_json::from_slice(&bytes1)?;
            let bc2: BlockComparison = serde_json::from_slice(&bytes2)?;
            return Ok(Some(bc1.cmp(&bc2)));
        }
        Ok(None)
    }

    fn dump_blocks_via_height(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::{fs::File, io::Write};
        trace!("Dumping blocks via height to {}", path.display());
        let mut file = File::create(path)?;

        for (key, _) in self
            .blocks_height_iterator(speedb::IteratorMode::Start)
            .flatten()
        {
            let state_hash = state_hash_suffix(&key)?;
            let block_height = block_u32_prefix_from_key(&key)?;
            let global_slot = self
                .get_block_global_slot(&state_hash)?
                .expect("global slot");

            writeln!(
                file,
                "height: {block_height}\nslot:   {global_slot}\nstate:  {state_hash}"
            )?;
        }
        Ok(())
    }

    fn blocks_via_height(&self, mode: IteratorMode) -> anyhow::Result<Vec<PrecomputedBlock>> {
        let mut blocks = vec![];
        trace!("Getting blocks via height (mode: {})", display_mode(mode));
        for (key, _) in self.blocks_height_iterator(mode).flatten() {
            let state_hash = state_hash_suffix(&key)?;
            blocks.push(self.get_block(&state_hash)?.expect("PCB").0);
        }
        Ok(blocks)
    }

    fn dump_blocks_via_global_slot(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::{fs::File, io::Write};
        trace!("Dumping blocks via global slot to {}", path.display());
        let mut file = File::create(path)?;

        for (key, _) in self
            .blocks_global_slot_iterator(speedb::IteratorMode::Start)
            .flatten()
        {
            let state_hash = state_hash_suffix(&key)?;
            let block_height = block_u32_prefix_from_key(&key)?;
            let global_slot = self
                .get_block_global_slot(&state_hash)?
                .expect("global slot");

            writeln!(
                file,
                "height: {block_height}\nslot:   {global_slot}\nstate:  {state_hash}"
            )?;
        }
        Ok(())
    }

    fn blocks_via_global_slot(&self, mode: IteratorMode) -> anyhow::Result<Vec<PrecomputedBlock>> {
        let mut blocks = vec![];
        trace!(
            "Getting blocks via global slot (mode: {})",
            display_mode(mode)
        );
        for (key, _) in self.blocks_global_slot_iterator(mode).flatten() {
            let state_hash = state_hash_suffix(&key)?;
            blocks.push(self.get_block(&state_hash)?.expect("PCB").0);
        }
        Ok(blocks)
    }
}

fn block_cmp(db: &IndexerStore, a: &BlockHash, b: &BlockHash) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let a_canonicity = db.get_block_canonicity(a).ok().flatten();
    let b_canonicity = db.get_block_canonicity(b).ok().flatten();
    let a_cmp = db.get_block_comparison(a).unwrap().unwrap();
    let b_cmp = db.get_block_comparison(b).unwrap().unwrap();
    match (a_canonicity, b_canonicity) {
        (Some(Canonicity::Canonical), _) => Ordering::Less,
        (_, Some(Canonicity::Canonical)) => Ordering::Greater,
        _ => a_cmp.cmp(&b_cmp),
    }
}

fn display_mode(mode: IteratorMode) -> String {
    match mode {
        IteratorMode::End => "End".to_string(),
        IteratorMode::Start => "Start".to_string(),
        IteratorMode::From(start, direction) => {
            format!("{} from {start:?}", display_direction(direction))
        }
    }
}

fn display_direction(direction: Direction) -> String {
    match direction {
        Direction::Forward => "Forward".to_string(),
        Direction::Reverse => "Reverse".to_string(),
    }
}
