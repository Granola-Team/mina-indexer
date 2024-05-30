use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
        BlockHash,
    },
    canonicity::store::CanonicityStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        diff::{account::PaymentDiff, LedgerBalanceUpdate},
        public_key::PublicKey,
        store::LedgerStore,
    },
    snark_work::store::SnarkStore,
    store::{
        from_be_bytes, global_slot_block_key, to_be_bytes, u32_prefix_key, u64_prefix_key,
        IndexerStore,
    },
};
use anyhow::bail;
use log::{error, trace};
use std::collections::HashSet;

impl BlockStore for IndexerStore {
    /// Add the given block at its indices and record a db event
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<Option<DbEvent>> {
        trace!("Adding block {}", block.summary());

        // add block to db
        let state_hash = block.state_hash().0;
        let value = serde_json::to_vec(&block)?;

        if matches!(
            self.database
                .get_pinned_cf(self.blocks_cf(), state_hash.as_bytes()),
            Ok(Some(_))
        ) {
            trace!("Block already present {}", block.summary());
            return Ok(None);
        }
        self.database
            .put_cf(self.blocks_cf(), state_hash.as_bytes(), value)?;

        // increment block production counts
        self.increment_block_production_count(block)?;

        // add to coinbase receiver index
        self.set_coinbase_receiver(&block.state_hash(), &block.coinbase_receiver())?;

        // add to blockchain length index
        self.set_blockchain_length(&block.state_hash(), block.blockchain_length())?;

        // add to parent hash index
        self.set_block_parent_hash(&block.state_hash(), &block.previous_state_hash())?;

        // add to balance update index
        self.set_block_balance_updates(
            &block.state_hash(),
            block.coinbase_receiver(),
            LedgerBalanceUpdate::from_precomputed(block),
        )?;

        // add to global slots block index
        self.database.put_cf(
            self.blocks_global_slot_idx_cf(),
            global_slot_block_key(block),
            b"",
        )?;

        // add block for each public key
        for pk in block.all_public_keys() {
            self.add_block_at_public_key(&pk, &block.state_hash())?;
        }

        // add height <-> global slot
        self.set_height_global_slot(block.blockchain_length(), block.global_slot_since_genesis())?;

        // add block to height list
        self.add_block_at_height(&block.state_hash(), block.blockchain_length())?;

        // add block to slots list
        self.add_block_at_slot(&block.state_hash(), block.global_slot_since_genesis())?;

        // add block user commands
        self.add_user_commands(block)?;

        // add block internal commands
        self.add_internal_commands(block)?;

        // add block SNARK work
        self.add_snark_work(block)?;

        // store pcb's version
        self.set_block_version(&block.state_hash(), block.version())?;

        // add new block db event only after all other data is added
        let db_event = DbEvent::Block(DbBlockEvent::NewBlock {
            state_hash: block.state_hash(),
            blockchain_length: block.blockchain_length(),
        });
        self.add_event(&IndexerEvent::Db(db_event.clone()))?;

        Ok(Some(db_event))
    }

    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting block with hash {}", state_hash.0);

        let key = state_hash.0.as_bytes();
        Ok(self
            .database
            .get_pinned_cf(self.blocks_cf(), key)?
            .map(|bytes| serde_json::from_slice(&bytes).unwrap()))
    }

    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting best block");
        match self.get_best_block_hash()? {
            None => Ok(None),
            Some(state_hash) => self.get_block(&state_hash),
        }
    }

    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting best block hash");
        Ok(self
            .database
            .get_pinned_cf(self.blocks_cf(), Self::BEST_TIP_BLOCK_KEY)?
            .map(|bytes| {
                <String as Into<BlockHash>>::into(String::from_utf8(bytes.to_vec()).unwrap())
            }))
    }

    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!("Setting best block");

        if let Some(old) = self.get_best_block_hash()? {
            if old == *state_hash {
                return Ok(());
            }

            let (balance_updates, coinbase_receivers) =
                self.common_ancestor_account_balance_updates(&old, state_hash)?;
            self.update_account_balances(state_hash, balance_updates, coinbase_receivers)?;
        }

        // set new best tip
        self.database.put_cf(
            self.blocks_cf(),
            Self::BEST_TIP_BLOCK_KEY,
            state_hash.0.as_bytes(),
        )?;

        // record new best tip event
        match self.get_block(state_hash)? {
            Some(block) => {
                self.add_event(&IndexerEvent::Db(DbEvent::Block(
                    DbBlockEvent::NewBestTip {
                        state_hash: block.state_hash(),
                        blockchain_length: block.blockchain_length(),
                    },
                )))?;
            }
            None => error!("Block missing from store: {}", state_hash.0),
        }
        Ok(())
    }

    fn get_block_parent_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting block's parent hash {state_hash}");

        Ok(self
            .database
            .get_cf(self.block_parent_hash_cf(), state_hash.0.as_bytes())
            .map(|o| o.map(|bytes| BlockHash::from_bytes(&bytes).expect("parent state hash")))?)
    }

    fn set_block_parent_hash(
        &self,
        state_hash: &BlockHash,
        previous_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting block parent hash {} -> {}",
            state_hash,
            previous_state_hash
        );

        Ok(self.database.put_cf(
            self.block_parent_hash_cf(),
            state_hash.0.as_bytes(),
            previous_state_hash.0.as_bytes(),
        )?)
    }

    fn get_blockchain_length(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting blockchain length {state_hash}");
        Ok(self
            .database
            .get_cf(self.blockchain_length_cf(), state_hash.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn set_blockchain_length(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting blockchain length {blockchain_length}: {state_hash}");
        Ok(self.database.put_cf(
            self.blockchain_length_cf(),
            state_hash.0.as_bytes(),
            to_be_bytes(blockchain_length),
        )?)
    }

    fn get_coinbase_receiver(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting coinbase receiver for {state_hash}");
        Ok(self
            .database
            .get_cf(self.coinbase_receiver_cf(), state_hash.0.as_bytes())
            .map_or(None, |bytes| {
                bytes.map(|b| PublicKey::from_bytes(&b).unwrap())
            }))
    }

    fn set_coinbase_receiver(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: &PublicKey,
    ) -> anyhow::Result<()> {
        trace!("Setting coinbase receiver: {state_hash} -> {coinbase_receiver}");
        Ok(self.database.put_cf(
            self.coinbase_receiver_cf(),
            state_hash.0.as_bytes(),
            coinbase_receiver.0.as_bytes(),
        )?)
    }

    // TODO make modular over different updates
    fn common_ancestor_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<(Vec<PaymentDiff>, HashSet<PublicKey>)> {
        trace!(
            "Getting common ancestor account balance updates:\n  old: {}\n  new: {}",
            old_best_tip.0,
            new_best_tip.0
        );
        let mut coinbase_receivers = HashSet::new();

        // follows the old best tip back to the common ancestor
        let mut a = old_best_tip.clone();
        let mut unapply = vec![];

        // follows the new best tip back to the common ancestor
        let mut b = new_best_tip.clone();
        let mut apply = vec![];

        let a_length = self.get_blockchain_length(&a)?.expect("a has a length");
        let b_length = self.get_blockchain_length(&b)?.expect("b has a length");

        // bring b back to the same height as a
        let genesis_state_hashes: Vec<BlockHash> = self.get_known_genesis_state_hashes()?;
        for _ in 0..(b_length - a_length) {
            // check if there's a previous block
            if genesis_state_hashes.contains(&b) {
                break;
            }

            coinbase_receivers.insert(self.get_coinbase_receiver(&b)?.expect("b has a coinbase"));
            apply.append(&mut self.get_block_balance_updates(&b)?.unwrap().1);
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && !genesis_state_hashes.contains(&a) {
            // retain coinbase receivers
            coinbase_receivers.insert(self.get_coinbase_receiver(&a)?.expect("a has a coinbase"));
            coinbase_receivers.insert(self.get_coinbase_receiver(&b)?.expect("b has a coinbase"));

            // add blocks to appropriate collection
            unapply.append(&mut self.get_block_balance_updates(&a)?.unwrap().1);
            apply.append(&mut self.get_block_balance_updates(&b)?.unwrap().1);

            // descend
            a = a_prev;
            b = b_prev;

            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // balance updates don't require this reverse, but other updates may
        apply.reverse();
        Ok((
            LedgerBalanceUpdate { apply, unapply }.to_diff_vec(),
            coinbase_receivers,
        ))
    }

    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<(PublicKey, Vec<PaymentDiff>)>> {
        trace!("Getting block balance updates for {}", state_hash.0);
        Ok(self
            .database
            .get_cf(self.account_balance_updates_cf(), state_hash.0.as_bytes())?
            .map(|bytes| serde_json::from_slice(&bytes).expect("balance updates")))
    }

    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: Vec<PaymentDiff>,
        coinbase_receivers: HashSet<PublicKey>,
    ) -> anyhow::Result<()> {
        trace!("Updating account balances {state_hash}");

        // update balances
        for (pk, amount) in LedgerBalanceUpdate::balance_updates(updates) {
            if amount != 0 {
                let pk = pk.into();
                let balance = self.get_account_balance(&pk)?.unwrap_or(0);
                let balance = if coinbase_receivers.contains(&pk) && balance == 0 && amount > 0 {
                    (balance + amount.unsigned_abs()).saturating_sub(MAINNET_ACCOUNT_CREATION_FEE.0)
                } else if amount > 0 {
                    balance + amount.unsigned_abs()
                } else {
                    balance.saturating_sub(amount.unsigned_abs())
                };

                // coinbase receivers may need to be removed
                self.update_account_balance(
                    &pk,
                    if coinbase_receivers.contains(&pk) && balance == 0 {
                        None
                    } else {
                        Some(balance)
                    },
                )?;
            }
        }
        Ok(())
    }

    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()> {
        trace!("Updating account balance {pk} -> {balance:?}");

        // update balance info
        if balance.is_none() {
            // delete stale data
            let b = self.get_account_balance(pk)?.unwrap_or(0);
            self.database
                .delete_cf(self.account_balance_cf(), pk.0.as_bytes())?;
            self.database
                .delete_cf(self.account_balance_sort_cf(), u64_prefix_key(b, &pk.0))?;
            return Ok(());
        }

        let balance = balance.unwrap();
        if let Some(old) = self.get_account_balance(pk)? {
            // delete stale balance sorting data
            self.database
                .delete_cf(self.account_balance_sort_cf(), u64_prefix_key(old, &pk.0))?;
        }
        self.database.put_cf(
            self.account_balance_cf(),
            pk.0.as_bytes(),
            balance.to_be_bytes(),
        )?;

        // add: {balance}{pk} -> _
        self.database.put_cf(
            self.account_balance_sort_cf(),
            u64_prefix_key(balance, &pk.0),
            b"",
        )?;
        Ok(())
    }

    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: PublicKey,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()> {
        trace!("Setting block balance updates for {state_hash}");
        self.database.put_cf(
            self.account_balance_updates_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&(coinbase_receiver, balance_updates))?,
        )?;
        Ok(())
    }

    fn get_num_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at height {blockchain_length}");
        Ok(
            match self
                .database
                .get_pinned_cf(self.lengths_cf(), blockchain_length.to_string().as_bytes())?
            {
                None => 0,
                Some(bytes) => String::from_utf8(bytes.to_vec())?.parse()?,
            },
        )
    }

    fn add_block_at_height(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at height {blockchain_length}");

        // increment num blocks at height
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        self.database.put_cf(
            self.lengths_cf(),
            blockchain_length.to_string().as_bytes(),
            (num_blocks_at_height + 1).to_string().as_bytes(),
        )?;

        // add the new key-value pair
        let key = format!("{blockchain_length}-{num_blocks_at_height}");
        Ok(self.database.put_cf(
            self.lengths_cf(),
            key.as_bytes(),
            state_hash.to_string().as_bytes(),
        )?)
    }

    fn get_blocks_at_height(
        &self,
        blockchain_length: u32,
    ) -> anyhow::Result<Vec<PrecomputedBlock>> {
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_height {
            let key = format!("{blockchain_length}-{n}");
            match self
                .database
                .get_pinned_cf(self.lengths_cf(), key.as_bytes())?
                .map(|bytes| bytes.to_vec())
            {
                None => break,
                Some(bytes) => {
                    let state_hash: BlockHash = String::from_utf8(bytes)?.into();
                    if let Some(block) = self.get_block(&state_hash)? {
                        blocks.push(block);
                    }
                }
            }
        }

        blocks.sort();
        Ok(blocks)
    }

    fn get_num_blocks_at_slot(&self, slot: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at slot {slot}");
        Ok(
            match self
                .database
                .get_pinned_cf(self.slots_cf(), slot.to_string().as_bytes())?
            {
                None => 0,
                Some(bytes) => String::from_utf8(bytes.to_vec())?.parse()?,
            },
        )
    }

    fn add_block_at_slot(&self, state_hash: &BlockHash, slot: u32) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at slot {slot}");

        // increment num blocks at slot
        let num_blocks_at_slot = self.get_num_blocks_at_slot(slot)?;
        self.database.put_cf(
            self.slots_cf(),
            slot.to_string().as_bytes(),
            (num_blocks_at_slot + 1).to_string().as_bytes(),
        )?;

        // add the new key-value pair
        let key = format!("{slot}-{num_blocks_at_slot}");
        Ok(self.database.put_cf(
            self.slots_cf(),
            key.as_bytes(),
            state_hash.to_string().as_bytes(),
        )?)
    }

    fn get_blocks_at_slot(&self, slot: u32) -> anyhow::Result<Vec<PrecomputedBlock>> {
        trace!("Getting blocks at slot {slot}");

        let num_blocks_at_slot = self.get_num_blocks_at_slot(slot)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_slot {
            let key = format!("{slot}-{n}");
            match self
                .database
                .get_pinned_cf(self.slots_cf(), key.as_bytes())?
                .map(|bytes| bytes.to_vec())
            {
                None => break,
                Some(bytes) => {
                    let state_hash: BlockHash = String::from_utf8(bytes)?.into();
                    if let Some(block) = self.get_block(&state_hash)? {
                        blocks.push(block);
                    }
                }
            }
        }

        blocks.sort();
        Ok(blocks)
    }

    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at public key {pk}");
        Ok(
            match self
                .database
                .get_pinned_cf(self.blocks_cf(), pk.to_string().as_bytes())?
            {
                None => 0,
                Some(bytes) => String::from_utf8(bytes.to_vec())?.parse()?,
            },
        )
    }

    fn add_block_at_public_key(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at public key {pk}");

        // increment num blocks at public key
        let num_blocks_at_pk = self.get_num_blocks_at_public_key(pk)?;
        self.database.put_cf(
            self.blocks_cf(),
            pk.to_string().as_bytes(),
            (num_blocks_at_pk + 1).to_string().as_bytes(),
        )?;

        // add the new key-value pair
        let key = format!("{pk}-{num_blocks_at_pk}");
        Ok(self.database.put_cf(
            self.blocks_cf(),
            key.as_bytes(),
            state_hash.to_string().as_bytes(),
        )?)
    }

    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<PrecomputedBlock>> {
        trace!("Getting blocks at public key {pk}");

        let num_blocks_at_pk = self.get_num_blocks_at_public_key(pk)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_pk {
            let key = format!("{pk}-{n}");
            match self
                .database
                .get_pinned_cf(self.blocks_cf(), key.as_bytes())?
                .map(|bytes| bytes.to_vec())
            {
                None => break,
                Some(bytes) => {
                    let state_hash: BlockHash = String::from_utf8(bytes)?.into();
                    if let Some(block) = self.get_block(&state_hash)? {
                        blocks.push(block);
                    }
                }
            }
        }

        blocks.sort();
        Ok(blocks)
    }

    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<PrecomputedBlock>> {
        trace!("Getting children of block {}", state_hash);

        if let Some(height) = self.get_block(state_hash)?.map(|b| b.blockchain_length()) {
            let blocks_at_next_height = self.get_blocks_at_height(height + 1)?;
            let mut children: Vec<PrecomputedBlock> = blocks_at_next_height
                .into_iter()
                .filter(|b| b.previous_state_hash() == *state_hash)
                .collect();
            children.sort();
            return Ok(children);
        }
        bail!("Block missing from store {}", state_hash)
    }

    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>> {
        trace!("Getting block {} version", state_hash.0);

        let key = state_hash.0.as_bytes();
        Ok(self
            .database
            .get_cf(self.blocks_version_cf(), key)?
            .map(|bytes| serde_json::from_slice(&bytes).unwrap()))
    }

    fn set_block_version(&self, state_hash: &BlockHash, version: PcbVersion) -> anyhow::Result<()> {
        trace!("Setting block {} version to {}", state_hash.0, version);

        let key = state_hash.0.as_bytes();
        let value = serde_json::to_vec(&version)?;
        Ok(self.database.put_cf(self.blocks_version_cf(), key, value)?)
    }

    fn set_height_global_slot(&self, blockchain_length: u32, slot: u32) -> anyhow::Result<()> {
        trace!("Setting height {} <-> slot {}", blockchain_length, slot);

        // add: slot -> height
        self.database.put_cf(
            self.block_global_slot_to_height_cf(),
            to_be_bytes(blockchain_length),
            to_be_bytes(slot),
        )?;

        // add: height -> slot
        self.database.put_cf(
            self.block_height_to_global_slot_cf(),
            to_be_bytes(slot),
            to_be_bytes(blockchain_length),
        )?;

        Ok(())
    }

    fn get_globl_slot_from_height(&self, blockchain_length: u32) -> anyhow::Result<Option<u32>> {
        trace!("Getting global slot for height {}", blockchain_length);
        Ok(self
            .database
            .get_cf(
                self.block_global_slot_to_height_cf(),
                to_be_bytes(blockchain_length),
            )?
            .map(from_be_bytes))
    }

    fn get_height_from_global_slot(
        &self,
        global_slot_since_genesis: u32,
    ) -> anyhow::Result<Option<u32>> {
        trace!(
            "Getting height for global slot {}",
            global_slot_since_genesis
        );
        Ok(self
            .database
            .get_cf(
                self.block_height_to_global_slot_cf(),
                to_be_bytes(global_slot_since_genesis),
            )?
            .map(from_be_bytes))
    }

    fn get_current_epoch(&self) -> anyhow::Result<u32> {
        trace!("Getting current epoch");
        Ok(self.get_best_block()?.map_or(0, |b| b.epoch_count()))
    }

    fn increment_block_production_count(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Incrementing block production count {}", block.summary());

        let creator = block.block_creator();
        let epoch = block.epoch_count();

        // increment pk epoch count
        let acc = self.get_block_production_pk_epoch_count(&creator, Some(epoch))?;
        self.database.put_cf(
            self.block_production_pk_epoch_cf(),
            u32_prefix_key(epoch, &creator.0),
            to_be_bytes(acc + 1),
        )?;

        // increment pk total count
        let acc = self.get_block_production_pk_total_count(&creator)?;
        self.database.put_cf(
            self.block_production_pk_total_cf(),
            creator.to_bytes(),
            to_be_bytes(acc + 1),
        )?;

        // increment epoch count
        let acc = self.get_block_production_epoch_count(epoch)?;
        self.database.put_cf(
            self.block_production_epoch_cf(),
            to_be_bytes(epoch),
            to_be_bytes(acc + 1),
        )?;

        // increment total count
        let acc = self.get_block_production_total_count()?;
        self.database
            .put(Self::TOTAL_NUM_BLOCKS_KEY, to_be_bytes(acc + 1))?;

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
                u32_prefix_key(epoch, &pk.0),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total block production count {pk}");
        Ok(self
            .database
            .get_cf(self.block_production_pk_total_cf(), pk.clone().to_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_epoch_count(&self, epoch: u32) -> anyhow::Result<u32> {
        trace!("Getting epoch block production count {epoch}");
        Ok(self
            .database
            .get_cf(self.block_production_epoch_cf(), to_be_bytes(epoch))?
            .map_or(0, from_be_bytes))
    }

    fn get_block_production_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total block production count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_BLOCKS_KEY)?
            .map_or(0, from_be_bytes))
    }
}
