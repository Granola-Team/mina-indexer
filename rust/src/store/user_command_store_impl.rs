//! User command store impl

use super::{
    column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, user_command_db_key_pk,
    username::UsernameStore, IndexerStore,
};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{
        precomputed::PrecomputedBlock,
        store::{BlockStore, DbBlockUpdate},
        BlockComparison,
    },
    command::{
        signed::{SignedCommandWithData, TxnHash},
        store::UserCommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    constants::millis_to_iso_date_string,
    ledger::token::TokenAddress,
    store::zkapp::tokens::ZkappTokenStore,
    utility::store::{
        block::{epoch_key, epoch_pk_key},
        command::user::{
            pk_txn_sort_key, pk_txn_sort_key_nonce, token_txn_sort_key, txn_block_key,
            txn_hash_of_key, txn_sort_key,
        },
        common::{from_be_bytes, pk_key_prefix, pk_txn_sort_key_sort, U32_LEN},
    },
};
use anyhow::{bail, Context, Result};
use log::{trace, warn};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::path::PathBuf;

impl UserCommandStore for IndexerStore {
    fn add_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let genesis_state_hash = block.genesis_state_hash();

        let user_commands = block.commands();
        let zkapp_commands = block.zkapp_commands();

        // per block
        self.set_block_user_commands_batch(block, batch)?;
        self.set_block_user_commands_count_batch(&state_hash, user_commands.len() as u32, batch)?;
        self.set_block_zkapp_commands_count_batch(&state_hash, zkapp_commands.len() as u32, batch)?;
        self.set_block_username_updates_batch(&state_hash, &block.username_updates(), batch)?;

        // per command
        for command in &user_commands {
            let is_zkapp = command.is_zkapp_command();
            let txn_hash = command.txn_hash()?;
            trace!("Adding user command {txn_hash} block {}", block.summary());

            // add signed command
            let signed_command_with_data = SignedCommandWithData::from(
                command.clone(),
                &state_hash.0,
                block.blockchain_length(),
                block.timestamp(),
                block.global_slot_since_genesis(),
            );

            let value = serde_json::to_vec(&signed_command_with_data)?;
            batch.put_cf(
                self.user_commands_cf(),
                txn_block_key(&txn_hash, &state_hash),
                &value,
            );

            if is_zkapp {
                batch.put_cf(
                    self.zkapp_commands_cf(),
                    txn_block_key(&txn_hash, &state_hash),
                    &value,
                );
            }

            // add state hash index
            self.set_user_command_state_hash_batch(state_hash.clone(), &txn_hash, batch)?;

            // add index for block height sorting
            batch.put_cf(
                self.user_commands_height_sort_cf(),
                txn_sort_key(block.blockchain_length(), &txn_hash, &state_hash),
                &value,
            );

            if is_zkapp {
                batch.put_cf(
                    self.zkapp_commands_height_sort_cf(),
                    txn_sort_key(block.blockchain_length(), &txn_hash, &state_hash),
                    &value,
                );
            }

            // add index for global slot sorting
            batch.put_cf(
                self.user_commands_slot_sort_cf(),
                txn_sort_key(block.global_slot_since_genesis(), &txn_hash, &state_hash),
                &value,
            );

            if is_zkapp {
                batch.put_cf(
                    self.zkapp_commands_slot_sort_cf(),
                    txn_sort_key(block.global_slot_since_genesis(), &txn_hash, &state_hash),
                    &value,
                );
            }

            // add per token txns
            for token in command.tokens().iter() {
                batch.put_cf(
                    self.user_commands_per_token_height_sort_cf(),
                    token_txn_sort_key(token, block.blockchain_length(), &txn_hash, &state_hash),
                    &value,
                );

                batch.put_cf(
                    self.user_commands_per_token_slot_sort_cf(),
                    token_txn_sort_key(
                        token,
                        block.global_slot_since_genesis(),
                        &txn_hash,
                        &state_hash,
                    ),
                    &value,
                );
            }

            // increment counts
            self.increment_user_commands_counts(command, epoch, &genesis_state_hash)?;

            if is_zkapp {
                self.increment_zkapp_commands_counts(command, epoch, &genesis_state_hash)?;
            }

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
                &value,
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
                &value,
            );

            // add receiver index
            for receiver in command.receiver() {
                batch.put_cf(
                    self.txn_to_height_sort_cf(),
                    pk_txn_sort_key(
                        &receiver,
                        block.blockchain_length(),
                        command.nonce().0,
                        &txn_hash,
                        &state_hash,
                    ),
                    &value,
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
                    &value,
                );
            }
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
                        c.clone(),
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
    ) -> Result<Option<SignedCommandWithData>> {
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
                .with_context(|| format!("txn hash {txn_hash} index {index}"))
                .expect("user command in block at index")
            }))
    }

    fn get_user_command_state_hash(
        &self,
        txn_hash: &TxnHash,
        state_hash: &StateHash,
    ) -> Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} in block {state_hash}");
        Ok(self
            .database
            .get_cf(self.user_commands_cf(), txn_block_key(txn_hash, state_hash))?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("txn hash {txn_hash} state hash {state_hash}"))
                    .expect("user command")
            }))
    }

    fn get_user_command_state_hashes(&self, txn_hash: &TxnHash) -> Result<Option<Vec<StateHash>>> {
        trace!("Getting user command blocks {txn_hash}");
        Ok(self
            .database
            .get_cf(
                self.user_commands_state_hashes_cf(),
                txn_hash.ref_inner().as_bytes(),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("txn hash {txn_hash}"))
                    .expect("user command state hashes")
            }))
    }

    fn set_user_command_state_hash_batch(
        &self,
        state_hash: StateHash,
        txn_hash: &TxnHash,
        batch: &mut WriteBatch,
    ) -> Result<()> {
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
        let blocks: Vec<StateHash> = block_cmps.into_iter().map(|c| c.state_hash).collect();
        batch.put_cf(
            self.user_commands_num_containing_blocks_cf(),
            txn_hash.ref_inner().as_bytes(),
            (blocks.len() as u32).to_be_bytes(),
        );

        // set containing blocks
        batch.put_cf(
            self.user_commands_state_hashes_cf(),
            txn_hash.ref_inner().as_bytes(),
            serde_json::to_vec(&blocks)?,
        );
        Ok(())
    }

    fn set_block_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()> {
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
        state_hash: &StateHash,
    ) -> Result<Option<Vec<UserCommandWithStatus>>> {
        trace!("Getting block user commands {state_hash}");
        Ok(self
            .database
            .get_cf(self.user_commands_per_block_cf(), state_hash.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("state hash {state_hash}"))
                    .expect("user commands")
            }))
    }

    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> Result<Option<Vec<SignedCommandWithData>>> {
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
                    .get_cf(self.user_commands_pk_cf(), key_n(pk, m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<SignedCommandWithData>>(&bytes)
                            .with_context(|| format!("user command pk {pk} index {m}"))
                            .expect("")
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
        start_state_hash: &StateHash,
        end_state_hash: &StateHash,
    ) -> Result<Vec<SignedCommandWithData>> {
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
            let mut state_hashes: Vec<StateHash> = vec![end_block.state_hash()];

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

    fn get_user_commands_num_containing_blocks(&self, txn_hash: &TxnHash) -> Result<Option<u32>> {
        trace!("Getting user commands num containing blocks {txn_hash}");
        Ok(self
            .database
            .get_cf(
                self.user_commands_num_containing_blocks_cf(),
                txn_hash.ref_inner().as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn write_user_commands_csv(&self, pk: &PublicKey, path: Option<PathBuf>) -> Result<PathBuf> {
        let mut txns = vec![];
        let direction = Direction::Reverse;

        // from txns
        for (key, _) in self.txn_from_height_iterator(pk, direction).flatten() {
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
        for (key, _) in self.txn_to_height_iterator(pk, direction).flatten() {
            let txn_pk = pk_key_prefix(&key);
            if txn_pk != *pk {
                break;
            }

            let height = pk_txn_sort_key_sort(&key);
            let nonce = pk_txn_sort_key_nonce(&key);
            let txn_hash = txn_hash_of_key(&key);

            txns.push((height, nonce, txn_hash));
        }

        // sort highest to lowest block height & nonce
        txns.sort();
        txns.reverse();

        // write txn records to csv
        let path = match (path, std::env::var("VOLUMES_DIR")) {
            (Some(path), _) => path,
            (_, Ok(dir)) => format!("{dir}/mina-indexer-user-commands/{pk}.csv").into(),
            (_, _) => format!("/mnt/mina-indexer-user-commands/{pk}.csv").into(),
        };
        let mut csv_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_path(&path)?;

        for (_, _, txn_hash) in txns {
            if let Some(cmd) = self.get_user_command(&txn_hash, 0)?.as_ref() {
                csv_writer.serialize(TxnCsvRecord::from_user_command(cmd))?;
            } else {
                bail!("User command missing: {txn_hash}")
            }
        }

        csv_writer.flush()?;
        Ok(path)
    }

    ///////////////
    // Iterators //
    ///////////////

    /// Key-value pairs
    /// ```
    /// - key: {height}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [txn_sort_key]
    fn user_commands_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.user_commands_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {slot}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - slot:       [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [txn_sort_key]
    fn user_commands_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.user_commands_slot_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {token}{height}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [token_txn_sort_key]
    fn user_commands_per_token_height_iterator(
        &self,
        token: &TokenAddress,
        direction: Direction,
    ) -> DBIterator<'_> {
        let mut start = [0u8; TokenAddress::LEN + U32_LEN + 1];
        start[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());

        if let Direction::Reverse = direction {
            // need to go beyond all possible keys with this token prefix
            start[TokenAddress::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[TokenAddress::LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        };

        self.database.iterator_cf(
            self.user_commands_per_token_height_sort_cf(),
            IteratorMode::From(&start, direction),
        )
    }

    /// Key-value pairs
    /// ```
    /// - key: {token}{slot}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - slot:       [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [token_txn_sort_key]
    fn user_commands_per_token_slot_iterator(
        &self,
        token: &TokenAddress,
        direction: Direction,
    ) -> DBIterator<'_> {
        let mut start = [0u8; TokenAddress::LEN + U32_LEN + 1];
        start[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());

        if let Direction::Reverse = direction {
            // need to go beyond all possible keys with this token prefix
            start[TokenAddress::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[TokenAddress::LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        };

        self.database.iterator_cf(
            self.user_commands_per_token_slot_sort_cf(),
            IteratorMode::From(&start, direction),
        )
    }

    /// Key-value pairs
    /// ```
    /// - key: {sender}{height}{nonce}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:     [PublicKey] bytes
    /// - height:     [u32] BE bytes
    /// - nonce:      [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// - amount:     [u64] BE bytes
    fn txn_from_height_iterator(&self, pk: &PublicKey, direction: Direction) -> DBIterator<'_> {
        // set start key
        let mut start = [0u8; PublicKey::LEN + U32_LEN + U32_LEN + 1];
        start[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());

        // get upper bound if reverse
        if let Direction::Reverse = direction {
            start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        }

        let mode = IteratorMode::From(&start, direction);
        self.database
            .iterator_cf(self.txn_from_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {sender}{slot}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:     [PublicKey] bytes
    /// - slot:       [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// - amount:     [u64] BE bytes
    fn txn_from_slot_iterator(&self, pk: &PublicKey, direction: Direction) -> DBIterator<'_> {
        // set start key
        let mut start = [0u8; PublicKey::LEN + U32_LEN + U32_LEN + 1];
        start[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());

        // get upper bound if reverse
        if let Direction::Reverse = direction {
            start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        }

        let mode = IteratorMode::From(&start, direction);
        self.database
            .iterator_cf(self.txn_from_slot_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{height}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - receiver:   [PublicKey] bytes
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// - amount:     [u64] BE bytes
    fn txn_to_height_iterator(&self, pk: &PublicKey, direction: Direction) -> DBIterator<'_> {
        // set start key
        let mut start = [0u8; PublicKey::LEN + U32_LEN + U32_LEN + 1];
        start[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());

        // get upper bound if reverse
        if let Direction::Reverse = direction {
            start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        }

        let mode = IteratorMode::From(&start, direction);
        self.database
            .iterator_cf(self.txn_to_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{slot}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - receiver:   [PublicKey] bytes
    /// - slot:       [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// - amount:     [u64] BE bytes
    fn txn_to_slot_iterator(&self, pk: &PublicKey, direction: Direction) -> DBIterator<'_> {
        // set start key
        let mut start = [0u8; PublicKey::LEN + U32_LEN + U32_LEN + 1];
        start[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());

        // get upper bound if reverse
        if let Direction::Reverse = direction {
            start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
            start[PublicKey::LEN..][U32_LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
        }

        let mode = IteratorMode::From(&start, direction);
        self.database.iterator_cf(self.txn_to_slot_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {height}{txn_hash}{state_hash}
    /// - val: b""
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes (right-padded)
    /// - state_hash: [StateHash] bytes
    fn zkapp_commands_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.zkapp_commands_height_sort_cf(), mode)
    }

    /// Key-value pairs
    /// ```
    /// - key: {slot}{txn_hash}{state_hash}
    /// - val: b""
    /// where
    /// - slot:       [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes (right-padded)
    /// - state_hash: [StateHash] bytes
    fn zkapp_commands_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.zkapp_commands_slot_sort_cf(), mode)
    }

    /////////////////////////
    // User command counts //
    /////////////////////////

    fn get_pk_num_user_commands_blocks(&self, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting number of user commands for {pk}");
        Ok(self
            .database
            .get_cf(self.user_commands_pk_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_user_commands_epoch_count(
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
            "Getting user command count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.user_commands_epoch_cf(),
                epoch_key(genesis_state_hash, epoch),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_zkapp_commands_epoch_count(
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
            "Getting zkapp command count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_commands_epoch_cf(),
                epoch_key(genesis_state_hash, epoch),
            )?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_epoch_count(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing user command count epoch {} genesis {}",
            epoch,
            genesis_state_hash,
        );

        let old = self.get_user_commands_epoch_count(Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.user_commands_epoch_cf(),
            epoch_key(genesis_state_hash, epoch),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn increment_zkapp_commands_epoch_count(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!("Incrementing zkapp command count epoch {epoch}");

        let old = self.get_zkapp_commands_epoch_count(Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.zkapp_commands_epoch_cf(),
            epoch_key(genesis_state_hash, epoch),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_user_commands_total_count(&self) -> Result<u32> {
        trace!("Getting user command total");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_zkapp_commands_total_count(&self) -> Result<u32> {
        trace!("Getting zkapp command total");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_total_count(&self) -> Result<()> {
        trace!("Incrementing user command total");

        let old = self.get_user_commands_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_USER_COMMANDS_KEY, (old + 1).to_be_bytes())?)
    }

    fn increment_zkapp_commands_total_count(&self) -> Result<()> {
        trace!("Incrementing zkapp command total");

        let old = self.get_zkapp_commands_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_ZKAPP_COMMANDS_KEY, (old + 1).to_be_bytes())?)
    }

    fn get_user_commands_pk_epoch_count(
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
            "Getting user command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.user_commands_pk_epoch_cf(),
                epoch_pk_key(genesis_state_hash, epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_zkapp_commands_pk_epoch_count(
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
            "Getting zkapp command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_commands_pk_epoch_cf(),
                epoch_pk_key(genesis_state_hash, epoch, pk),
            )?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing user command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        let old =
            self.get_user_commands_pk_epoch_count(pk, Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.user_commands_pk_epoch_cf(),
            epoch_pk_key(genesis_state_hash, epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn increment_zkapp_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        trace!(
            "Incrementing zkapp command count pk {} epoch {} genesis {}",
            pk,
            epoch,
            genesis_state_hash,
        );

        let old =
            self.get_zkapp_commands_pk_epoch_count(pk, Some(epoch), Some(genesis_state_hash))?;
        Ok(self.database.put_cf(
            self.zkapp_commands_pk_epoch_cf(),
            epoch_pk_key(genesis_state_hash, epoch, pk),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn get_user_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32> {
        trace!("Getting pk total user commands count {pk}");

        Ok(self
            .database
            .get_cf(self.user_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn get_zkapp_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32> {
        trace!("Getting pk total zkapp commands count {pk}");

        Ok(self
            .database
            .get_cf(self.zkapp_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()> {
        trace!("Incrementing user command pk total num {pk}");

        let old = self.get_user_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.user_commands_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn increment_zkapp_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()> {
        trace!("Incrementing zkapp command pk total num {pk}");

        let old = self.get_zkapp_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.zkapp_commands_pk_total_cf(),
            pk.0.as_bytes(),
            (old + 1).to_be_bytes(),
        )?)
    }

    fn set_block_user_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Setting block user command count {state_hash} -> {count}");

        batch.put_cf(
            self.block_user_command_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        );

        Ok(())
    }

    fn set_block_zkapp_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Setting block zkapp command count {state_hash} -> {count}");

        batch.put_cf(
            self.block_zkapp_command_counts_cf(),
            state_hash.0.as_bytes(),
            count.to_be_bytes(),
        );

        Ok(())
    }

    fn get_block_user_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>> {
        trace!("Getting block user command count {state_hash}");

        Ok(self
            .database
            .get_cf(self.block_user_command_counts_cf(), state_hash.0.as_bytes())?
            .map(|bytes| from_be_bytes(bytes.to_vec())))
    }

    fn get_block_zkapp_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>> {
        trace!("Getting block zkapp command count {state_hash}");

        Ok(self
            .database
            .get_cf(
                self.block_zkapp_command_counts_cf(),
                state_hash.0.as_bytes(),
            )?
            .map(|bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_user_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        if command.is_applied() {
            self.increment_applied_user_commands_count(1)?;
        } else {
            self.increment_failed_user_commands_count(1)?;
        }

        // sender epoch & total
        let sender = command.sender();
        self.increment_user_commands_pk_epoch_count(&sender, epoch, genesis_state_hash)?;
        self.increment_user_commands_pk_total_count(&sender)?;

        // receiver epoch & total
        for receiver in command.receiver() {
            if sender != receiver {
                self.increment_user_commands_pk_epoch_count(&receiver, epoch, genesis_state_hash)?;
                self.increment_user_commands_pk_total_count(&receiver)?;
            }
        }

        // epoch & total counts
        self.increment_user_commands_epoch_count(epoch, genesis_state_hash)?;
        self.increment_user_commands_total_count()
    }

    fn increment_zkapp_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()> {
        if command.is_applied() {
            self.increment_applied_zkapp_commands_count(1)?;
        } else {
            self.increment_failed_zkapp_commands_count(1)?;
        }

        // sender epoch & total
        let sender = command.sender();
        self.increment_zkapp_commands_pk_epoch_count(&sender, epoch, genesis_state_hash)?;
        self.increment_zkapp_commands_pk_total_count(&sender)?;

        // receiver epoch & total
        for receiver in command.receiver() {
            if sender != receiver {
                self.increment_zkapp_commands_pk_epoch_count(&receiver, epoch, genesis_state_hash)?;
                self.increment_zkapp_commands_pk_total_count(&receiver)?;
            }
        }

        // epoch & total counts
        self.increment_zkapp_commands_epoch_count(epoch, genesis_state_hash)?;
        self.increment_zkapp_commands_total_count()
    }

    fn get_applied_user_commands_count(&self) -> Result<u32> {
        trace!("Getting applied user command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_APPLIED_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_applied_zkapp_commands_count(&self) -> Result<u32> {
        trace!("Getting applied zkapp command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_APPLIED_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_failed_user_commands_count(&self) -> Result<u32> {
        trace!("Getting failed user command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_FAILED_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_failed_zkapp_commands_count(&self) -> Result<u32> {
        trace!("Getting failed zkapp command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_FAILED_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_applied_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing applied user command count");

        let old = self.get_applied_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_USER_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_applied_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing applied zkapp command count");

        let old = self.get_applied_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_ZKAPP_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_failed_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing failed user command count");

        let old = self.get_failed_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_USER_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_failed_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing failed zkapp command count");

        let old = self.get_failed_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_ZKAPP_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn decrement_applied_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing applied user command count");

        let old = self.get_applied_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_USER_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_applied_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing applied zkapp command count");

        let old = self.get_applied_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_ZKAPP_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_failed_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing failed user command count");

        let old = self.get_failed_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_USER_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_failed_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing failed zkapp command count");

        let old = self.get_failed_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_ZKAPP_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    // canonical commands

    fn get_canonical_user_commands_count(&self) -> Result<u32> {
        trace!("Getting canonical user command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_CANONICAL_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_canonical_zkapp_commands_count(&self) -> Result<u32> {
        trace!("Getting canonical zkapp command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_CANONICAL_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing canonical user command count");

        let old = self.get_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_USER_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing canonical zkapp command count");

        let old = self.get_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn decrement_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing canonical user command count");

        let old = self.get_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_USER_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing canonical zkapp command count");

        let old = self.get_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    // applied canonical commands

    fn get_applied_canonical_user_commands_count(&self) -> Result<u32> {
        trace!("Getting applied canonical user command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_APPLIED_CANONICAL_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_applied_canonical_zkapp_commands_count(&self) -> Result<u32> {
        trace!("Getting applied canonical zkapp command count");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_APPLIED_CANONICAL_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_applied_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing applied canonical user command count");

        let old = self.get_applied_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_CANONICAL_USER_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_applied_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing applied canonical zkapp command count");
        let old = self.get_applied_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn decrement_applied_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing applied canonical user command count");
        let old = self.get_applied_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_CANONICAL_USER_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_applied_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing applied canonical zkapp command count");
        let old = self.get_applied_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_APPLIED_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    // failed canonical commands

    fn get_failed_canonical_user_commands_count(&self) -> Result<u32> {
        trace!("Getting failed canonical user command count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_FAILED_CANONICAL_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_failed_canonical_zkapp_commands_count(&self) -> Result<u32> {
        trace!("Getting failed canonical zkapp command count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_FAILED_CANONICAL_ZKAPP_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_failed_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing failed canonical user command count");
        let old = self.get_failed_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_CANONICAL_USER_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn increment_failed_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Incrementing failed canonical zkapp command count");
        let old = self.get_failed_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old + incr).to_be_bytes(),
        )?)
    }

    fn decrement_failed_canonical_user_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing failed canonical user command count");
        let old = self.get_failed_canonical_user_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_CANONICAL_USER_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn decrement_failed_canonical_zkapp_commands_count(&self, incr: u32) -> Result<()> {
        trace!("Decrementing failed canonical zkapp command count");
        let old = self.get_failed_canonical_zkapp_commands_count()?;
        Ok(self.database.put(
            Self::TOTAL_NUM_FAILED_CANONICAL_ZKAPP_COMMANDS_KEY,
            (old.saturating_sub(incr)).to_be_bytes(),
        )?)
    }

    fn update_user_commands(&self, block: &DbBlockUpdate) -> Result<()> {
        for update in block.unapply.iter() {
            if let Some(user_commands) = self
                .get_block_user_commands(&update.state_hash)
                .ok()
                .flatten()
            {
                // decrement token transaction counts
                for command in &user_commands {
                    for token in command.tokens() {
                        self.decrement_token_txns_num(&token)?;
                    }
                }

                let (applied_uc, failed_uc): (Vec<_>, Vec<_>) =
                    user_commands.iter().partition(|uc| uc.is_applied());

                self.decrement_canonical_user_commands_count(user_commands.len() as u32)?;
                self.decrement_applied_canonical_user_commands_count(applied_uc.len() as u32)?;
                self.decrement_failed_canonical_user_commands_count(failed_uc.len() as u32)?;
            }
        }

        for update in block.apply.iter() {
            if let Some(user_commands) = self
                .get_block_user_commands(&update.state_hash)
                .ok()
                .flatten()
            {
                // increment token transaction counts
                for command in &user_commands {
                    for token in command.tokens() {
                        self.increment_token_txns_num(&token)?;
                    }
                }

                let (applied_uc, failed_uc): (Vec<_>, Vec<_>) =
                    user_commands.iter().partition(|uc| uc.is_applied());

                self.increment_canonical_user_commands_count(user_commands.len() as u32)?;
                self.increment_applied_canonical_user_commands_count(applied_uc.len() as u32)?;
                self.increment_failed_canonical_user_commands_count(failed_uc.len() as u32)?;

                let zkapp_commands: Vec<_> = user_commands
                    .iter()
                    .filter(|uc| uc.is_zkapp_command())
                    .collect();
                let (applied_zkapp, failed_zkapp): (
                    Vec<&UserCommandWithStatus>,
                    Vec<&UserCommandWithStatus>,
                ) = zkapp_commands.iter().partition(|uc| uc.is_applied());

                self.increment_canonical_zkapp_commands_count(zkapp_commands.len() as u32)?;
                self.increment_applied_canonical_zkapp_commands_count(applied_zkapp.len() as u32)?;
                self.increment_failed_canonical_zkapp_commands_count(failed_zkapp.len() as u32)?;
            }
        }

        Ok(())
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
            to: cmd
                .command
                .receiver_pk()
                .first()
                .expect("receiver")
                .0
                .to_owned(),
            nonce: cmd.nonce.0,
            hash: cmd.txn_hash.ref_inner(),
            fee: cmd.command.fee(),
            amount: cmd.command.amount(),
            memo: cmd.command.memo(),
            kind: cmd.command.kind().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::precomputed::PcbVersion;
    use std::{env, path::Path};
    use tempfile::TempDir;

    fn create_indexer_store() -> Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(env::current_dir()?)?;
        IndexerStore::new(temp_dir.path(), true)
    }

    #[test]
    fn increment_non_zkapp_commands_counts() -> Result<()> {
        let store = create_indexer_store()?;

        let path = Path::new("./tests/data/misc_blocks/mainnet-278424-3NLbUZF8568pK56NJuSpCkfLTQTKpoiNiruju1Hpr6qpoAbuN9Yr.json");
        let pcb = PrecomputedBlock::parse_file(path, PcbVersion::V1)?;

        for (num, cmd) in pcb.zkapp_commands().iter().enumerate() {
            let epoch = pcb.epoch_count();
            let genesis_state_hash = pcb.genesis_state_hash();

            store.increment_user_commands_counts(cmd, epoch, &genesis_state_hash)?;

            assert_eq!(store.get_applied_user_commands_count()?, num as u32 + 1);
            assert_eq!(store.get_failed_user_commands_count()?, 0);
        }

        Ok(())
    }

    #[test]
    fn increment_zkapp_commands_counts() -> Result<()> {
        let store = create_indexer_store()?;

        let path = Path::new("./tests/data/hardfork/mainnet-359617-3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW.json");
        let pcb = PrecomputedBlock::parse_file(path, PcbVersion::V2)?;

        for (num, zkapp_cmd) in pcb.zkapp_commands().iter().enumerate() {
            let epoch = pcb.epoch_count();
            let genesis_state_hash = pcb.genesis_state_hash();

            store.increment_zkapp_commands_counts(zkapp_cmd, epoch, &genesis_state_hash)?;

            assert_eq!(store.get_applied_zkapp_commands_count()?, num as u32 + 1);
            assert_eq!(store.get_failed_zkapp_commands_count()?, 0);
        }

        Ok(())
    }

    #[test]
    fn test_incr_dec_applied_user_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Increment applied user commands count
        indexer.increment_applied_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_user_commands_count()?, 1);

        // Increment again
        indexer.increment_applied_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_user_commands_count()?, 2);

        // Decrement applied user commands count
        indexer.decrement_applied_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_user_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_applied_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_user_commands_count()?, 0);

        // Ensure it does not go below 0
        indexer.decrement_applied_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_user_commands_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_incr_dec_failed_user_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Increment failed user commands count
        indexer.increment_failed_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_user_commands_count()?, 1);

        // Increment again
        indexer.increment_failed_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_user_commands_count()?, 2);

        // Decrement failed user commands count
        indexer.decrement_failed_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_user_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_failed_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_user_commands_count()?, 0);

        // Ensure it does not go below 0
        indexer.decrement_failed_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_user_commands_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_incr_dec_canonical_user_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Increment canonical user commands count
        indexer.increment_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_canonical_user_commands_count()?, 1);

        // Increment again
        indexer.increment_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_canonical_user_commands_count()?, 2);

        // Decrement canonical user commands count
        indexer.decrement_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_canonical_user_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_canonical_user_commands_count()?, 0);

        // Ensure it does not go below 0
        indexer.decrement_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_canonical_user_commands_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_incr_dec_applied_canonical_user_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Increment applied canonical user commands count
        indexer.increment_applied_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_canonical_user_commands_count()?, 1);

        // Increment again
        indexer.increment_applied_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_canonical_user_commands_count()?, 2);

        // Decrement applied canonical user commands count
        indexer.decrement_applied_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_canonical_user_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_applied_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_canonical_user_commands_count()?, 0);

        // Ensure it does not go below 0
        indexer.decrement_applied_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_applied_canonical_user_commands_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_incr_dec_failed_canonical_user_commands_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        // Increment failed canonical user commands count
        indexer.increment_failed_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_canonical_user_commands_count()?, 1);

        // Increment again
        indexer.increment_failed_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_canonical_user_commands_count()?, 2);

        // Decrement failed canonical user commands count
        indexer.decrement_failed_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_canonical_user_commands_count()?, 1);

        // Decrement to 0
        indexer.decrement_failed_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_canonical_user_commands_count()?, 0);

        // Ensure it does not go below 0
        indexer.decrement_failed_canonical_user_commands_count(1)?;
        assert_eq!(indexer.get_failed_canonical_user_commands_count()?, 0);

        Ok(())
    }
}
