use super::{
    column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, user_command_db_key_pk,
    username::UsernameStore, IndexerStore,
};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockComparison, BlockHash},
    command::{
        signed::{SignedCommand, SignedCommandWithData, TxnHash},
        store::UserCommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    constants::millis_to_iso_date_string,
    ledger::public_key::PublicKey,
    utility::store::{
        command::user::*, from_be_bytes, pk_key_prefix, pk_txn_sort_key_sort, u32_prefix_key,
    },
};
use anyhow::bail;
use log::{trace, warn};
use speedb::{DBIterator, IteratorMode, WriteBatch};
use std::path::PathBuf;

impl UserCommandStore for IndexerStore {
    fn add_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let user_commands = block.commands();

        // per block
        self.set_block_user_commands_batch(block, batch)?;
        self.set_block_user_commands_count_batch(&state_hash, user_commands.len() as u32, batch)?;
        self.set_block_username_updates_batch(&state_hash, &block.username_updates(), batch)?;

        // per command
        for command in &user_commands {
            let signed = SignedCommand::from(command.clone());
            let txn_hash = signed.hash_signed_command()?;
            trace!("Adding user command {txn_hash} block {}", block.summary());

            // add signed command
            batch.put_cf(
                self.user_commands_cf(),
                txn_block_key(&txn_hash, &state_hash),
                serde_json::to_vec(&SignedCommandWithData::from(
                    command,
                    &state_hash.0,
                    block.blockchain_length(),
                    block.timestamp(),
                    block.global_slot_since_genesis(),
                ))?,
            );

            // add state hash index
            self.set_user_command_state_hash_batch(state_hash.clone(), &txn_hash, batch)?;

            // add index for global slot sorting
            batch.put_cf(
                self.user_commands_slot_sort_cf(),
                txn_sort_key(block.global_slot_since_genesis(), &txn_hash, &state_hash),
                b"",
            );

            // add index for block height sorting
            batch.put_cf(
                self.user_commands_height_sort_cf(),
                txn_sort_key(block.blockchain_length(), &txn_hash, &state_hash),
                b"",
            );

            // increment counts
            self.increment_user_commands_counts(command, epoch)?;

            // add: `txn_hash -> global_slot`
            // so we can reconstruct the key
            batch.put_cf(
                self.user_commands_txn_hash_to_global_slot_cf(),
                txn_hash.ref_inner().as_bytes(),
                block.global_slot_since_genesis().to_be_bytes(),
            );

            // add sender index
            let sender = command.sender();
            batch.put_cf(
                self.txn_from_height_sort_cf(),
                pk_txn_sort_key(
                    &sender,
                    block.blockchain_length(),
                    command.nonce().0,
                    &txn_hash,
                    &state_hash,
                ),
                command.amount().to_be_bytes(),
            );
            batch.put_cf(
                self.txn_from_slot_sort_cf(),
                pk_txn_sort_key(
                    &sender,
                    block.global_slot_since_genesis(),
                    command.nonce().0,
                    &txn_hash,
                    &state_hash,
                ),
                command.amount().to_be_bytes(),
            );

            // add receiver index
            let receiver = command.receiver();
            batch.put_cf(
                self.txn_to_height_sort_cf(),
                pk_txn_sort_key(
                    &receiver,
                    block.blockchain_length(),
                    command.nonce().0,
                    &txn_hash,
                    &state_hash,
                ),
                command.amount().to_be_bytes(),
            );
            batch.put_cf(
                self.txn_to_slot_sort_cf(),
                pk_txn_sort_key(
                    &receiver,
                    block.global_slot_since_genesis(),
                    command.nonce().0,
                    &txn_hash,
                    &state_hash,
                ),
                command.amount().to_be_bytes(),
            );
        }

        // per account
        // add: "pk -> linked list of signed commands with state hash"
        for pk in block.all_command_public_keys() {
            let n = self
                .get_pk_num_user_commands_blocks(&pk)?
                .unwrap_or_default();
            let block_pk_commands: Vec<SignedCommandWithData> = user_commands
                .iter()
                .filter(|cmd| cmd.contains_public_key(&pk))
                .map(|c| {
                    SignedCommandWithData::from(
                        c,
                        &state_hash.0,
                        block.blockchain_length(),
                        block.timestamp(),
                        block.global_slot_since_genesis(),
                    )
                })
                .collect();

            if !block_pk_commands.is_empty() {
                // write these commands to the next key for pk
                batch.put_cf(
                    self.user_commands_pk_cf(),
                    user_command_db_key_pk(&pk.0, n),
                    serde_json::to_vec(&block_pk_commands)?,
                );

                // update pk's num commands
                batch.put_cf(
                    self.user_commands_pk_num_cf(),
                    pk.0.as_bytes(),
                    (n + 1).to_be_bytes(),
                );
            }
        }
        Ok(())
    }

    fn get_user_command(
        &self,
        txn_hash: &TxnHash,
        index: u32,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} index {index}");
        Ok(self
            .get_user_command_state_hashes(txn_hash)
            .ok()
            .flatten()
            .and_then(|blocks| {
                self.get_user_command_state_hash(
                    txn_hash,
                    blocks.get(index as usize).expect("user command in block"),
                )
                .expect("user command in block at index")
            }))
    }

    fn get_user_command_state_hash(
        &self,
        txn_hash: &TxnHash,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} in block {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.user_commands_cf(), txn_block_key(txn_hash, state_hash))?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_user_command_state_hashes(
        &self,
        txn_hash: &TxnHash,
    ) -> anyhow::Result<Option<Vec<BlockHash>>> {
        trace!("Getting user command blocks {txn_hash}");
        Ok(self
            .database
            .get_pinned_cf(
                self.user_command_state_hashes_cf(),
                txn_hash.ref_inner().as_bytes(),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn set_user_command_state_hash_batch(
        &self,
        state_hash: BlockHash,
        txn_hash: &TxnHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting user command {txn_hash} block {state_hash}");
        let mut blocks = self
            .get_user_command_state_hashes(txn_hash)?
            .unwrap_or_default();
        blocks.push(state_hash);

        let mut block_cmps: Vec<BlockComparison> = blocks
            .iter()
            .filter_map(|b| self.get_block_comparison(b).ok())
            .flatten()
            .collect();
        block_cmps.sort();

        // set num containing blocks
        let blocks: Vec<BlockHash> = block_cmps.into_iter().map(|c| c.state_hash).collect();
        batch.put_cf(
            self.user_commands_num_containing_blocks_cf(),
            txn_hash.ref_inner().as_bytes(),
            (blocks.len() as u32).to_be_bytes(),
        );

        // set containing blocks
        batch.put_cf(
            self.user_command_state_hashes_cf(),
            txn_hash.ref_inner().as_bytes(),
            serde_json::to_vec(&blocks)?,
        );
        Ok(())
    }

    fn set_block_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        trace!("Setting block user commands {state_hash}");
        batch.put_cf(
            self.user_commands_per_block_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&block.commands())?,
        );
        Ok(())
    }

    fn get_block_user_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<UserCommandWithStatus>>> {
        trace!("Getting block user commands {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.user_commands_per_block_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithData>>> {
        trace!("Getting user commands for public key {pk}");

        let mut commands = vec![];
        fn key_n(pk: &PublicKey, n: u32) -> Vec<u8> {
            user_command_db_key_pk(&pk.0, n).to_vec()
        }

        if let Some(n) = self.get_pk_num_user_commands_blocks(pk)? {
            // collect user commands from all pk's blocks
            for m in 0..n {
                if let Some(mut block_m_commands) = self
                    .database
                    .get_pinned_cf(self.user_commands_pk_cf(), key_n(pk, m))?
                    .and_then(|bytes| {
                        serde_json::from_slice::<Vec<SignedCommandWithData>>(&bytes).ok()
                    })
                {
                    commands.append(&mut block_m_commands);
                } else {
                    commands.clear();
                    break;
                }
            }
            return Ok(Some(commands));
        }
        Ok(None)
    }

    fn get_user_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>> {
        let start_block_opt = self.get_block(start_state_hash)?.map(|b| b.0);
        let end_block_opt = self.get_block(end_state_hash)?.map(|b| b.0);
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
            while let Some((block, _)) = self.get_block(&prev_hash)? {
                if num == 0 {
                    break;
                }

                num -= 1;
                state_hashes.push(prev_hash);
                prev_hash = block.previous_state_hash();
            }

            if let Ok(Some(cmds)) = self.get_user_commands_for_public_key(pk) {
                return Ok(cmds
                    .into_iter()
                    .filter(|c| state_hashes.contains(&c.state_hash))
                    .collect());
            }
        }
        Ok(vec![])
    }

    fn get_user_commands_num_containing_blocks(
        &self,
        txn_hash: &TxnHash,
    ) -> anyhow::Result<Option<u32>> {
        trace!("Getting user commands num containing blocks {txn_hash}");
        Ok(self
            .database
            .get_cf(
                self.user_commands_num_containing_blocks_cf(),
                txn_hash.ref_inner().as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn write_user_commands_csv(
        &self,
        pk: &PublicKey,
        path: Option<PathBuf>,
    ) -> anyhow::Result<PathBuf> {
        let mut txns = vec![];
        let start = pk_txn_sort_key_prefix(pk, u32::MAX);
        let mode = IteratorMode::From(&start, speedb::Direction::Reverse);

        // from txns
        for (key, _) in self.txn_from_height_iterator(mode).flatten() {
            let txn_pk = pk_key_prefix(&key);
            if txn_pk != *pk {
                break;
            }
            let height = pk_txn_sort_key_sort(&key);
            let nonce = pk_txn_sort_key_nonce(&key);
            let txn_hash = txn_hash_of_key(&key);
            txns.push((height, nonce, txn_hash));
        }

        // to txns
        for (key, _) in self.txn_to_height_iterator(mode).flatten() {
            let txn_pk = pk_key_prefix(&key);
            if txn_pk != *pk {
                break;
            }
            let height = pk_txn_sort_key_sort(&key);
            let nonce = pk_txn_sort_key_nonce(&key);
            let txn_hash = txn_hash_of_key(&key);
            txns.push((height, nonce, txn_hash));
        }

        txns.sort();
        txns.reverse();

        // write txn records to csv
        let path = if let Some(path) = path {
            path.display().to_string()
        } else {
            let dir = if let Ok(dir) = std::env::var("VOLUMES_DIR") {
                dir
            } else {
                "/mnt".into()
            };
            format!("{dir}/mina-indexer-user-commands/{pk}.csv")
        };
        let mut csv_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_path(path.clone())?;
        for (_, _, txn_hash) in txns {
            if let Some(cmd) = self.get_user_command(&txn_hash, 0)?.as_ref() {
                csv_writer.serialize(TxnCsvRecord::from_user_command(cmd))?;
            } else {
                bail!("User command missing: {txn_hash}")
            }
        }

        csv_writer.flush()?;
        Ok(path.into())
    }

    ///////////////
    // Iterators //
    ///////////////

    fn user_commands_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.user_commands_slot_sort_cf(), mode)
    }

    fn user_commands_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.user_commands_height_sort_cf(), mode)
    }

    fn txn_from_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.txn_from_height_sort_cf(), mode)
    }

    fn txn_from_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.txn_from_slot_sort_cf(), mode)
    }

    fn txn_to_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.txn_to_height_sort_cf(), mode)
    }

    fn txn_to_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database.iterator_cf(self.txn_to_slot_sort_cf(), mode)
    }

    /////////////////////////
    // User command counts //
    /////////////////////////

    fn get_pk_num_user_commands_blocks(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting number of user commands for {pk}");
        Ok(self
            .database
            .get_cf(self.user_commands_pk_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_user_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting user command epoch {epoch}");
        Ok(self
            .database
            .get_cf(self.user_commands_epoch_cf(), epoch.to_be_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing user command epoch {epoch}");
        let old = self.get_user_commands_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.user_commands_epoch_cf(),
            epoch.to_be_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_user_commands_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting user command total");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing user command total");
        let old = self.get_user_commands_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_USER_COMMANDS_KEY, (old + 1).to_be_bytes())?)
    }

    fn get_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting user command epoch {epoch} num {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.user_commands_pk_epoch_cf(), u32_prefix_key(epoch, pk))?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} user commands count {pk}");
        let old = self.get_user_commands_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.database.put_cf(
            self.user_commands_pk_epoch_cf(),
            u32_prefix_key(epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total user commands count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.user_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing user command pk total num {pk}");
        let old = self.get_user_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.user_commands_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn set_block_user_commands_count_batch(
        &self,
        state_hash: &BlockHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block user command count {state_hash} -> {count}");
        batch.put_cf(
            self.block_user_command_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        );
        Ok(())
    }

    fn get_block_user_commands_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block user command count {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.block_user_command_counts_cf(), state_hash.0.as_bytes())?
            .map(|bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_user_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Incrementing user commands counts {:?}",
            command.to_command()
        );

        // sender epoch & total
        let sender = command.sender();
        self.increment_user_commands_pk_epoch_count(&sender, epoch)?;
        self.increment_user_commands_pk_total_count(&sender)?;

        // receiver epoch & total
        let receiver = command.receiver();
        if sender != receiver {
            self.increment_user_commands_pk_epoch_count(&receiver, epoch)?;
            self.increment_user_commands_pk_total_count(&receiver)?;
        }

        // epoch & total counts
        self.increment_user_commands_epoch_count(epoch)?;
        self.increment_user_commands_total_count()
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct TxnCsvRecord<'a> {
    date: String,
    block_height: u32,
    block_state_hash: &'a str,
    from: String,
    to: String,
    nonce: u32,
    hash: &'a str,
    fee: u64,
    amount: u64,
    memo: String,
    kind: String,
}

impl<'a> TxnCsvRecord<'a> {
    fn from_user_command(cmd: &'a SignedCommandWithData) -> Self {
        Self {
            date: millis_to_iso_date_string(cmd.date_time as i64),
            block_height: cmd.blockchain_length,
            block_state_hash: &cmd.state_hash.0,
            from: cmd.command.source_pk().0,
            to: cmd.command.receiver_pk().0,
            nonce: cmd.nonce.0,
            hash: cmd.tx_hash.ref_inner(),
            fee: cmd.command.fee(),
            amount: cmd.command.amount(),
            memo: cmd.command.memo(),
            kind: cmd.command.kind().to_string(),
        }
    }
}
