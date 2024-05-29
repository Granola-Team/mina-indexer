use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    command::{
        signed::{SignedCommand, SignedCommandWithData},
        store::UserCommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    ledger::public_key::PublicKey,
    store::{
        from_be_bytes, to_be_bytes, txn_sort_key, u32_prefix_key, user_command_db_key,
        user_command_db_key_pk, IndexerStore,
    },
};
use log::{trace, warn};

// TODO add iterators

impl UserCommandStore for IndexerStore {
    fn add_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let epoch = block.epoch_count();
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
            if self
                .database
                .get_cf(
                    self.commands_txn_hash_to_global_slot_mainnet_cf(),
                    txn_hash.as_bytes(),
                )?
                .is_none()
            {
                // if not present already, increment counts
                self.increment_user_commands_counts(command, epoch)?;
            }

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
        self.database.put_cf(self.user_commands_cf(), key, value)?;

        // add: "pk -> linked list of signed commands with state hash"
        for pk in block.all_command_public_keys() {
            trace!("Adding user command for public key {}", pk.0);

            // get pk num commands
            let n = self.get_pk_num_user_commands(&pk.0)?.unwrap_or(0);
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
                self.database.put_cf(self.user_commands_cf(), key, value)?;

                // update pk's num commands
                let key = user_command_db_key(&pk.0);
                let next_n = (n + 1).to_string();
                self.database
                    .put_cf(self.user_commands_cf(), key, next_n.as_bytes())?;
            }
        }
        Ok(())
    }

    fn get_user_command_by_hash(
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

    fn get_user_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<UserCommandWithStatus>> {
        let state_hash = &state_hash.0;
        trace!("Getting user commands in block {}", state_hash);

        let key = user_command_db_key(state_hash);
        if let Some(commands_bytes) = self.database.get_pinned_cf(self.user_commands_cf(), key)? {
            return Ok(serde_json::from_slice(&commands_bytes)?);
        }
        Ok(vec![])
    }

    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<SignedCommandWithData>> {
        trace!("Getting user commands for public key {}", pk.0);

        let commands_cf = self.user_commands_cf();
        let mut commands = vec![];
        fn key_n(pk: &str, n: u32) -> Vec<u8> {
            user_command_db_key_pk(&pk.to_string(), n).to_vec()
        }

        if let Some(n) = self.get_pk_num_user_commands(&pk.0)? {
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

            return Ok(self
                .get_user_commands_for_public_key(pk)?
                .into_iter()
                .filter(|c| state_hashes.contains(&c.state_hash))
                .collect());
        }
        Ok(vec![])
    }

    fn get_pk_num_user_commands(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        trace!("Getting number of internal commands for {}", pk);

        let key = user_command_db_key(&pk.to_string());
        Ok(self
            .database
            .get_pinned_cf(self.user_commands_cf(), key)?
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }))
    }

    fn get_user_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting user command epoch {epoch}");
        Ok(self
            .database
            .get_cf(self.user_commands_epoch_cf(), to_be_bytes(epoch))?
            .map_or(0, from_be_bytes))
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
            .get_cf(
                self.user_commands_pk_epoch_cf(),
                u32_prefix_key(epoch, &pk.0),
            )?
            .map_or(0, from_be_bytes))
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
            .get_cf(self.user_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
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
