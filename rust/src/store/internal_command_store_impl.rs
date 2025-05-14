//! Internal command store impl

use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{
        precomputed::PrecomputedBlock,
        store::{BlockStore, DbBlockUpdate},
    },
    command::internal::{store::InternalCommandStore, DbInternalCommandWithData},
    constants::millis_to_iso_date_string,
    store::Result,
    utility::store::{
        block::{epoch_key, epoch_pk_key},
        command::internal::*,
        common::{from_be_bytes, pk_key_prefix, pk_txn_sort_key_sort, U32_LEN},
    },
};
use anyhow::{bail, Context};
use log::trace;
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::path::PathBuf;

impl InternalCommandStore for IndexerStore {
    /// Index internal commands on public keys & state hash
    fn add_internal_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Adding internal commands for block {}", block.summary());

        // block data
        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let genesis_state_hash = block.genesis_state_hash();

        // internal commands
        let internal_cmds_with_data = DbInternalCommandWithData::from_precomputed(block);

        // per block internal command count
        self.set_block_internal_commands_count_batch(
            &state_hash,
            internal_cmds_with_data.len() as u32,
            batch,
        )?;

        self.database.put_cf(
            self.internal_commands_block_num_cf(),
            state_hash.0.as_bytes(),
            (internal_cmds_with_data.len() as u32).to_be_bytes(),
        )?;

        // increment internal command counts &
        // sort by pk, block height & global slot
        self.increment_internal_commands_total_count(internal_cmds_with_data.len() as u32)?;

        for (i, int_cmd) in internal_cmds_with_data.iter().enumerate() {
            let pk = int_cmd.recipient();

            self.increment_internal_commands_counts(int_cmd, epoch, &genesis_state_hash)?;
            self.set_block_internal_command(block, i as u32, int_cmd)?;
            self.set_pk_internal_command(&pk, int_cmd)?;

            // sort data
            let value = serde_json::to_vec(int_cmd)?;
            self.database.put_cf(
                self.internal_commands_pk_block_height_sort_cf(),
                internal_commmand_pk_sort_key(
                    &pk,
                    block_height,
                    &state_hash,
                    i as u32,
                    int_cmd.kind(),
                ),
                &value,
            )?;

            self.database.put_cf(
                self.internal_commands_pk_global_slot_sort_cf(),
                internal_commmand_pk_sort_key(
                    &pk,
                    global_slot,
                    &state_hash,
                    i as u32,
                    int_cmd.kind(),
                ),
                &value,
            )?;
        }

        Ok(())
    }

    fn set_block_internal_command(
        &self,
        block: &PrecomputedBlock,
        index: u32,
        internal_command: &DbInternalCommandWithData,
    ) -> Result<()> {
        trace!(
            "Setting block internal command index {index}: {}",
            block.summary()
        );

        let state_hash = block.state_hash();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();

        // value
        let value = serde_json::to_vec(internal_command)?;

        // store
        self.database.put_cf(
            self.internal_commands_cf(),
            internal_commmand_block_key(&state_hash, index),
            &value,
        )?;

        // sort by block height
        self.database.put_cf(
            self.internal_commands_block_height_sort_cf(),
            internal_commmand_sort_key(block_height, &state_hash, index),
            &value,
        )?;

        // sort by global slot
        self.database.put_cf(
            self.internal_commands_global_slot_sort_cf(),
            internal_commmand_sort_key(global_slot, &state_hash, index),
            &value,
        )?;

        Ok(())
    }

    fn get_block_internal_command(
        &self,
        state_hash: &StateHash,
        index: u32,
    ) -> Result<Option<DbInternalCommandWithData>> {
        trace!("Getting internal command block {state_hash} index {index}");

        Ok(self
            .database
            .get_cf(
                self.internal_commands_cf(),
                internal_commmand_block_key(state_hash, index),
            )?
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("block {} index {}", state_hash, index))
                    .expect("internal command")
            }))
    }

    fn set_pk_internal_command(
        &self,
        pk: &PublicKey,
        internal_command: &DbInternalCommandWithData,
    ) -> Result<()> {
        let n = self.get_pk_num_internal_commands(pk)?.unwrap_or_default();
        trace!("Setting internal command pk {pk} index {n}");

        self.database.put_cf(
            self.internal_commands_pk_cf(),
            internal_command_pk_key(pk, n),
            serde_json::to_vec(internal_command)?,
        )?;

        self.database.put_cf(
            self.internal_commands_pk_num_cf(),
            pk.0.as_bytes(),
            (n + 1).to_be_bytes(),
        )?;

        Ok(())
    }

    fn get_pk_internal_command(
        &self,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<DbInternalCommandWithData>> {
        trace!("Getting internal command pk {pk} index {index}");

        Ok(self
            .database
            .get_cf(
                self.internal_commands_pk_cf(),
                internal_commmand_pk_key(pk, index),
            )?
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("pk {} index {}", pk, index))
                    .expect("pk internal command")
            }))
    }

    fn get_internal_commands(
        &self,
        state_hash: &StateHash,
    ) -> Result<Vec<DbInternalCommandWithData>> {
        trace!("Getting internal commands in block {state_hash}");
        let mut res = vec![];

        if let Some(num) = self.get_block_internal_commands_count(state_hash)? {
            for n in 0..num {
                res.push(
                    self.get_block_internal_command(state_hash, n)?
                        .expect("internal command exists"),
                );
            }
        }

        Ok(res)
    }

    fn get_internal_commands_public_key(
        &self,
        pk: &PublicKey,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<DbInternalCommandWithData>> {
        trace!("Getting internal commands for public key {pk}");
        let mut internal_cmds = vec![];

        if let Some(n) = self.get_pk_num_internal_commands(pk)? {
            for m in offset as u32..std::cmp::min(limit as u32, n) {
                if let Some(internal_command) = self.get_pk_internal_command(pk, m)? {
                    internal_cmds.push(internal_command);
                } else {
                    internal_cmds.clear();
                    break;
                }
            }
        }

        Ok(internal_cmds)
    }

    /// Number of blocks containing `pk` internal commands
    fn get_pk_num_internal_commands(&self, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting pk num internal commands {pk}");
        Ok(self
            .database
            .get_cf(self.internal_commands_pk_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn write_internal_commands_csv(&self, pk: PublicKey, path: Option<PathBuf>) -> Result<PathBuf> {
        let mut cmds = vec![];
        for (key, _) in self
            .internal_commands_pk_block_height_iterator(pk.clone(), Direction::Reverse)
            .flatten()
        {
            let cmd_pk = pk_key_prefix(&key);
            if cmd_pk != pk {
                break;
            }

            let height = pk_txn_sort_key_sort(&key);
            let state_hash = internal_command_pk_sort_key_state_hash(&key);
            let index = internal_command_pk_sort_key_index(&key);
            let kind = internal_command_pk_sort_key_kind(&key);

            cmds.push((height, kind, index, state_hash));
        }

        cmds.sort();
        cmds.reverse();

        // write internal command records to csv
        let path = if let Some(path) = path {
            path.display().to_string()
        } else {
            let dir = if let Ok(dir) = std::env::var("VOLUMES_DIR") {
                dir
            } else {
                "/mnt".into()
            };

            format!("{dir}/mina-indexer-internal-commands/{pk}.csv")
        };

        let mut csv_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_path(path.clone())?;

        for (_, _, index, state_hash) in cmds {
            if let Some(cmd) = self
                .get_block_internal_command(&state_hash, index)?
                .as_ref()
            {
                csv_writer.serialize(CsvRecordInternalCommand::from_internal_command(cmd))?;
            } else {
                bail!("Internal command missing for block {state_hash}")
            }
        }

        csv_writer.flush()?;
        Ok(path.into())
    }

    ///////////////
    // Iterators //
    ///////////////

    /// Key-value pairs
    /// ```
    /// - key: {height}{state_hash}{index}{kind}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - state_hash: [StateHash] bytes
    /// - index:      [u32] BE bytes
    /// - kind:       0, 1, or 2
    /// ```
    /// Use with [internal_commmand_sort_key]
    fn internal_commands_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.internal_commands_block_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {global_slot}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - global_slot: [u32] BE bytes
    /// - state_hash:  [StateHash] bytes
    /// - index:       [u32] BE bytes
    /// - kind:        0, 1, or 2
    /// ```
    /// Use with [internal_commmand_sort_key]
    fn internal_commands_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.internal_commands_global_slot_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {recipient}{height}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - recipient:  [PublicKey] bytes
    /// - height:     [u32] BE bytes
    /// - state_hash: [StateHash] bytes
    /// - index:      [u32] BE bytes
    /// ```
    /// Use with [internal_commmand_pk_sort_key]
    fn internal_commands_pk_block_height_iterator(
        &self,
        pk: PublicKey,
        direction: Direction,
    ) -> DBIterator<'_> {
        let pk_bytes = pk.to_bytes();
        let mut start = [0; PublicKey::LEN + U32_LEN + 1];
        let mode = match direction {
            Direction::Forward => IteratorMode::From(&pk_bytes, direction),
            Direction::Reverse => {
                start[..PublicKey::LEN].copy_from_slice(&pk_bytes);
                start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());

                // need to start after the target account
                start[PublicKey::LEN..][U32_LEN..].copy_from_slice("D".as_bytes());
                IteratorMode::From(&start, direction)
            }
        };

        self.database
            .iterator_cf(self.internal_commands_pk_block_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {recipient}{global_slot}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - recipient:   [PublicKey] bytes
    /// - global_slot: [u32] BE bytes
    /// - state_hash:  [StateHash] bytes
    /// - index:       [u32] BE bytes
    /// ```
    /// Use with [internal_commmand_pk_sort_key]
    fn internal_commands_pk_global_slot_iterator(
        &self,
        pk: PublicKey,
        direction: Direction,
    ) -> DBIterator<'_> {
        let pk_bytes = pk.to_bytes();
        let mut start = [0; PublicKey::LEN + U32_LEN + 1];
        let mode = match direction {
            Direction::Forward => IteratorMode::From(&pk_bytes, direction),
            Direction::Reverse => {
                start[..PublicKey::LEN].copy_from_slice(&pk_bytes);
                start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());

                // need to start after the target account
                start[PublicKey::LEN..][U32_LEN..].copy_from_slice("D".as_bytes());
                IteratorMode::From(&start, direction)
            }
        };

        self.database
            .iterator_cf(self.internal_commands_pk_global_slot_sort_cf(), mode)
    }

    /////////////////////////////
    // Internal command counts //
    /////////////////////////////

    fn get_internal_commands_epoch_count(
        &self,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting internal command count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.internal_commands_epoch_cf(),
                epoch_key(genesis_state_hash, epoch),
            )?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_epoch_count(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing internal command count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        let old = self.get_internal_commands_epoch_count(Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.internal_commands_epoch_cf(),
            epoch_key(genesis_state_hash, epoch),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_internal_commands_total_count(&self) -> Result<u32> {
        trace!("Getting internal command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_FEE_TRANSFERS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_total_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing internal command count");

        let old = self.get_internal_commands_total_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FEE_TRANSFERS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn get_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        let best_block_genesis_hash = self.get_best_block_genesis_hash().unwrap();
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis state hash")
        });

        trace!(
            "Getting internal command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.internal_commands_pk_epoch_cf(),
                epoch_pk_key(genesis_state_hash, epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing internal command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        let old =
            self.get_internal_commands_pk_epoch_count(pk, Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.internal_commands_pk_epoch_cf(),
            epoch_pk_key(genesis_state_hash, epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_internal_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32> {
        trace!("Getting internal command count pk {}", pk);

        Ok(self
            .database
            .get_cf(self.internal_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()> {
        trace!("Incrementing internal command count pk {}", pk);

        let old = self.get_internal_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.internal_commands_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_block_internal_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>> {
        trace!("Getting block internal command count {}", state_hash);

        Ok(self
            .database
            .get_cf(
                self.block_internal_command_counts_cf(),
                state_hash.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn set_block_internal_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Setting block internal command count {state_hash} -> {count}");

        batch.put_cf(
            self.block_internal_command_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        );

        Ok(())
    }

    fn increment_internal_commands_counts(
        &self,
        internal_command: &DbInternalCommandWithData,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!("Incrementing internal command counts {internal_command:?}");

        // receiver epoch & total
        let receiver = &internal_command.recipient();
        self.increment_internal_commands_pk_epoch_count(receiver, epoch, genesis_state_hash)?;
        self.increment_internal_commands_pk_total_count(receiver)?;

        // epoch count
        self.increment_internal_commands_epoch_count(epoch, genesis_state_hash)
    }

    /// get canonical internal commands count
    fn get_canonical_internal_commands_count(&self) -> Result<u32> {
        trace!("Getting canonical internal command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY)?
            .map_or(0, from_be_bytes))
    }

    /// Increment canonical internal commands count
    fn increment_canonical_internal_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Increment canonical internal commands count");

        let old = self.get_canonical_internal_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    /// Decrement canonical internal commands count
    fn decrement_canonical_internal_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrement canonical internal commands count");

        let old = self.get_canonical_internal_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    /// Update Internal Commands from DbBlockUpdate
    fn update_internal_commands(&self, block: &DbBlockUpdate) -> Result<()> {
        for update in block.unapply.iter() {
            let internal_commands = self.get_internal_commands(&update.state_hash)?;
            self.decrement_canonical_internal_commands_count(internal_commands.len() as u32)?;
        }

        for update in block.apply.iter() {
            let internal_commands = self.get_internal_commands(&update.state_hash)?;
            self.increment_canonical_internal_commands_count(internal_commands.len() as u32)?;
        }

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct CsvRecordInternalCommand<'a> {
    date: String,
    block_height: u32,
    block_state_hash: &'a str,
    recipient: &'a str,
    amount: u64,
    kind: String,
}

impl<'a> CsvRecordInternalCommand<'a> {
    fn from_internal_command(cmd: &'a DbInternalCommandWithData) -> Self {
        use DbInternalCommandWithData::*;
        match cmd {
            Coinbase {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
            }
            | FeeTransfer {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
            } => Self {
                amount: *amount,
                recipient: &receiver.0,
                block_height: *block_height,
                block_state_hash: &state_hash.0,
                date: millis_to_iso_date_string(*date_time),
                kind: kind.to_string(),
            },
        }
    }
}

#[cfg(all(test, feature = "tier2"))]
mod tests {
    use super::InternalCommandStore;
    use crate::{
        base::public_key::PublicKey,
        block::{
            precomputed::{PcbVersion, PrecomputedBlock},
            store::BlockStore,
        },
        store::IndexerStore,
    };
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_indexer_store() -> anyhow::Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(std::env::current_dir()?)?;
        IndexerStore::new(temp_dir.path(), true)
    }

    #[test]
    fn test_incr_dec_canonical_internal_commands_count() -> anyhow::Result<()> {
        let store = create_indexer_store()?;

        // Test incrementing canonical internal commands count
        store.increment_canonical_internal_commands_count(1)?;
        assert_eq!(store.get_canonical_internal_commands_count()?, 1);

        // Increment again
        store.increment_canonical_internal_commands_count(1)?;
        assert_eq!(store.get_canonical_internal_commands_count()?, 2);

        // Test decrementing canonical internal commands count
        store.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(store.get_canonical_internal_commands_count()?, 1);

        // Decrement to 0
        store.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(store.get_canonical_internal_commands_count()?, 0);

        // Ensure count does not go below 0
        store.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(store.get_canonical_internal_commands_count()?, 0);

        Ok(())
    }

    #[test]
    fn genesis_v2() -> anyhow::Result<()> {
        let store = create_indexer_store()?;

        let path = PathBuf::from("./data/genesis_blocks/mainnet-359605-3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        let pk: PublicKey = "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".into();
        let state_hash = block.state_hash();

        // add the block
        store.add_block(&block, 0)?;

        // check block internal commands
        let block_internal_cmds = store.get_internal_commands(&state_hash)?;
        assert_eq!(block_internal_cmds, vec![]);

        // check pk internal commands
        let pk_internal_cmds: Vec<_> = store
            .internal_commands_pk_block_height_iterator(pk.clone(), speedb::Direction::Reverse)
            .flatten()
            .collect();
        assert_eq!(pk_internal_cmds, vec![]);

        let pk_internal_cmds: Vec<_> = store
            .internal_commands_pk_global_slot_iterator(pk.clone(), speedb::Direction::Reverse)
            .flatten()
            .collect();
        assert_eq!(pk_internal_cmds, vec![]);

        // check coinbase receiver
        let receiver = store.get_coinbase_receiver(&state_hash)?.unwrap();
        assert_eq!(receiver, pk);

        Ok(())
    }
}
