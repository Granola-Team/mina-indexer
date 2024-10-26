use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::{
    block::{
        precomputed::PrecomputedBlock,
        store::{BlockStore, DbBlockUpdate},
        BlockHash,
    },
    command::internal::{
        store::InternalCommandStore, DbInternalCommand, DbInternalCommandWithData,
    },
    constants::millis_to_iso_date_string,
    ledger::public_key::PublicKey,
    utility::store::{
        command::internal::*, from_be_bytes, pk_key_prefix, pk_txn_sort_key_sort, u32_prefix_key,
        U32_LEN,
    },
};
use anyhow::bail;
use log::trace;
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::path::PathBuf;

impl InternalCommandStore for IndexerStore {
    /// Index internal commands on public keys & state hash
    fn add_internal_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let date_time = block.timestamp() as i64;
        trace!("Adding internal commands for block {}", block.summary());

        // add cmds with data to public keys
        let internal_cmds_with_data: Vec<DbInternalCommandWithData> =
            DbInternalCommand::from_precomputed(block)
                .into_iter()
                .map(|c| {
                    DbInternalCommandWithData::from_internal_cmd(
                        c,
                        state_hash.clone(),
                        block_height,
                        date_time,
                    )
                })
                .collect();

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
            self.increment_internal_commands_counts(int_cmd, epoch)?;
            self.set_block_internal_command(block, i as u32, int_cmd)?;
            self.set_pk_internal_command(&pk, int_cmd)?;

            // sort data
            self.database.put_cf(
                self.internal_commands_pk_block_height_sort_cf(),
                internal_commmand_pk_sort_key(
                    &pk,
                    block_height,
                    &state_hash,
                    i as u32,
                    int_cmd.kind(),
                ),
                serde_json::to_vec(int_cmd)?,
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
                serde_json::to_vec(int_cmd)?,
            )?;
        }
        Ok(())
    }

    fn set_block_internal_command(
        &self,
        block: &PrecomputedBlock,
        index: u32,
        internal_command: &DbInternalCommandWithData,
    ) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        trace!("Setting block internal command {state_hash} index {index}");

        self.database.put_cf(
            self.internal_commands_cf(),
            internal_commmand_block_key(&state_hash, index),
            serde_json::to_vec(internal_command)?,
        )?;
        self.database.put_cf(
            self.internal_commands_block_height_sort_cf(),
            internal_commmand_sort_key(block_height, &state_hash, index),
            serde_json::to_vec(internal_command)?,
        )?;
        self.database.put_cf(
            self.internal_commands_global_slot_sort_cf(),
            internal_commmand_sort_key(global_slot, &state_hash, index),
            serde_json::to_vec(internal_command)?,
        )?;
        Ok(())
    }

    fn get_block_internal_command(
        &self,
        state_hash: &BlockHash,
        index: u32,
    ) -> anyhow::Result<Option<DbInternalCommandWithData>> {
        trace!("Getting internal command block {state_hash} index {index}");
        Ok(self
            .database
            .get_cf(
                self.internal_commands_cf(),
                internal_commmand_block_key(state_hash, index),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn set_pk_internal_command(
        &self,
        pk: &PublicKey,
        internal_command: &DbInternalCommandWithData,
    ) -> anyhow::Result<()> {
        let n = self.get_pk_num_internal_commands(pk)?.unwrap_or(0);
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
    ) -> anyhow::Result<Option<DbInternalCommandWithData>> {
        trace!("Getting internal command pk {pk} index {index}");
        Ok(self
            .database
            .get_cf(
                self.internal_commands_pk_cf(),
                internal_commmand_pk_key(pk, index),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_internal_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<DbInternalCommandWithData>> {
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
    ) -> anyhow::Result<Vec<DbInternalCommandWithData>> {
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
    fn get_pk_num_internal_commands(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting pk num internal commands {pk}");
        Ok(self
            .database
            .get_cf(self.internal_commands_pk_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn write_internal_commands_csv(
        &self,
        pk: PublicKey,
        path: Option<PathBuf>,
    ) -> anyhow::Result<PathBuf> {
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

    fn internal_commands_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.internal_commands_block_height_sort_cf(), mode)
    }

    fn internal_commands_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.internal_commands_global_slot_sort_cf(), mode)
    }

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

    fn get_internal_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting internal command epoch {epoch}");
        Ok(self
            .database
            .get_cf(self.internal_commands_epoch_cf(), epoch.to_be_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing internal command epoch {epoch}");
        let old = self.get_internal_commands_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.internal_commands_epoch_cf(),
            epoch.to_be_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_internal_commands_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting internal command total");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_FEE_TRANSFERS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_total_count(&self, incr: u32) -> anyhow::Result<()> {
        trace!("Incrementing internal command total");
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
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting internal command epoch {epoch} num {pk}");
        Ok(self
            .database
            .get_cf(
                self.internal_commands_pk_epoch_cf(),
                u32_prefix_key(epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} internal commands count {pk}");
        let old = self.get_internal_commands_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.database.put_cf(
            self.internal_commands_pk_epoch_cf(),
            u32_prefix_key(epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total internal commands count {pk}");
        Ok(self
            .database
            .get_cf(self.internal_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing internal command pk total num {pk}");
        let old = self.get_internal_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.internal_commands_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_block_internal_commands_count(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<u32>> {
        trace!("Getting block internal command count");
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
        state_hash: &BlockHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
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
    ) -> anyhow::Result<()> {
        let receiver = match internal_command {
            DbInternalCommandWithData::Coinbase { .. } => return Ok(()),
            DbInternalCommandWithData::FeeTransfer { receiver, .. } => receiver,
        };
        trace!("Incrementing internal command counts {internal_command:?}");

        // receiver epoch & total
        self.increment_internal_commands_pk_epoch_count(receiver, epoch)?;
        self.increment_internal_commands_pk_total_count(receiver)?;

        // epoch & total counts
        self.increment_internal_commands_epoch_count(epoch)
    }

    /// get canonical internal commands count
    fn get_canonical_internal_commands_count(&self) -> anyhow::Result<u32> {
        trace!("Getting canonical internal command count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY)?
            .map_or(0, from_be_bytes))
    }

    /// Increment canonical internal commands count
    fn increment_canonical_internal_commands_count(&self, incr: u32) -> anyhow::Result<()> {
        trace!("Increment canonical internal commands count");
        let old = self.get_canonical_internal_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    /// Decrement canonical internal commands count
    fn decrement_canonical_internal_commands_count(&self, incr: u32) -> anyhow::Result<()> {
        trace!("Decrement canonical internal commands count");
        let old = self.get_canonical_internal_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_FEE_TRANSFERS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    /// Update Internal Commands from DbBlockUpdate
    fn update_internal_commands(&self, block: &DbBlockUpdate) -> anyhow::Result<()> {
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

#[cfg(test)]
mod internal_command_store_impl_tests {
    use super::*;
    use anyhow::Result;
    use std::env;
    use tempfile::TempDir;

    // Utility function to create an in-memory IndexerStore for testing
    fn create_indexer_store() -> Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(env::current_dir()?)?;
        let store = IndexerStore::new(temp_dir.path())?;
        Ok(store)
    }

    #[test]
    fn test_incr_dec_canonical_internal_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Test incrementing canonical internal commands count
        indexer.increment_canonical_internal_commands_count(1)?;
        assert_eq!(indexer.get_canonical_internal_commands_count()?, 1);

        // Increment again
        indexer.increment_canonical_internal_commands_count(1)?;
        assert_eq!(indexer.get_canonical_internal_commands_count()?, 2);

        // Test decrementing canonical internal commands count
        indexer.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(indexer.get_canonical_internal_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(indexer.get_canonical_internal_commands_count()?, 0);

        // Ensure count does not go below 0
        indexer.decrement_canonical_internal_commands_count(1)?;
        assert_eq!(indexer.get_canonical_internal_commands_count()?, 0);

        Ok(())
    }
}
