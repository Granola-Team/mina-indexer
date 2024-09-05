use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    command::internal::{store::InternalCommandStore, InternalCommand, InternalCommandWithData},
    ledger::public_key::PublicKey,
    store::{from_be_bytes, to_be_bytes, u32_prefix_key, IndexerStore},
};
use log::trace;
use speedb::DBIterator;

impl InternalCommandStore for IndexerStore {
    /// Index internal commands on public keys & state hash
    fn add_internal_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let epoch = block.epoch_count();
        trace!("Adding internal commands for block {}", block.summary());

        // add internal cmds to state hash
        let key = format!("internal-{}", block.state_hash().0);
        let internal_cmds = InternalCommand::from_precomputed(block);
        self.database.put_cf(
            self.internal_commands_cf(),
            key.as_bytes(),
            serde_json::to_vec(&internal_cmds)?,
        )?;

        // per block internal command count
        self.set_block_internal_commands_count(&block.state_hash(), internal_cmds.len() as u32)?;

        // add cmds with data to public keys
        let internal_cmds_with_data: Vec<InternalCommandWithData> = internal_cmds
            .clone()
            .into_iter()
            .map(|c| InternalCommandWithData::from_internal_cmd(c, block))
            .collect();

        // increment internal command counts
        for internal_cmd in &internal_cmds_with_data {
            self.increment_internal_commands_counts(internal_cmd, epoch)?;
        }

        fn internal_commmand_key(global_slot: u32, state_hash: &str, index: usize) -> Vec<u8> {
            let mut bytes = to_be_bytes(global_slot).to_vec();
            bytes.append(&mut state_hash.as_bytes().to_vec());
            bytes.append(&mut index.to_be_bytes().to_vec());
            bytes
        }

        for (i, int_cmd) in internal_cmds_with_data.iter().enumerate() {
            let key =
                internal_commmand_key(block.global_slot_since_genesis(), &block.state_hash().0, i);
            self.database.put_cf(
                self.internal_commands_slot_cf(),
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
                self.internal_commands_cf(),
                key.as_bytes(),
                serde_json::to_vec(&pk_internal_cmds_with_data)?,
            )?;

            // update pk's number of internal cmds
            let key = format!("internal-{}", pk.0);
            let next_n = (n + 1).to_string();
            self.database.put_cf(
                self.internal_commands_cf(),
                key.as_bytes(),
                next_n.as_bytes(),
            )?;
        }
        Ok(())
    }

    fn get_internal_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<InternalCommandWithData>> {
        trace!("Getting internal commands in block {}", state_hash.0);
        let block = self.get_block(state_hash)?.expect("block to exist").0;

        let key = format!("internal-{}", state_hash.0);
        if let Some(commands_bytes) = self
            .database
            .get_pinned_cf(self.internal_commands_cf(), key.as_bytes())?
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

        let commands_cf = self.internal_commands_cf();
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
        trace!("Getting pk num internal commands {pk}");
        let key = format!("internal-{}", pk);
        Ok(self
            .database
            .get_pinned_cf(self.internal_commands_cf(), key.as_bytes())?
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }))
    }

    fn internal_commands_global_slot_interator(
        &self,
        mode: speedb::IteratorMode,
    ) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.internal_commands_slot_cf(), mode)
    }

    fn get_internal_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting internal command epoch {epoch}");
        Ok(self
            .database
            .get_pinned_cf(self.internal_commands_epoch_cf(), to_be_bytes(epoch))?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_internal_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing internal command epoch {epoch}");
        let old = self.get_internal_commands_epoch_count(Some(epoch))?;
        Ok(self.database.put_cf(
            self.internal_commands_epoch_cf(),
            to_be_bytes(epoch),
            to_be_bytes(old + 1),
        )?)
    }

    fn get_internal_commands_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting internal command total");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_FEE_TRANSFERS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_internal_commands_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing internal command total");

        let old = self.get_internal_commands_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_FEE_TRANSFERS_KEY, to_be_bytes(old + 1))?)
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
            .get_pinned_cf(
                self.internal_commands_pk_epoch_cf(),
                u32_prefix_key(epoch, pk),
            )?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
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
            to_be_bytes(old + 1),
        )?)
    }

    fn get_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total internal commands count {pk}");
        Ok(self
            .database
            .get_pinned_cf(self.internal_commands_pk_total_cf(), pk.0.as_bytes())?
            .map_or(0, |bytes| from_be_bytes(bytes.to_vec())))
    }

    fn increment_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing internal command pk total num {pk}");

        let old = self.get_internal_commands_pk_total_count(pk)?;
        Ok(self.database.put_cf(
            self.internal_commands_pk_total_cf(),
            pk.0.as_bytes(),
            to_be_bytes(old + 1),
        )?)
    }

    fn get_block_internal_commands_count(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<u32>> {
        trace!("Getting block internal command count");
        Ok(self
            .database
            .get_pinned_cf(
                self.block_internal_command_counts_cf(),
                state_hash.0.as_bytes(),
            )?
            .map(|bytes| from_be_bytes(bytes.to_vec())))
    }

    fn set_block_internal_commands_count(
        &self,
        state_hash: &BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting block internal command count {state_hash} -> {count}");
        Ok(self.database.put_cf(
            self.block_internal_command_counts_cf(),
            state_hash.0.as_bytes(),
            to_be_bytes(count),
        )?)
    }

    fn increment_internal_commands_counts(
        &self,
        internal_command: &InternalCommandWithData,
        epoch: u32,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = match internal_command {
            InternalCommandWithData::Coinbase { .. } => return Ok(()),
            InternalCommandWithData::FeeTransfer {
                sender, receiver, ..
            } => (sender, receiver),
        };
        trace!(
            "Incrementing internal command counts {:?}",
            internal_command
        );

        // sender epoch & total
        self.increment_internal_commands_pk_epoch_count(sender, epoch)?;
        self.increment_internal_commands_pk_total_count(sender)?;

        // receiver epoch & total
        if sender != receiver {
            self.increment_internal_commands_pk_epoch_count(receiver, epoch)?;
            self.increment_internal_commands_pk_total_count(receiver)?;
        }

        // epoch & total counts
        self.increment_internal_commands_epoch_count(epoch)?;
        self.increment_internal_commands_total_count()
    }
}
