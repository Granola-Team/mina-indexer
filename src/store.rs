use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    command::{
        signed::{SignedCommand, SignedCommandWithStateHash},
        store::CommandStore,
        UserCommandWithStatus,
    },
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{public_key::PublicKey, store::LedgerStore, Ledger},
};
use anyhow::anyhow;
use speedb::{ColumnFamilyDescriptor, DB};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::{info, trace, warn};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
}

impl IndexerStore {
    pub fn new_read_only(path: &Path, secondary: &Path) -> anyhow::Result<Self> {
        let database_opts = speedb::Options::default();
        let database = speedb::DBWithThreadMode::open_cf_as_secondary(
            &database_opts,
            path,
            secondary,
            vec!["blocks", "canonicity", "commands", "events", "ledgers"],
        )?;
        Ok(Self {
            db_path: PathBuf::from(secondary),
            database,
        })
    }

    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut cf_opts = speedb::Options::default();
        cf_opts.set_max_write_buffer_number(16);
        let blocks = ColumnFamilyDescriptor::new("blocks", cf_opts.clone());
        let canonicity = ColumnFamilyDescriptor::new("canonicity", cf_opts.clone());
        let commands = ColumnFamilyDescriptor::new("commands", cf_opts.clone());
        let events = ColumnFamilyDescriptor::new("events", cf_opts.clone());
        let ledgers = ColumnFamilyDescriptor::new("ledgers", cf_opts);

        let mut database_opts = speedb::Options::default();
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);
        let database = speedb::DBWithThreadMode::open_cf_descriptors(
            &database_opts,
            path,
            vec![blocks, canonicity, commands, events, ledgers],
        )?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }

    pub fn create_checkpoint(&self, path: &Path) -> anyhow::Result<()> {
        use speedb::checkpoint::Checkpoint;

        let checkpoint = Checkpoint::new(&self.database)?;
        Checkpoint::create_checkpoint(&checkpoint, path)
            .map_err(|e| anyhow!("Error creating db checkpoint: {}", e.to_string()))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn blocks_cf(&self) -> &speedb::ColumnFamily {
        self.database
            .cf_handle("blocks")
            .expect("blocks column family exists")
    }

    fn canonicity_cf(&self) -> &speedb::ColumnFamily {
        self.database
            .cf_handle("canonicity")
            .expect("canonicity column family exists")
    }

    fn commands_cf(&self) -> &speedb::ColumnFamily {
        self.database
            .cf_handle("commands")
            .expect("commands column family exists")
    }

    fn ledgers_cf(&self) -> &speedb::ColumnFamily {
        self.database
            .cf_handle("ledgers")
            .expect("ledgers column family exists")
    }

    fn events_cf(&self) -> &speedb::ColumnFamily {
        self.database
            .cf_handle("events")
            .expect("events column family exists")
    }
}

impl BlockStore for IndexerStore {
    /// Add the given block at its state hash and record a DbNewBlockevent
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<DbEvent> {
        trace!(
            "Adding block with height {} and hash {}",
            block.blockchain_length,
            block.state_hash
        );
        self.database.try_catch_up_with_primary().unwrap_or(());

        // add block to db
        let key = block.state_hash.as_bytes();
        let value = serde_json::to_vec(&block)?;
        let blocks_cf = self.blocks_cf();
        self.database.put_cf(&blocks_cf, key, value)?;

        // add block commands
        self.add_commands(block)?;

        // add new block db event only after all other data is added
        let db_event = DbEvent::Block(DbBlockEvent::NewBlock {
            state_hash: block.state_hash.clone(),
            blockchain_length: block.blockchain_length,
        });
        self.add_event(&IndexerEvent::Db(db_event.clone()))?;

        Ok(db_event)
    }

    /// Get the block with the specified hash
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting block with hash {}", state_hash.0);
        self.database.try_catch_up_with_primary().unwrap_or(());

        let key = state_hash.0.as_bytes();
        let blocks_cf = self.blocks_cf();
        match self
            .database
            .get_pinned_cf(&blocks_cf, key)?
            .map(|bytes| bytes.to_vec())
        {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        }
    }
}

impl CanonicityStore for IndexerStore {
    /// Add a canonical state hash at the specified blockchain_length
    fn add_canonical_block(&self, height: u32, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!(
            "Adding canonical block at height {height} with hash {}",
            state_hash.0
        );
        self.database.try_catch_up_with_primary().unwrap_or(());

        // height -> state hash
        let key = height.to_be_bytes();
        let value = serde_json::to_vec(&state_hash)?;
        let canonicity_cf = self.canonicity_cf();
        self.database.put_cf(&canonicity_cf, key, value)?;

        // update canonical chain length
        self.set_max_canonical_blockchain_length(height)?;

        // record new canonical block event
        self.add_event(&IndexerEvent::Db(DbEvent::Canonicity(
            DbCanonicityEvent::NewCanonicalBlock {
                state_hash: state_hash.0.clone(),
                blockchain_length: height,
            },
        )))?;

        Ok(())
    }

    /// Get the state hash of the canonical block with the specified blockchain_length
    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting canonical hash at height {height}");
        self.database.try_catch_up_with_primary().unwrap_or(());

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

    /// Get the length of the canonical chain
    fn get_max_canonical_blockchain_length(&self) -> anyhow::Result<Option<u32>> {
        trace!("Getting max canonical blockchain length");
        self.database.try_catch_up_with_primary().unwrap_or(());

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

    /// Set the length of the canonical chain
    fn set_max_canonical_blockchain_length(&self, height: u32) -> anyhow::Result<()> {
        trace!("Setting max canonical blockchain length to {height}");
        self.database.try_catch_up_with_primary().unwrap_or(());

        let canonicity_cf = self.canonicity_cf();
        let value = serde_json::to_vec(&height)?;
        self.database
            .put_cf(&canonicity_cf, Self::MAX_CANONICAL_KEY, value)?;
        Ok(())
    }

    /// Get the specified block's canonicity
    fn get_block_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        trace!("Getting canonicity of block with hash {}", state_hash.0);
        self.database.try_catch_up_with_primary().unwrap_or(());

        if let Some(PrecomputedBlock {
            blockchain_length, ..
        }) = self.get_block(state_hash)?
        {
            if let Some(max_canonical_length) = self.get_max_canonical_blockchain_length()? {
                if blockchain_length > max_canonical_length {
                    return Ok(Some(Canonicity::Pending));
                } else if self.get_canonical_hash_at_height(blockchain_length)?
                    == Some(state_hash.clone())
                {
                    return Ok(Some(Canonicity::Canonical));
                } else {
                    return Ok(Some(Canonicity::Orphaned));
                }
            }
        }
        Ok(None)
    }
}

impl LedgerStore for IndexerStore {
    fn add_ledger(&self, ledger_hash: &str, ledger: Ledger) -> anyhow::Result<()> {
        trace!("Adding ledger at {}", ledger_hash);
        self.database.try_catch_up_with_primary().unwrap_or(());

        // add ledger to db
        let key = ledger_hash.as_bytes();
        let value = ledger.to_string();
        let value = value.as_bytes();
        let ledgers_cf = self.ledgers_cf();
        self.database.put_cf(&ledgers_cf, key, value)?;

        // add new ledger event
        self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
            DbLedgerEvent::NewLedger {
                hash: ledger_hash.into(),
            },
        )))?;
        Ok(())
    }

    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()> {
        trace!("Adding ledger at state hash {}", state_hash.0);
        self.database.try_catch_up_with_primary().unwrap_or(());

        // add ledger to db
        let key = state_hash.0.as_bytes();
        let value = ledger.to_string();
        let value = value.as_bytes();
        let ledgers_cf = self.ledgers_cf();
        self.database.put_cf(&ledgers_cf, key, value)?;

        // add new ledger event
        self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
            DbLedgerEvent::NewLedger {
                hash: state_hash.0.clone(),
            },
        )))?;
        Ok(())
    }

    fn get_ledger_state_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting ledger at state hash {}", state_hash.0);
        self.database.try_catch_up_with_primary().unwrap_or(());

        let ledgers_cf = self.ledgers_cf();
        let mut state_hash = state_hash.clone();
        let mut to_apply = vec![];

        // walk chain back to a stored ledger (canonical)
        // collect blocks to compute the current ledger
        while self
            .database
            .get_pinned_cf(&ledgers_cf, state_hash.0.as_bytes())?
            .is_none()
        {
            if let Some(block) = self.get_block(&state_hash)? {
                to_apply.push(block.clone());
                state_hash = block.previous_state_hash();
            } else {
                return Ok(None);
            }
        }

        to_apply.reverse();

        let key = state_hash.0.as_bytes();
        if let Some(mut ledger) = self
            .database
            .get_pinned_cf(&ledgers_cf, key)?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
        {
            for block in to_apply {
                ledger.apply_post_balances(&block);
            }

            return Ok(Some(ledger));
        }

        Ok(None)
    }

    /// Get the ledger at the given ledger hash
    fn get_ledger(&self, ledger_hash: &str) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting ledger at {ledger_hash}");
        self.database.try_catch_up_with_primary().unwrap_or(());

        let ledgers_cf = self.ledgers_cf();
        let key = ledger_hash.as_bytes();
        if let Some(ledger) = self
            .database
            .get_pinned_cf(&ledgers_cf, key)?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
        {
            return Ok(Some(ledger));
        }

        Ok(None)
    }

    /// Get the canonical ledger at the specified height
    fn get_ledger_at_height(&self, height: u32) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting ledger at height {height}");
        self.database.try_catch_up_with_primary().unwrap_or(());

        match self.get_canonical_hash_at_height(height)? {
            None => Ok(None),
            Some(state_hash) => self.get_ledger_state_hash(&state_hash),
        }
    }
}

impl EventStore for IndexerStore {
    fn add_event(&self, event: &IndexerEvent) -> anyhow::Result<u32> {
        let seq_num = self.get_next_seq_num()?;
        trace!("Adding event {seq_num}: {:?}", event);

        if let IndexerEvent::WitnessTree(_) = event {
            return Ok(seq_num);
        }
        self.database.try_catch_up_with_primary().unwrap_or(());

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
        self.database.try_catch_up_with_primary().unwrap_or(());

        let key = seq_num.to_be_bytes();
        let events_cf = self.events_cf();
        let event = self.database.get_pinned_cf(&events_cf, key)?;
        let event = event.map(|bytes| serde_json::from_slice(&bytes).unwrap());

        trace!("Getting event {seq_num}: {:?}", event.clone().unwrap());
        Ok(event)
    }

    fn get_next_seq_num(&self) -> anyhow::Result<u32> {
        trace!("Getting next event sequence number");
        self.database.try_catch_up_with_primary().unwrap_or(());

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

impl CommandStore for IndexerStore {
    fn add_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!(
            "Adding commands from block (length {}) {}",
            block.blockchain_length,
            block.state_hash
        );

        let commands_cf = self.commands_cf();
        let signed_commands = SignedCommand::from_precomputed(block);

        // add: command hash -> signed command with state hash
        for signed_command in &signed_commands {
            let hash = signed_command.hash_signed_command()?;
            trace!(
                "Adding command hash {hash} (length {}, state hash {})",
                block.blockchain_length,
                block.state_hash.clone(),
            );

            let key = hash.as_bytes();
            let value = serde_json::to_vec(&SignedCommandWithStateHash::from(
                signed_command,
                &block.state_hash,
            ))?;
            self.database.put_cf(commands_cf, key, value)?;
        }

        // add: state hash -> user commands with status
        let key = block.state_hash.as_bytes();
        let value = serde_json::to_vec(&block.commands())?;
        self.database.put_cf(&commands_cf, key, value)?;

        // add: "pk -> linked list of signed commands with state hash"
        for pk in block.all_public_keys() {
            let pk_str = pk.to_address();
            trace!("Adding command pk {pk}");

            // get pk num commands
            let n = self.get_pk_num_commands(&pk_str)?.unwrap_or(0);
            let block_pk_commands: Vec<SignedCommandWithStateHash> = signed_commands
                .iter()
                .filter(|cmd| cmd.contains_public_key(&pk))
                .map(|c| SignedCommandWithStateHash::from(c, &block.state_hash))
                .collect();

            if !block_pk_commands.is_empty() {
                // write these commands to the next key for pk
                let key = format!("{pk_str}{n}").as_bytes().to_vec();
                let value = serde_json::to_vec(&block_pk_commands)?;
                self.database.put_cf(commands_cf, &key, value)?;

                // update pk's num commands
                let key = pk_str.as_bytes();
                let next_n = (n + 1).to_string();
                let value = next_n.as_bytes();
                self.database.put_cf(&commands_cf, key, value)?;
            }
        }

        Ok(())
    }

    fn get_command_by_hash(
        &self,
        command_hash: &str,
    ) -> anyhow::Result<Option<SignedCommandWithStateHash>> {
        trace!("Getting command by hash {}", command_hash);
        self.database.try_catch_up_with_primary().unwrap_or(());

        let key = command_hash.as_bytes();
        let commands_cf = self.commands_cf();
        if let Some(commands_bytes) = self.database.get_pinned_cf(commands_cf, key)? {
            return Ok(Some(serde_json::from_slice(&commands_bytes)?));
        }
        Ok(None)
    }

    fn get_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<UserCommandWithStatus>>> {
        trace!("Getting commands in block {}", state_hash.0);
        self.database.try_catch_up_with_primary().unwrap_or(());

        let key = state_hash.0.as_bytes();
        let commands_cf = self.commands_cf();
        if let Some(commands_bytes) = self.database.get_pinned_cf(commands_cf, key)? {
            return Ok(Some(serde_json::from_slice(&commands_bytes)?));
        }
        Ok(None)
    }

    fn get_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithStateHash>>> {
        let pk = pk.to_address();
        trace!("Getting commands for public key {pk}");
        self.database.try_catch_up_with_primary().unwrap_or(());

        let commands_cf = self.commands_cf();
        let mut commands = None;
        fn key_n(pk: String, n: u32) -> Vec<u8> {
            format!("{pk}{n}").as_bytes().to_vec()
        }

        if let Some(n) = self.get_pk_num_commands(&pk)? {
            for m in 0..n {
                if let Some(mut block_m_commands) = self
                    .database
                    .get_pinned_cf(commands_cf, key_n(pk.clone(), m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<SignedCommandWithStateHash>>(&bytes)
                            .expect("signed commands with state hash")
                    })
                {
                    let mut cmds = commands.unwrap_or(vec![]);
                    cmds.append(&mut block_m_commands);
                    commands = Some(cmds);
                } else {
                    commands = None;
                    break;
                }
            }
        }

        // only returns some if all fetches are successful
        Ok(commands)
    }

    fn get_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithStateHash>>> {
        let start_block_opt = self.get_block(start_state_hash)?;
        let end_block_opt = self.get_block(end_state_hash)?;

        if let (Some(start_block), Some(end_block)) = (start_block_opt, end_block_opt) {
            let start_height = start_block.blockchain_length;
            let end_height = end_block.blockchain_length;

            if end_height < start_height {
                warn!("Block (length {end_height}) {end_state_hash} is lower than block (length {start_height}) {start_state_hash}");
                return Ok(None);
            }

            let mut num = end_height - start_height;
            let mut prev_hash = end_block.previous_state_hash();
            let mut state_hashes: Vec<BlockHash> = vec![end_block.state_hash.into()];
            while let Some(block) = self.get_block(&prev_hash)? {
                if num == 0 {
                    break;
                }

                num -= 1;
                state_hashes.push(prev_hash);
                prev_hash = block.previous_state_hash();
            }

            if let Some(commands) = self.get_commands_for_public_key(pk)? {
                return Ok(Some(
                    commands
                        .into_iter()
                        .filter(|c| state_hashes.contains(&c.state_hash))
                        .collect(),
                ));
            }
        }

        Ok(None)
    }

    /// Number of blocks containing `pk` commands
    fn get_pk_num_commands(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        let key = pk.as_bytes();
        Ok(self
            .database
            .get_pinned_cf(self.commands_cf(), key)?
            .map(|bytes| {
                let s = String::from_utf8(bytes.to_vec()).expect("valid utf8");
                info!("********* {}", s);
                s.parse().expect("n is u32")
            }))
    }
}

impl IndexerStore {
    const NEXT_EVENT_SEQ_NUM_KEY: &[u8] = "next_event_seq_num".as_bytes();
    const MAX_CANONICAL_KEY: &[u8] = "max_canonical_blockchain_length".as_bytes();

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
