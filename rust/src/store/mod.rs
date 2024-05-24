//! This module contains the implementations of all store traits for the
//! [IndexerStore]

mod column_families;
mod fixed_keys;

use self::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
        BlockHash,
    },
    canonicity::{store::CanonicityStore, Canonicity},
    chain::{store::ChainStore, ChainId, Network},
    command::{
        internal::{InternalCommand, InternalCommandWithData},
        signed::{SignedCommand, SignedCommandWithData},
        store::CommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        diff::{account::PaymentDiff, LedgerBalanceUpdate},
        public_key::PublicKey,
        staking::{AggregatedEpochStakeDelegations, StakingLedger},
        store::LedgerStore,
        Ledger, LedgerHash,
    },
    snark_work::{
        store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal,
    },
};
use anyhow::{anyhow, bail};
use log::{error, trace, warn};
use speedb::{
    ColumnFamily, ColumnFamilyDescriptor, DBCompressionType, DBIterator, IteratorMode, DB,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
    pub is_primary: bool,
}

impl IndexerStore {
    /// Add the corresponding CF helper to [ColumnFamilyHelpers]
    const COLUMN_FAMILIES: [&'static str; 27] = [
        "account-balance",
        "account-balance-sort",
        "account-balance-updates",
        "block-production-pk-epoch", // [block_production_pk_epoch_cf]
        "block-production-pk-total", // [block_production_pk_total_cf]
        "block-production-epoch",    // [block_production_epoch_cf]
        "blocks-state-hash",
        "blocks-version",
        "blocks-global-slot-idx",
        "blocks-at-length",
        "blocks-at-slot",
        "block-height-to-slot", // [block_height_to_global_slot_cf]
        "block-slot-to-height", // [block_global_slot_to_height_cf]
        "canonicity",
        "commands",
        "mainnet-commands-slot",
        "mainnet-cmds-txn-global-slot",
        "mainnet-internal-commands",
        "events",
        "ledgers",
        "snarks",
        "snark-work-top-producers",
        "snark-work-top-producers-sort",
        "snark-work-fees",     // [snark_work_fees_cf]
        "chain-id-to-network", // [chain_id_to_network_cf]
        "txn-from",            // [txn_from_cf]
        "txn-to",              // [txn_to_cf]
    ];

    /// Creates a new _primary_ indexer store
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        // check that all column families are included
        assert_eq!(Self::COLUMN_FAMILIES.len(), Self::NUM_COLUMN_FAMILIES);

        let mut cf_opts = speedb::Options::default();
        cf_opts.set_max_write_buffer_number(16);
        cf_opts.set_compression_type(DBCompressionType::Zstd);

        let mut database_opts = speedb::Options::default();
        database_opts.set_compression_type(DBCompressionType::Zstd);
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);

        let column_families: Vec<ColumnFamilyDescriptor> = Self::COLUMN_FAMILIES
            .iter()
            .map(|cf| ColumnFamilyDescriptor::new(*cf, cf_opts.clone()))
            .collect();
        Ok(Self {
            is_primary: true,
            db_path: path.into(),
            database: speedb::DBWithThreadMode::open_cf_descriptors(
                &database_opts,
                path,
                column_families,
            )?,
        })
    }

    pub fn create_checkpoint(&self, path: &Path) -> anyhow::Result<()> {
        use speedb::checkpoint::Checkpoint;

        let checkpoint = Checkpoint::new(&self.database)?;
        Checkpoint::create_checkpoint(&checkpoint, path)
            .map_err(|e| anyhow!("Error creating db checkpoint: {}", e))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// [BlockStore] implementation

fn global_slot_block_key(block: &PrecomputedBlock) -> Vec<u8> {
    let mut res = to_be_bytes(block.global_slot_since_genesis());
    res.append(&mut block.state_hash().to_bytes());
    res
}

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
        self.add_commands(block)?;

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
            if old != *state_hash {
                let balance_updates =
                    self.common_ancestor_account_balance_updates(&old, state_hash)?;
                self.update_account_balances(balance_updates)?;
            }
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

    // TODO make modular over different account updates
    fn common_ancestor_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<LedgerBalanceUpdate> {
        trace!(
            "Getting common ancestor account balance updates:\n  old: {}\n  new: {}",
            old_best_tip.0,
            new_best_tip.0
        );

        // follows the old best tip back to the common ancestor
        let mut a = self.get_block(old_best_tip)?.expect("old best block");
        let mut unapply = LedgerBalanceUpdate::from_precomputed(&a);

        // follows the new best tip back to the common ancestor
        let mut b = self.get_block(new_best_tip)?.expect("new best block");
        let mut apply = LedgerBalanceUpdate::from_precomputed(&b);

        // b is better than a
        assert!(b < a, "\nb: {}\na: {}", b.summary(), a.summary());

        // bring b back to the same height as a
        let genesis_state_hashes: Vec<BlockHash> = self.get_known_genesis_state_hashes()?;
        for _ in 0..(b.blockchain_length() - a.blockchain_length()) {
            let prev = b.previous_state_hash();

            // check if there's a previous block
            if genesis_state_hashes.contains(&b.state_hash()) {
                break;
            }

            b = self.get_block(&prev)?.expect("previous block");
            apply.append(&mut LedgerBalanceUpdate::from_precomputed(&b));
        }
        assert_eq!(a.blockchain_length(), b.blockchain_length());

        while a.previous_state_hash() != b.previous_state_hash()
            && genesis_state_hashes.contains(&a.previous_state_hash())
        {
            let a_prev = self
                .get_block(&a.previous_state_hash())?
                .expect("a prev block");
            let b_prev = self
                .get_block(&b.previous_state_hash())?
                .expect("b prev block");

            unapply.append(&mut LedgerBalanceUpdate::from_precomputed(&a_prev));
            apply.append(&mut LedgerBalanceUpdate::from_precomputed(&b_prev));

            a = a_prev;
            b = b_prev;
        }

        apply.reverse();
        Ok(LedgerBalanceUpdate { apply, unapply })
    }

    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<PaymentDiff>> {
        trace!("Getting block balance updates for {}", state_hash.0);
        Ok(self
            .database
            .get_cf(self.account_balance_updates_cf(), state_hash.0.as_bytes())?
            .map_or(vec![], |bytes| {
                serde_json::from_slice::<Vec<PaymentDiff>>(&bytes).expect("balance updates")
            }))
    }

    fn update_account_balances(&self, update: LedgerBalanceUpdate) -> anyhow::Result<()> {
        trace!("Updating account balances");

        let mut update = update;
        let balance_updates = update.balance_updates();
        for (pk, amount) in balance_updates {
            let key = pk.0.as_bytes();
            let balance = self
                .database
                .get_cf(self.account_balance_cf(), key)?
                .map_or(0, |bytes| {
                    String::from_utf8(bytes).unwrap().parse::<u64>().unwrap()
                });

            // delete stale data
            self.database.delete_cf(self.account_balance_cf(), key)?;

            // write new data
            self.database.put_cf(
                self.account_balance_cf(),
                key,
                ((balance as i64 + amount) as u64).to_string().as_bytes(),
            )?;
        }
        Ok(())
    }

    fn set_account_balance(&self, pk: &PublicKey, balance: u64) -> anyhow::Result<()> {
        trace!("Setting account balance: {} -> {balance}", pk.0);
        self.database.put_cf(
            self.account_balance_cf(),
            pk.0.as_bytes(),
            balance.to_string().as_bytes(),
        )?;
        Ok(())
    }

    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()> {
        trace!("Setting block balance updates for {}", state_hash.0);
        self.database.put_cf(
            self.account_balance_updates_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&balance_updates)?,
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

/// [CanonicityStore] implementation

impl CanonicityStore for IndexerStore {
    fn add_canonical_block(&self, height: u32, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!(
            "Adding canonical block (length {}): {}",
            state_hash.0,
            height
        );

        // height -> state hash
        self.database.put_cf(
            self.canonicity_cf(),
            height.to_be_bytes(),
            serde_json::to_vec(state_hash)?,
        )?;

        // update canonical chain length
        self.set_max_canonical_blockchain_length(height)?;

        // update top snarkers based on the incoming canonical block
        if let Some(completed_works) = self.get_snark_work_in_block(state_hash)? {
            self.update_top_snarkers(completed_works)?;
        }

        // record new genesis state hash
        if height == 1 {
            if let Some(mut genesis_state_hashes) = self
                .database
                .get_cf(self.canonicity_cf(), Self::KNOWN_GENESIS_STATE_HASHES_KEY)?
                .map(|bytes| {
                    serde_json::from_slice::<Vec<BlockHash>>(&bytes).expect("genesis state hashes")
                })
                .clone()
            {
                // check if hash is present, then add it
                if !genesis_state_hashes.contains(state_hash) {
                    genesis_state_hashes.push(state_hash.clone());
                    self.database.put_cf(
                        self.canonicity_cf(),
                        Self::KNOWN_GENESIS_STATE_HASHES_KEY,
                        serde_json::to_vec(&genesis_state_hashes)?,
                    )?;
                }
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
            .get_cf(self.canonicity_cf(), Self::KNOWN_GENESIS_STATE_HASHES_KEY)?
            .map_or(vec![], |bytes| {
                serde_json::from_slice(&bytes).expect("genesis state hashes")
            }))
    }

    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting canonical hash at height {height}");

        let key = height.to_be_bytes();
        let canonicity_cf = self.canonicity_cf();
        match self
            .database
            .get_pinned_cf(&canonicity_cf, key)?
            .map(|bytes| bytes.to_vec())
        {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        }
    }

    fn get_max_canonical_blockchain_length(&self) -> anyhow::Result<Option<u32>> {
        trace!("Getting max canonical blockchain length");

        let canonicity_cf = self.canonicity_cf();
        match self
            .database
            .get_pinned_cf(&canonicity_cf, Self::MAX_CANONICAL_KEY)?
            .map(|bytes| bytes.to_vec())
        {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        }
    }

    fn set_max_canonical_blockchain_length(&self, height: u32) -> anyhow::Result<()> {
        trace!("Setting max canonical blockchain length to {height}");

        let canonicity_cf = self.canonicity_cf();
        let value = serde_json::to_vec(&height)?;
        self.database
            .put_cf(&canonicity_cf, Self::MAX_CANONICAL_KEY, value)?;
        Ok(())
    }

    fn get_block_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        trace!("Getting canonicity of block with hash {}", state_hash.0);

        if let Ok(Some(best_tip)) = self.get_best_block() {
            if let Some(blockchain_length) = self.get_block(state_hash)?.map(|pcb| match pcb {
                PrecomputedBlock::V1(v1) => v1.blockchain_length,
                PrecomputedBlock::V2(pcb_v2) => {
                    pcb_v2.protocol_state.body.consensus_state.blockchain_length
                }
            }) {
                if blockchain_length > best_tip.blockchain_length() {
                    return Ok(None);
                } else if let Some(max_canonical_length) =
                    self.get_max_canonical_blockchain_length()?
                {
                    if blockchain_length > max_canonical_length {
                        // follow best chain back from tip to given block
                        let mut curr_block = best_tip;
                        while curr_block.state_hash() != *state_hash
                            && curr_block.blockchain_length() > max_canonical_length
                        {
                            if let Some(parent) =
                                self.get_block(&curr_block.previous_state_hash())?
                            {
                                curr_block = parent;
                            } else {
                                break;
                            }
                        }

                        if curr_block.state_hash() == *state_hash
                            && curr_block.blockchain_length() > max_canonical_length
                        {
                            return Ok(Some(Canonicity::Canonical));
                        } else {
                            return Ok(Some(Canonicity::Orphaned));
                        }
                    } else if self.get_canonical_hash_at_height(blockchain_length)?
                        == Some(state_hash.clone())
                    {
                        return Ok(Some(Canonicity::Canonical));
                    } else {
                        return Ok(Some(Canonicity::Orphaned));
                    }
                }
            }
        }
        Ok(None)
    }
}

/// [LedgerStore] implementation

/// [DBIterator] for balance-sorted accounts
/// - key: `{balance BE bytes}{pk bytes}`
/// - value: empty byte
pub fn account_balance_iterator<'a>(
    db: &'a Arc<IndexerStore>,
    mode: IteratorMode,
) -> DBIterator<'a> {
    db.database.iterator_cf(db.account_balance_sort_cf(), mode)
}

impl LedgerStore for IndexerStore {
    fn add_ledger(&self, ledger_hash: &LedgerHash, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!(
            "Adding staged ledger\nstate_hash: {}\nledger_hash: {}",
            state_hash.0,
            ledger_hash.0
        );

        // add state hash for ledger to db
        let key = ledger_hash.0.as_bytes();
        let value = state_hash.0.as_bytes();
        self.database.put_cf(self.ledgers_cf(), key, value)?;
        Ok(())
    }

    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()> {
        trace!("Adding staged ledger state hash {}", state_hash.0);

        // add ledger to db
        let key = state_hash.0.as_bytes();
        let value = ledger.to_string();
        let value = value.as_bytes();
        self.database.put_cf(self.ledgers_cf(), key, value)?;

        // index on state hash & add new ledger event
        if state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH {
            self.add_ledger(&LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()), state_hash)?;
            self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                DbLedgerEvent::NewLedger {
                    blockchain_length: 0,
                    state_hash: state_hash.clone(),
                    ledger_hash: LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()),
                },
            )))?;
        } else {
            match self.get_block(state_hash)? {
                Some(block) => {
                    let ledger_hash = block.staged_ledger_hash();
                    self.add_ledger(&ledger_hash, state_hash)?;
                    self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                        DbLedgerEvent::NewLedger {
                            ledger_hash,
                            state_hash: block.state_hash(),
                            blockchain_length: block.blockchain_length(),
                        },
                    )))?;
                }
                None => error!("Block missing from store {}", state_hash.0),
            }
        }
        Ok(())
    }

    fn get_ledger_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger state hash {}", state_hash.0);

        let mut state_hash = state_hash.clone();
        let mut to_apply = vec![];

        // walk chain back to a stored ledger
        // collect blocks to compute the current ledger
        while self
            .database
            .get_pinned_cf(self.ledgers_cf(), state_hash.0.as_bytes())?
            .is_none()
        {
            trace!("No staged ledger found for state hash {}", state_hash);
            if let Some(block) = self.get_block(&state_hash)? {
                to_apply.push(block.clone());
                state_hash = block.previous_state_hash();
                trace!(
                    "Checking for staged ledger state hash {}",
                    block.previous_state_hash().0
                );
            } else {
                error!("Block missing from store: {}", state_hash.0);
                return Ok(None);
            }
        }

        trace!("Found staged ledger state hash {}", state_hash.0);
        to_apply.reverse();

        if let Some(mut ledger) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), state_hash.0.as_bytes())?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
        {
            if let Some(requested_block) = to_apply.last() {
                for block in &to_apply {
                    ledger._apply_diff_from_precomputed(block)?;
                }

                if memoize {
                    trace!("Memoizing ledger for block {}", requested_block.summary());
                    self.add_ledger_state_hash(&requested_block.state_hash(), ledger.clone())?;
                    self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                        DbLedgerEvent::NewLedger {
                            state_hash: requested_block.state_hash(),
                            ledger_hash: requested_block.staged_ledger_hash(),
                            blockchain_length: requested_block.blockchain_length(),
                        },
                    )))?;
                }
            }
            return Ok(Some(ledger));
        }
        Ok(None)
    }

    fn get_ledger(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger hash {}", ledger_hash.0);

        let key = ledger_hash.0.as_bytes();
        if let Some(state_hash) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key)?
            .map(|bytes| BlockHash(String::from_utf8(bytes.to_vec()).unwrap()))
        {
            let key = state_hash.0.as_bytes();
            if let Some(ledger) = self
                .database
                .get_pinned_cf(self.ledgers_cf(), key)?
                .map(|bytes| bytes.to_vec())
                .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
            {
                return Ok(Some(ledger));
            }
        }
        Ok(None)
    }

    fn get_ledger_at_height(&self, height: u32, memoize: bool) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger height {}", height);

        match self.get_canonical_hash_at_height(height)? {
            None => Ok(None),
            Some(state_hash) => self.get_ledger_state_hash(&state_hash, memoize),
        }
    }

    fn get_staking_ledger_at_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger epoch {}", epoch);

        // default to current genesis state hash
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or(self.get_best_block()?.unwrap().genesis_state_hash());
        let key = format!("staking-{}-{}", genesis_state_hash.0, epoch);
        if let Some(ledger_result) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key.as_bytes())?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| {
                let ledger_hash = String::from_utf8(bytes)?;
                self.get_staking_ledger_hash(&ledger_hash.into())
            })
        {
            return ledger_result;
        }
        Ok(None)
    }

    fn get_staking_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger hash {}", ledger_hash.0);

        if let Some(bytes) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), ledger_hash.0.as_bytes())?
        {
            return Ok(Some(serde_json::from_slice::<StakingLedger>(&bytes)?));
        }
        Ok(None)
    }

    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        let epoch = staking_ledger.epoch;
        trace!("Adding staking ledger {}", staking_ledger.summary());

        // add ledger at ledger hash
        let key = staking_ledger.ledger_hash.0.as_bytes();
        let value = serde_json::to_vec(&staking_ledger)?;
        let is_new = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key)?
            .is_none();
        self.database.put_cf(self.ledgers_cf(), key, value)?;

        // add (genesis state hash, epoch) index
        let key = format!("staking-{}-{}", genesis_state_hash.0, epoch);
        let value = staking_ledger.ledger_hash.0.as_bytes();
        self.database
            .put_cf(self.ledgers_cf(), key.as_bytes(), value)?;

        // aggregate staking delegations
        trace!("Aggregating staking delegations epoch {}", epoch);
        let aggregated_delegations = staking_ledger.aggregate_delegations()?;
        let key = format!("delegations-{}-{}", genesis_state_hash.0, epoch);
        self.database.put_cf(
            self.ledgers_cf(),
            key.as_bytes(),
            serde_json::to_vec(&aggregated_delegations)?,
        )?;

        if is_new {
            // add new ledger event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::NewStakingLedger {
                    epoch,
                    ledger_hash: staking_ledger.ledger_hash.clone(),
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;

            // add new aggregated delegation event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::AggregateDelegations {
                    epoch: staking_ledger.epoch,
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;
        }

        Ok(())
    }

    fn get_delegations_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>> {
        trace!("Getting staking delegations for epoch {}", epoch);

        // default to current genesis state hash
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or(self.get_best_block()?.unwrap().genesis_state_hash());
        let key = format!("delegations-{}-{}", genesis_state_hash.0, epoch);

        if let Some(bytes) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key.as_bytes())?
        {
            return Ok(Some(serde_json::from_slice(&bytes)?));
        }
        Ok(None)
    }

    fn store_ledger_balances(&self, ledger: &Ledger) -> anyhow::Result<()> {
        trace!("Storing ledger account balances");

        for (pk, acct) in &ledger.accounts {
            self.set_account_balance(pk, acct.balance.0)?;
        }
        Ok(())
    }
}

/// [EventStore] implementation

impl EventStore for IndexerStore {
    fn add_event(&self, event: &IndexerEvent) -> anyhow::Result<u32> {
        let seq_num = self.get_next_seq_num()?;
        trace!("Adding event {seq_num}: {:?}", event);

        if matches!(event, IndexerEvent::WitnessTree(_)) {
            return Ok(seq_num);
        }

        // add event to db
        let key = seq_num.to_be_bytes();
        let value = serde_json::to_vec(&event)?;
        let events_cf = self.events_cf();
        self.database.put_cf(&events_cf, key, value)?;

        // increment event sequence number
        let next_seq_num = seq_num + 1;
        let value = serde_json::to_vec(&next_seq_num)?;
        self.database
            .put_cf(&events_cf, Self::NEXT_EVENT_SEQ_NUM_KEY, value)?;

        // return next event sequence number
        Ok(next_seq_num)
    }

    fn get_event(&self, seq_num: u32) -> anyhow::Result<Option<IndexerEvent>> {
        let key = seq_num.to_be_bytes();
        let events_cf = self.events_cf();
        let event = self.database.get_pinned_cf(&events_cf, key)?;
        let event = event.map(|bytes| serde_json::from_slice(&bytes).unwrap());

        trace!("Getting event {seq_num}: {:?}", event.clone().unwrap());
        Ok(event)
    }

    fn get_next_seq_num(&self) -> anyhow::Result<u32> {
        trace!("Getting next event sequence number");

        if let Some(bytes) = self
            .database
            .get_pinned_cf(&self.events_cf(), Self::NEXT_EVENT_SEQ_NUM_KEY)?
        {
            serde_json::from_slice(&bytes).map_err(anyhow::Error::from)
        } else {
            Ok(0)
        }
    }

    fn get_event_log(&self) -> anyhow::Result<Vec<IndexerEvent>> {
        trace!("Getting event log");

        let mut events = vec![];

        for n in 0..self.get_next_seq_num()? {
            if let Some(event) = self.get_event(n)? {
                events.push(event);
            }
        }
        Ok(events)
    }
}

/// [CommandStore] implementation

const COMMAND_KEY_PREFIX: &str = "user-";

/// Creates a new user command (transaction) database key from a &String
fn user_command_db_key_str(str: &String) -> String {
    format!("{COMMAND_KEY_PREFIX}{str}")
}

/// Creates a new user command (transaction) database key from one &String
fn user_command_db_key(str: &String) -> Vec<u8> {
    user_command_db_key_str(str).into_bytes()
}

/// Creates a new user command (transaction) database key for a public key
fn user_command_db_key_pk(pk: &String, n: u32) -> Vec<u8> {
    format!("{}-{n}", user_command_db_key_str(pk)).into_bytes()
}

/// Returns a user command (transaction) block state hash from a database key
pub fn convert_user_command_db_key_to_block_hash(db_key: &[u8]) -> anyhow::Result<BlockHash> {
    let db_key_str = std::str::from_utf8(db_key)?;
    let stripped_key = db_key_str.strip_prefix(COMMAND_KEY_PREFIX);

    if let Some(stripped_key) = stripped_key {
        let split_key: Vec<&str> = stripped_key.splitn(2, '-').collect();

        if let Some(first_part) = split_key.first() {
            return Ok(BlockHash(first_part.to_string()));
        }
    }
    bail!("User command key does not start with '{COMMAND_KEY_PREFIX}': {db_key_str}")
}

/// [DBIterator] for blocks
/// - key: `{global slot BE bytes}{state hash bytes}`
/// - value: empty byte
///
/// Use [blocks_global_slot_idx_state_hash_from_key] to extract state hash
pub fn blocks_global_slot_idx_iterator<'a>(
    db: &'a Arc<IndexerStore>,
    mode: IteratorMode,
) -> DBIterator<'a> {
    db.database
        .iterator_cf(db.blocks_global_slot_idx_cf(), mode)
}

/// Extracts state hash from the iterator entry (key)
pub fn blocks_global_slot_idx_state_hash_from_key(key: &[u8]) -> anyhow::Result<String> {
    Ok(String::from_utf8(key[4..].to_vec())?)
}

/// [DBIterator] for user commands (transactions)
pub fn user_commands_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.commands_slot_mainnet_cf(), mode)
}

/// [DBIterator] for user commands by sender
pub fn txn_from_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.txn_from_cf(), mode)
}

/// [DBIterator] for user commands by receiver
pub fn txn_to_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.txn_to_cf(), mode)
}

/// Global slot number from `key` in [user_commands_iterator]
/// - keep the first 4 bytes
pub fn user_commands_iterator_global_slot(key: &[u8]) -> u32 {
    from_be_bytes(key[0..4].to_vec())
}

/// Transaction hash from `key` in [user_commands_iterator]
/// - discard the first 4 bytes
pub fn user_commands_iterator_txn_hash(key: &[u8]) -> anyhow::Result<String> {
    String::from_utf8(key[4..].to_vec()).map_err(|e| anyhow!("Error reading txn hash: {}", e))
}

/// [SignedCommandWithData] from `entry` in [user_commands_iterator]
pub fn user_commands_iterator_signed_command(
    value: &[u8],
) -> anyhow::Result<SignedCommandWithData> {
    Ok(serde_json::from_slice::<SignedCommandWithData>(value)?)
}

pub fn to_be_bytes(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

pub fn from_be_bytes(bytes: Vec<u8>) -> u32 {
    const SIZE: usize = 4;
    let mut be_bytes = [0; SIZE];

    be_bytes[..SIZE].copy_from_slice(&bytes[..SIZE]);
    u32::from_be_bytes(be_bytes)
}

/// The first 4 bytes are `prefix` in big endian
/// - `prefix`: global slot, epoch number, etc
/// - `suffix`: txn hash, public key, etc
fn u32_prefix_key(prefix: u32, suffix: &str) -> Vec<u8> {
    let mut bytes = to_be_bytes(prefix);
    bytes.append(&mut suffix.as_bytes().to_vec());
    bytes
}

/// Key format for sorting txns by sender/receiver: `{pk}{slot}{hash}`
/// - pk:   55 bytes (public key)
/// - slot: 4 BE bytes
/// - hash: rem bytes (txn hash)
pub fn txn_sort_key(public_key: PublicKey, global_slot: u32, txn_hash: &str) -> Vec<u8> {
    let mut bytes = public_key.to_bytes();
    bytes.append(&mut to_be_bytes(global_slot));
    bytes.append(&mut txn_hash.as_bytes().to_vec());
    bytes
}

pub fn txn_sort_key_prefix(public_key: PublicKey, global_slot: u32) -> Vec<u8> {
    let mut bytes = public_key.to_bytes();
    bytes.append(&mut to_be_bytes(global_slot));
    bytes
}

pub fn txn_sort_key_pk(key: &[u8]) -> PublicKey {
    PublicKey::from_bytes(&key[..PublicKey::LEN]).expect("public key")
}

pub fn txn_sort_key_global_slot(key: &[u8]) -> u32 {
    from_be_bytes(key[PublicKey::LEN..(PublicKey::LEN + 4)].to_vec())
}

pub fn txn_sort_key_txn_hash(key: &[u8]) -> String {
    String::from_utf8(key[(PublicKey::LEN + 4)..].to_vec()).expect("txn hash")
}

impl CommandStore for IndexerStore {
    fn add_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let user_commands = block.commands();
        for command in &user_commands {
            let signed = SignedCommand::from(command.clone());
            let txn_hash = signed.hash_signed_command()?;
            trace!("Adding user command hash {} {}", txn_hash, block.summary());

            // add: key `{global_slot}{txn_hash}` -> signed command with data
            // global_slot is written in big endian so lexicographic ordering corresponds to
            // slot ordering
            self.database.put_cf(
                self.commands_slot_mainnet_cf(),
                u32_prefix_key(block.global_slot_since_genesis(), &txn_hash),
                serde_json::to_vec(&SignedCommandWithData::from(
                    command,
                    &block.state_hash().0,
                    block.blockchain_length(),
                    block.timestamp(),
                    block.global_slot_since_genesis(),
                ))?,
            )?;

            // add: key (txn hash) -> value (global slot) so we can
            // reconstruct the key
            self.database.put_cf(
                self.commands_txn_hash_to_global_slot_mainnet_cf(),
                txn_hash.as_bytes(),
                block.global_slot_since_genesis().to_be_bytes(),
            )?;

            // add sender index
            // `{sender}{global_slot BE}{txn_hash} -> amount BE`
            self.database.put_cf(
                self.txn_from_cf(),
                txn_sort_key(
                    command.sender(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                ),
                command.amount().to_be_bytes(),
            )?;

            // add receiver index
            // `{receiver}{global_slot BE}{txn_hash} -> amount BE`
            self.database.put_cf(
                self.txn_to_cf(),
                txn_sort_key(
                    command.receiver(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                ),
                command.amount().to_be_bytes(),
            )?;
        }

        // add: key (state hash) -> user commands with status
        let key = user_command_db_key(&block.state_hash().0);
        let value = serde_json::to_vec(&block.commands())?;
        self.database.put_cf(self.commands_cf(), key, value)?;

        // add: "pk -> linked list of signed commands with state hash"
        for pk in block.all_command_public_keys() {
            trace!("Adding user command for public key {}", pk.0);

            // get pk num commands
            let n = self.get_pk_num_commands(&pk.0)?.unwrap_or(0);
            let block_pk_commands: Vec<SignedCommandWithData> = user_commands
                .iter()
                .filter(|cmd| cmd.contains_public_key(&pk))
                .map(|c| {
                    SignedCommandWithData::from(
                        c,
                        &block.state_hash().0,
                        block.blockchain_length(),
                        block.timestamp(),
                        block.global_slot_since_genesis(),
                    )
                })
                .collect();

            if !block_pk_commands.is_empty() {
                // write these commands to the next key for pk
                let key = user_command_db_key_pk(&pk.0, n);
                let value = serde_json::to_vec(&block_pk_commands)?;
                self.database.put_cf(self.commands_cf(), key, value)?;

                // update pk's num commands
                let key = user_command_db_key(&pk.0);
                let next_n = (n + 1).to_string();
                self.database
                    .put_cf(self.commands_cf(), key, next_n.as_bytes())?;
            }
        }
        Ok(())
    }

    fn get_command_by_hash(
        &self,
        command_hash: &str,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command by hash {}", command_hash);
        if let Some(global_slot_bytes) = self.database.get_pinned_cf(
            self.commands_txn_hash_to_global_slot_mainnet_cf(),
            command_hash.as_bytes(),
        )? {
            let mut key = global_slot_bytes.to_vec();
            key.append(&mut command_hash.as_bytes().to_vec());
            if let Some(commands_bytes) = self
                .database
                .get_pinned_cf(self.commands_slot_mainnet_cf(), key)?
            {
                return Ok(Some(serde_json::from_slice(&commands_bytes)?));
            }
        }
        Ok(None)
    }

    fn get_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<UserCommandWithStatus>> {
        let state_hash = &state_hash.0;
        trace!("Getting user commands in block {}", state_hash);

        let key = user_command_db_key(state_hash);
        if let Some(commands_bytes) = self.database.get_pinned_cf(self.commands_cf(), key)? {
            return Ok(serde_json::from_slice(&commands_bytes)?);
        }
        Ok(vec![])
    }

    fn get_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<SignedCommandWithData>> {
        trace!("Getting user commands for public key {}", pk.0);

        let commands_cf = self.commands_cf();
        let mut commands = vec![];
        fn key_n(pk: &str, n: u32) -> Vec<u8> {
            user_command_db_key_pk(&pk.to_string(), n).to_vec()
        }

        if let Some(n) = self.get_pk_num_commands(&pk.0)? {
            for m in 0..n {
                if let Some(mut block_m_commands) = self
                    .database
                    .get_pinned_cf(commands_cf, key_n(&pk.0, m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<SignedCommandWithData>>(&bytes)
                            .expect("signed commands with state hash")
                    })
                {
                    commands.append(&mut block_m_commands);
                } else {
                    commands.clear();
                    break;
                }
            }
        }
        Ok(commands)
    }

    fn get_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>> {
        let start_block_opt = self.get_block(start_state_hash)?;
        let end_block_opt = self.get_block(end_state_hash)?;
        trace!(
            "Getting user commands between {:?} and {:?}",
            start_block_opt.as_ref().map(|b| b.summary()),
            end_block_opt.as_ref().map(|b| b.summary())
        );

        if let (Some(start_block), Some(end_block)) = (start_block_opt, end_block_opt) {
            let start_height = start_block.blockchain_length();
            let end_height = end_block.blockchain_length();

            if end_height < start_height {
                warn!("Block (length {end_height}) {end_state_hash} is lower than block (length {start_height}) {start_state_hash}");
                return Ok(vec![]);
            }

            let mut num = end_height - start_height;
            let mut prev_hash = end_block.previous_state_hash();
            let mut state_hashes: Vec<BlockHash> = vec![end_block.state_hash()];
            while let Some(block) = self.get_block(&prev_hash)? {
                if num == 0 {
                    break;
                }

                num -= 1;
                state_hashes.push(prev_hash);
                prev_hash = block.previous_state_hash();
            }

            return Ok(self
                .get_commands_for_public_key(pk)?
                .into_iter()
                .filter(|c| state_hashes.contains(&c.state_hash))
                .collect());
        }
        Ok(vec![])
    }

    /// Number of blocks containing `pk` commands
    fn get_pk_num_commands(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        trace!("Getting number of internal commands for {}", pk);

        let key = user_command_db_key(&pk.to_string());
        Ok(self
            .database
            .get_pinned_cf(self.commands_cf(), key)?
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }))
    }

    /// Index internal commands on public keys & state hash
    fn add_internal_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding internal commands for block {}", block.summary());

        // add cmds to state hash
        let key = format!("internal-{}", block.state_hash().0);
        let internal_cmds = InternalCommand::from_precomputed(block);
        self.database.put_cf(
            self.commands_cf(),
            key.as_bytes(),
            serde_json::to_vec(&internal_cmds)?,
        )?;

        // add cmds with data to public keys
        let internal_cmds_with_data: Vec<InternalCommandWithData> = internal_cmds
            .clone()
            .into_iter()
            .map(|c| InternalCommandWithData::from_internal_cmd(c, block))
            .collect();

        fn internal_commmand_key(global_slot: u32, state_hash: &str, index: usize) -> Vec<u8> {
            let mut bytes = to_be_bytes(global_slot);
            bytes.append(&mut state_hash.as_bytes().to_vec());
            bytes.append(&mut index.to_be_bytes().to_vec());
            bytes
        }

        for (i, int_cmd) in internal_cmds_with_data.iter().enumerate() {
            let key =
                internal_commmand_key(block.global_slot_since_genesis(), &block.state_hash().0, i);
            self.database.put_cf(
                self.internal_commands_cf(),
                key,
                serde_json::to_vec(&int_cmd)?,
            )?;
        }

        for pk in block.all_public_keys() {
            trace!("Writing internal commands for {}", pk.0);

            let n = self.get_pk_num_internal_commands(&pk.0)?.unwrap_or(0);
            let key = format!("internal-{}-{}", pk.0, n);
            let pk_internal_cmds_with_data: Vec<InternalCommandWithData> = internal_cmds_with_data
                .iter()
                .filter_map(|cmd| {
                    if cmd.contains_pk(&pk) {
                        Some(cmd.clone())
                    } else {
                        None
                    }
                })
                .collect();
            self.database.put_cf(
                self.commands_cf(),
                key.as_bytes(),
                serde_json::to_vec(&pk_internal_cmds_with_data)?,
            )?;

            // update pk's number of internal cmds
            let key = format!("internal-{}", pk.0);
            let next_n = (n + 1).to_string();
            self.database
                .put_cf(self.commands_cf(), key.as_bytes(), next_n.as_bytes())?;
        }
        Ok(())
    }

    fn get_internal_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<InternalCommandWithData>> {
        trace!("Getting internal commands in block {}", state_hash.0);
        let block = self.get_block(state_hash)?.expect("block to exist");

        let key = format!("internal-{}", state_hash.0);
        if let Some(commands_bytes) = self
            .database
            .get_pinned_cf(self.commands_cf(), key.as_bytes())?
        {
            let res: Vec<InternalCommand> = serde_json::from_slice(&commands_bytes)?;
            return Ok(res
                .into_iter()
                .map(|cmd| InternalCommandWithData::from_internal_cmd(cmd, &block))
                .collect());
        }
        Ok(vec![])
    }

    fn get_internal_commands_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<InternalCommandWithData>> {
        trace!("Getting internal commands for public key {}", pk.0);

        let commands_cf = self.commands_cf();
        let mut internal_cmds = vec![];
        fn key_n(pk: String, n: u32) -> Vec<u8> {
            format!("internal-{}-{}", pk, n).as_bytes().to_vec()
        }

        if let Some(n) = self.get_pk_num_internal_commands(&pk.0)? {
            for m in 0..n {
                if let Some(mut block_m_internal_cmds) = self
                    .database
                    .get_pinned_cf(commands_cf, key_n(pk.0.clone(), m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<InternalCommandWithData>>(&bytes)
                            .expect("internal commands with data")
                    })
                {
                    internal_cmds.append(&mut block_m_internal_cmds);
                } else {
                    internal_cmds.clear();
                    break;
                }
            }
        }
        Ok(internal_cmds)
    }

    /// Number of blocks containing `pk` internal commands
    fn get_pk_num_internal_commands(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        let key = format!("internal-{}", pk);
        Ok(self
            .database
            .get_pinned_cf(self.commands_cf(), key.as_bytes())?
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }))
    }

    fn get_internal_commands_interator(&self, mode: speedb::IteratorMode) -> DBIterator<'_> {
        self.database.iterator_cf(self.internal_commands_cf(), mode)
    }
}

/// [SnarkStore] implementation

/// [DBIterator] for snark work
pub fn top_snarkers_iterator<'a>(db: &'a IndexerStore, mode: IteratorMode) -> DBIterator<'a> {
    db.database
        .iterator_cf(db.snark_top_producers_sort_cf(), mode)
}

/// [DBIterator] for snark work fees
pub fn snark_fees_iterator<'a>(db: &'a IndexerStore, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.snark_work_fees_cf(), mode)
}

fn fee_prefix(fee: u64) -> Vec<u8> {
    fee.to_be_bytes().to_vec()
}

/// The first 8 bytes are fees in big endian.
fn fee_prefix_key(fee: u64, suffix: &str) -> Vec<u8> {
    let mut bytes = fee_prefix(fee);
    bytes.append(&mut suffix.as_bytes().to_vec());
    bytes
}

/// Key format `{fee}{slot}{pk}{state_hash}{num}`
/// - fee:  8 BE bytes
/// - slot: 4 BE bytes
/// - num:  4 BE bytes
fn snark_fee_prefix_key(
    fee: u64,
    global_slot: u32,
    pk: PublicKey,
    state_hash: BlockHash,
    num: u32,
) -> Vec<u8> {
    let mut bytes = fee.to_be_bytes().to_vec();
    bytes.append(&mut global_slot.to_be_bytes().to_vec());
    bytes.append(&mut pk.0.as_bytes().to_vec());
    bytes.append(&mut state_hash.0.as_bytes().to_vec());
    bytes.append(&mut num.to_be_bytes().to_vec());
    bytes
}

impl SnarkStore for IndexerStore {
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding SNARK work from block {}", block.summary());

        let completed_works = SnarkWorkSummary::from_precomputed(block);
        let completed_works_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(block);

        // add: state hash -> snark works
        let state_hash = block.state_hash().0;
        let key = state_hash.as_bytes();
        let value = serde_json::to_vec(&completed_works)?;
        self.database.put_cf(self.snarks_cf(), key, value)?;

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
                    fee_prefix_key(old_total, &snark.prover.0),
                )?
            }
        }

        // replace stale data with updated
        for (prover, (old_total, new_fees)) in prover_fees.iter() {
            let total_fees = old_total + new_fees;
            let key = fee_prefix_key(total_fees, &prover.0);
            self.database
                .put_cf(self.snark_top_producers_sort_cf(), key, b"")?
        }

        Ok(())
    }

    fn get_top_snarkers(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>> {
        trace!("Getting top {n} SNARK workers");

        Ok(top_snarkers_iterator(self, IteratorMode::End)
            .take(n)
            .map(|res| {
                res.map(|(bytes, _)| SnarkWorkTotal {
                    prover: String::from_utf8(bytes[8..].to_vec())
                        .expect("public key bytes")
                        .into(),
                    total_fees: u64::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]),
                })
                .expect("snark work iterator")
            })
            .collect())
    }
}

impl ChainStore for IndexerStore {
    fn set_chain_id_for_network(
        &self,
        chain_id: &ChainId,
        network: &Network,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting chain id '{}' for network '{}'",
            chain_id.0,
            network
        );

        let chain_bytes = chain_id.0.as_bytes();

        // add the new pair
        self.database.put_cf(
            self.chain_id_to_network_cf(),
            chain_bytes,
            network.to_string().as_bytes(),
        )?;

        // update current chain_id
        self.database.put(Self::CHAIN_ID_KEY, chain_bytes)?;
        Ok(())
    }

    fn get_network(&self, chain_id: &ChainId) -> anyhow::Result<Network> {
        trace!("Getting network for chain id: {}", chain_id.0);
        Ok(Network::from(
            self.database
                .get_cf(self.chain_id_to_network_cf(), chain_id.0.as_bytes())?
                .expect("network should exist in database"),
        ))
    }

    fn get_current_network(&self) -> anyhow::Result<Network> {
        trace!("Getting current network");
        self.get_network(&self.get_chain_id()?)
    }

    fn get_chain_id(&self) -> anyhow::Result<ChainId> {
        trace!("Getting chain id");
        Ok(ChainId(String::from_utf8(
            self.database
                .get(Self::CHAIN_ID_KEY)?
                .expect("chain id should exist in database"),
        )?))
    }
}

impl FixedKeys for IndexerStore {}

impl IndexerStore {
    pub fn db_stats(&self) -> String {
        self.database
            .property_value(speedb::properties::DBSTATS)
            .unwrap()
            .unwrap()
    }

    pub fn memtables_size(&self) -> String {
        self.database
            .property_value(speedb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_live_data_size(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::ESTIMATE_LIVE_DATA_SIZE)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_num_keys(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::ESTIMATE_NUM_KEYS)
            .unwrap()
            .unwrap()
    }

    pub fn cur_size_all_mem_tables(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }
}

impl ColumnFamilyHelpers for IndexerStore {
    /// CF for storing account balances (best ledger):
    /// `pk -> balance`
    fn account_balance_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance")
            .expect("account-balance column family exists")
    }

    /// CF for sorting account's by balance
    /// `{balance}{pk} -> _`
    ///
    /// - `balance`: 8 BE bytes
    fn account_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance-sort")
            .expect("account-balance-sort column family exists")
    }

    /// CF for storing account balance updates:
    /// `state hash -> balance updates`
    fn account_balance_updates_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance-updates")
            .expect("account-balance-updates column family exists")
    }

    /// CF for storing all blocks
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// CF for storing block versions:
    /// `state hash -> pcb version`
    fn blocks_version_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-version")
            .expect("blocks-version column family exists")
    }

    /// CF for sorting blocks by global slot
    /// `{global_slot}{state_hash} -> _`
    fn blocks_global_slot_idx_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-global-slot-idx")
            .expect("blocks-global-slot-idx column family exists")
    }

    /// CF for storing: height -> global slot
    fn block_height_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-height-to-slot")
            .expect("block-height-to-slot column family exists")
    }

    /// CF for storing: global slot -> height
    fn block_global_slot_to_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-slot-to-height")
            .expect("block-slot-to-height column family exists")
    }

    /// CF for storing blocks at a fixed height:
    /// `height -> list of blocks at height`
    ///
    /// - `list of blocks at height`: sorted from best to worst
    fn lengths_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-length")
            .expect("blocks-at-length column family exists")
    }

    /// CF for storing blocks at a fixed global slot:
    /// `global slot -> list of blocks at slot`
    ///
    /// - `list of blocks at slot`: sorted from best to worst
    fn slots_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-slot")
            .expect("blocks-at-slot column family exists")
    }

    fn canonicity_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("canonicity")
            .expect("canonicity column family exists")
    }

    fn commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("commands")
            .expect("commands column family exists")
    }

    fn internal_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-internal-commands")
            .expect("mainnet-internal-commands column family exists")
    }

    /// CF for sorting user commands: `{global_slot}{txn_hash} -> data`
    ///
    /// - `global_slot`: 4 BE bytes
    fn commands_slot_mainnet_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-commands-slot")
            .expect("mainnet-commands-slot column family exists")
    }

    /// CF for storing: `txn_hash -> global_slot`
    ///
    /// - `global_slot`: 4 BE bytes
    fn commands_txn_hash_to_global_slot_mainnet_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-cmds-txn-global-slot")
            .expect("mainnet-cmds-txn-global-slot column family exists")
    }

    fn ledgers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("ledgers")
            .expect("ledgers column family exists")
    }

    fn events_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("events")
            .expect("events column family exists")
    }

    fn snarks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks")
            .expect("snarks column family exists")
    }

    /// CF for storing all snark work fee totals
    fn snark_top_producers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers")
            .expect("snark-work-top-producers column family exists")
    }

    /// CF for sorting all snark work fee totals
    fn snark_top_producers_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers-sort")
            .expect("snark-work-top-producers-sort column family exists")
    }

    /// CF for storing/sorting SNARK work fees
    fn snark_work_fees_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-fees")
            .expect("snark-work-fees column family exists")
    }

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("chain-id-to-network")
            .expect("chain-id-to-network column family exists")
    }

    /// CF for sorting user commands by sender public key
    fn txn_from_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from")
            .expect("txn-from column family exists")
    }

    /// CF for sorting user commands by receiver public key in [CommandStore]
    fn txn_to_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-to")
            .expect("txn-to column family exists")
    }

    /// CF for per epoch per account block prodution info
    /// - key: `{epoch BE bytes}{pk}`
    /// - value: number of blocks produced by `pk` in `epoch`
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-epoch")
            .expect("block-production-pk-epoch column family exists")
    }

    /// CF for per account total block prodution info
    /// - key: `pk`
    /// - value: total number of blocks produced by `pk`
    fn block_production_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-total")
            .expect("block-production-pk-total column family exists")
    }

    /// CF for per epoch block production totals
    /// - key: `epoch`
    /// - value: number of blocks produced in `epoch`
    fn block_production_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-epoch")
            .expect("block-production-epoch column family exists")
    }
}