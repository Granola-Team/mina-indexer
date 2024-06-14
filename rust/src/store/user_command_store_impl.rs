use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockComparison, BlockHash},
    command::{
        signed::{SignedCommand, SignedCommandWithData},
        store::UserCommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    ledger::public_key::PublicKey,
    store::{
        from_be_bytes, pk_txn_sort_key, to_be_bytes, txn_block_key, txn_sort_key, u32_prefix_key,
        user_command_db_key_pk, username::UsernameStore, IndexerStore,
    },
};
use log::{trace, warn};
use speedb::{DBIterator, IteratorMode};

impl UserCommandStore for IndexerStore {
    fn add_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let user_commands = block.commands();

        // per block
        self.set_block_user_commands(block)?;
        self.set_block_user_commands_count(&state_hash, user_commands.len() as u32)?;
        self.set_block_username_updates(&state_hash, &block.username_updates())?;

        // per command
        for command in &user_commands {
            let signed = SignedCommand::from(command.clone());
            let txn_hash = signed.hash_signed_command()?;
            trace!("Adding user command {txn_hash} block {}", block.summary());

            // add signed command
            self.database.put_cf(
                self.user_commands_cf(),
                txn_block_key(&txn_hash, state_hash.clone()),
                serde_json::to_vec(&SignedCommandWithData::from(
                    command,
                    &block.state_hash().0,
                    block.blockchain_length(),
                    block.timestamp(),
                    block.global_slot_since_genesis(),
                ))?,
            )?;

            // add state hash index
            self.set_user_command_state_hash(state_hash.clone(), &txn_hash)?;

            // add index for global slot sorting
            self.database.put_cf(
                self.user_commands_slot_sort_cf(),
                txn_sort_key(
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    state_hash.clone(),
                ),
                b"",
            )?;

            // add index for block height sorting
            self.database.put_cf(
                self.user_commands_height_sort_cf(),
                txn_sort_key(block.blockchain_length(), &txn_hash, state_hash.clone()),
                b"",
            )?;

            // increment counts
            self.increment_user_commands_counts(command, epoch)?;

            // add: `txn_hash -> global_slot`
            // so we can reconstruct the key
            self.database.put_cf(
                self.user_commands_txn_hash_to_global_slot_cf(),
                txn_hash.as_bytes(),
                to_be_bytes(block.global_slot_since_genesis()),
            )?;

            // add sender index
            self.database.put_cf(
                self.txn_from_height_sort_cf(),
                pk_txn_sort_key(
                    command.sender(),
                    block.blockchain_length(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
            self.database.put_cf(
                self.txn_from_slot_sort_cf(),
                pk_txn_sort_key(
                    command.sender(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;

            // add receiver index
            self.database.put_cf(
                self.txn_to_height_sort_cf(),
                pk_txn_sort_key(
                    command.receiver(),
                    block.blockchain_length(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
            self.database.put_cf(
                self.txn_to_slot_sort_cf(),
                pk_txn_sort_key(
                    command.receiver(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
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
                        &block.state_hash().0,
                        block.blockchain_length(),
                        block.timestamp(),
                        block.global_slot_since_genesis(),
                    )
                })
                .collect();

            if !block_pk_commands.is_empty() {
                // write these commands to the next key for pk
                self.database.put_cf(
                    self.user_commands_pk_cf(),
                    user_command_db_key_pk(&pk.0, n),
                    serde_json::to_vec(&block_pk_commands)?,
                )?;

                // update pk's num commands
                self.database.put_cf(
                    self.user_commands_pk_num_cf(),
                    pk.0.as_bytes(),
                    to_be_bytes(n + 1),
                )?;
            }
        }
        Ok(())
    }

    fn get_user_command(
        &self,
        txn_hash: &str,
        index: u32,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} index {index}");
        Ok(self
            .get_user_command_state_hashes(txn_hash)
            .ok()
            .flatten()
            .and_then(|b| b.get(index as usize).cloned())
            .and_then(|state_hash| {
                self.get_user_command_state_hash(txn_hash, &state_hash)
                    .unwrap()
            }))
    }

    fn get_user_command_state_hash(
        &self,
        txn_hash: &str,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} in block {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(
                self.user_commands_cf(),
                txn_block_key(txn_hash, state_hash.clone()),
            )?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn get_user_command_state_hashes(
        &self,
        txn_hash: &str,
    ) -> anyhow::Result<Option<Vec<BlockHash>>> {
        trace!("Getting user command blocks {txn_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.user_command_state_hashes_cf(), txn_hash.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    fn set_user_command_state_hash(
        &self,
        state_hash: BlockHash,
        txn_hash: &str,
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

        let blocks: Vec<BlockHash> = block_cmps.into_iter().map(|c| c.state_hash).collect();
        // set num containing blocks
        self.database.put_cf(
            self.user_commands_num_containing_blocks_cf(),
            txn_hash.as_bytes(),
            to_be_bytes(blocks.len() as u32),
        )?;

        // set containing blocks
        self.database.put_cf(
            self.user_command_state_hashes_cf(),
            txn_hash.as_bytes(),
            serde_json::to_vec(&blocks)?,
        )?;
        Ok(())
    }

    fn set_block_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        trace!("Setting block user commands {state_hash}");
        Ok(self.database.put_cf(
            self.user_commands_per_block_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&block.commands())?,
        )?)
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
        txn_hash: &str,
    ) -> anyhow::Result<Option<u32>> {
        trace!("Getting user commands num containing blocks {txn_hash}");
        Ok(self
            .database
            .get_cf(
                self.user_commands_num_containing_blocks_cf(),
                txn_hash.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn user_commands_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.user_commands_slot_sort_cf(), mode)
    }

    fn user_commands_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.user_commands_height_sort_cf(), mode)
    }

    fn txn_from_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.txn_from_height_sort_cf(), mode)
    }

    fn txn_from_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.txn_from_slot_sort_cf(), mode)
    }

    fn txn_to_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.txn_to_height_sort_cf(), mode)
    }

    fn txn_to_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
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
            .get_pinned_cf(self.user_commands_epoch_cf(), to_be_bytes(epoch))?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_user_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing user command epoch {epoch}");
        let old = self.get_user_commands_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.user_commands_epoch_cf(),
            to_be_bytes(epoch),
            to_be_bytes(old + 1),
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
            .put(Self::TOTAL_NUM_USER_COMMANDS_KEY, to_be_bytes(old + 1))?)
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
            .get_pinned_cf(
                self.user_commands_pk_epoch_cf(),
                u32_prefix_key(epoch, &pk.0),
            )?
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
            u32_prefix_key(epoch, &pk.0),
            to_be_bytes(old + 1),
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
            to_be_bytes(old + 1),
        )?)
    }

    fn set_block_user_commands_count(
        &self,
        state_hash: &BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting block user command count {state_hash} -> {count}");
        Ok(self.database.put_cf(
            self.block_user_command_counts_cf(),
            state_hash.0.as_bytes(),
            to_be_bytes(count),
        )?)
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
