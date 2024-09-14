use super::{column_families::ColumnFamilyHelpers, IndexerStore};
use crate::{
    block::{store::BlockStore, BlockHash},
    chain::store::ChainStore,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        public_key::PublicKey,
        staking::{
            AggregatedEpochStakeDelegations, EpochStakeDelegation, StakingAccount, StakingLedger,
        },
        store::staking::{StakingAccountWithEpochDelegation, StakingLedgerStore},
        LedgerHash,
    },
    utility::store::{from_be_bytes, staking_ledger::*, u64_from_be_bytes},
};
use anyhow::Context;
use log::{error, trace};
use speedb::{DBIterator, Direction, IteratorMode};
use std::collections::HashMap;

impl StakingLedgerStore for IndexerStore {
    fn get_staking_account(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<StakingAccount>> {
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash)?
        {
            let best_block_genesis_hash = self.get_best_block_genesis_hash()?;
            let genesis_state_hash = genesis_state_hash
                .or(best_block_genesis_hash.as_ref())
                .unwrap();
            let key = staking_ledger_account_key(genesis_state_hash, epoch, &ledger_hash, pk);
            return Ok(self
                .database
                .get_cf(self.staking_ledger_accounts_cf(), key)?
                .and_then(|bytes| serde_json::from_slice(&bytes).ok()));
        }

        error!("Ledger hash not present for epoch {epoch}");
        Ok(None)
    }

    fn set_staking_account(
        &self,
        pk: &PublicKey,
        epoch: u32,
        ledger_hash: &LedgerHash,
        genesis_state_hash: &BlockHash,
        staking_account_with_delegation: StakingAccountWithEpochDelegation,
    ) -> anyhow::Result<()> {
        trace!("Setting staking account {pk}");

        // add staking account
        self.database.put_cf(
            self.staking_ledger_accounts_cf(),
            staking_ledger_account_key(genesis_state_hash, epoch, ledger_hash, pk),
            serde_json::to_vec(&staking_account_with_delegation.account)?,
        )?;

        // add staking delegations
        self.database.put_cf(
            self.staking_delegations_cf(),
            staking_ledger_account_key(genesis_state_hash, epoch, ledger_hash, pk),
            serde_json::to_vec(&staking_account_with_delegation.delegation)?,
        )?;

        // balance/stake sort
        let account_serde_bytes = serde_json::to_vec(&staking_account_with_delegation)?;
        self.database.put_cf(
            self.staking_ledger_balance_sort_cf(),
            staking_ledger_sort_key(epoch, staking_account_with_delegation.account.balance, pk),
            &account_serde_bytes,
        )?;
        self.database.put_cf(
            self.staking_ledger_stake_sort_cf(),
            staking_ledger_sort_key(
                epoch,
                staking_account_with_delegation
                    .delegation
                    .total_delegated
                    .unwrap_or_default(),
                pk,
            ),
            &account_serde_bytes,
        )?;
        Ok(())
    }

    fn get_staking_ledger(
        &self,
        ledger_hash: &LedgerHash,
        epoch: Option<u32>,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        match epoch {
            None => {
                trace!("Getting staking ledger by hash {ledger_hash}");
                if let Some(epoch) = self.get_epoch(ledger_hash)? {
                    return self.build_staking_ledger(epoch, genesis_state_hash);
                }
                Ok(None)
            }
            Some(epoch) => {
                trace!("Getting staking ledger by epoch {epoch}");
                if let Ok(Some(staking_ledger)) =
                    self.build_staking_ledger(epoch, genesis_state_hash)
                {
                    if staking_ledger.ledger_hash == *ledger_hash {
                        return Ok(Some(staking_ledger));
                    } else {
                        error!("Invalid ledger hash {ledger_hash} for epoch {epoch}")
                    }
                }
                Ok(None)
            }
        }
    }

    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Adding staking ledger {}", staking_ledger.summary());
        let epoch = staking_ledger.epoch;
        let key = staking_ledger_epoch_key(
            genesis_state_hash,
            staking_ledger.epoch,
            &staking_ledger.ledger_hash,
        );
        let is_new = self
            .database
            .get_cf(self.staking_ledger_persisted_cf(), key)?
            .is_none();

        // persist new staking ledger
        if is_new {
            self.database
                .put_cf(self.staking_ledger_persisted_cf(), key, b"")?;
        }

        // additional indices
        let ledger_hash = staking_ledger.ledger_hash.clone();
        self.set_staking_ledger_hash_epoch_pair(&ledger_hash, epoch, Some(genesis_state_hash))?;
        self.set_staking_ledger_hash_genesis_pair(&ledger_hash, genesis_state_hash)?;
        self.set_total_currency(&ledger_hash, staking_ledger.total_currency)?;

        // add staking ledger count at epoch
        let count = staking_ledger.staking_ledger.len();
        self.set_staking_ledger_accounts_count_epoch(epoch, genesis_state_hash, count as u32)?;

        // add staking ledger accounts & per epoch balance-sorted data
        let aggregated_delegations = staking_ledger.aggregate_delegations()?;
        for (pk, account) in staking_ledger.staking_ledger {
            let delegation = aggregated_delegations
                .delegations
                .get(&pk)
                .cloned()
                .expect("delegation exists");
            self.set_staking_account(
                &pk,
                epoch,
                &ledger_hash,
                genesis_state_hash,
                StakingAccountWithEpochDelegation {
                    account,
                    delegation,
                },
            )?;
        }

        if is_new {
            // add new ledger event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::NewStakingLedger {
                    epoch,
                    ledger_hash,
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;

            // add new aggregated delegation event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::AggregateDelegations {
                    epoch,
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;
        }
        Ok(())
    }

    fn get_epoch_delegations(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<EpochStakeDelegation>> {
        trace!("Getting epoch {epoch} aggregated delegations for {pk}");
        let ledger_hash = self
            .get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash)?
            .expect("staking ledger hash");
        let best_block_genesis_hash = self.get_best_block_genesis_hash()?;
        let genesis_state_hash = genesis_state_hash.unwrap_or_else(|| {
            best_block_genesis_hash
                .as_ref()
                .expect("best block genesis hash")
        });
        Ok(self
            .database
            .get_cf(
                self.staking_delegations_cf(),
                staking_ledger_account_key(genesis_state_hash, epoch, &ledger_hash, pk),
            )?
            .map(|bytes| serde_json::from_slice(&bytes).expect("epoch staking delegation bytes")))
    }

    fn get_epoch(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting epoch for staking ledger {ledger_hash}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_hash_to_epoch_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_staking_ledger_hash_by_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<LedgerHash>> {
        trace!("Getting staking ledger hash for epoch {epoch}");
        let best_block_genesis_hash = self.get_best_block_genesis_hash()?;
        let genesis_state_hash = genesis_state_hash
            .or(best_block_genesis_hash.as_ref())
            .unwrap();
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_epoch_to_hash_cf(),
                staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            )?
            .map(LedgerHash::from_bytes_or_panic))
    }

    fn set_staking_ledger_hash_epoch_pair(
        &self,
        ledger_hash: &LedgerHash,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<()> {
        trace!("Setting epoch {epoch} for staking ledger {ledger_hash}");
        let best_block_genesis_hash = self.get_best_block_genesis_hash()?;
        let genesis_state_hash = genesis_state_hash
            .or(best_block_genesis_hash.as_ref())
            .unwrap();
        self.database.put_cf(
            self.staking_ledger_epoch_to_hash_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            ledger_hash.0.as_bytes(),
        )?;
        Ok(self.database.put_cf(
            self.staking_ledger_hash_to_epoch_cf(),
            ledger_hash.0.as_bytes(),
            epoch.to_be_bytes(),
        )?)
    }

    fn set_staking_ledger_hash_genesis_pair(
        &self,
        ledger_hash: &LedgerHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Setting genesis state hash {genesis_state_hash} for staking ledger {ledger_hash}");
        Ok(self.database.put_cf(
            self.staking_ledger_genesis_hash_cf(),
            ledger_hash.0.as_bytes(),
            genesis_state_hash.0.as_bytes(),
        )?)
    }

    fn get_genesis_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting genesis state hash for staking ledger {ledger_hash}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_genesis_hash_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .map(BlockHash::from_bytes_or_panic))
    }

    fn set_total_currency(
        &self,
        ledger_hash: &LedgerHash,
        total_currency: u64,
    ) -> anyhow::Result<()> {
        trace!("Setting total currency {total_currency} for staking ledger {ledger_hash}");
        Ok(self.database.put_cf(
            self.staking_ledger_total_currency_cf(),
            ledger_hash.0.as_bytes(),
            total_currency.to_be_bytes(),
        )?)
    }

    fn get_total_currency(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u64>> {
        trace!("Getting total currency for staking ledger {ledger_hash}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_total_currency_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .and_then(|bytes| u64_from_be_bytes(&bytes).ok()))
    }

    fn get_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<u32> {
        trace!("Getting staking ledger accounts count for epoch {epoch} {genesis_state_hash:?}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_accounts_count_epoch_cf(),
                staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            )?
            .map_or(0, from_be_bytes))
    }

    fn set_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting staking ledger accounts count for epoch {epoch} {genesis_state_hash:?}: {count}");
        Ok(self.database.put_cf(
            self.staking_ledger_accounts_count_epoch_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            count.to_be_bytes(),
        )?)
    }

    fn build_staking_ledger(
        &self,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Building staking ledger epoch {epoch}");
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash)?
        {
            if let (network, Some(total_currency), Some(genesis_hash)) = (
                self.get_current_network()?,
                self.get_total_currency(&ledger_hash)?,
                self.get_genesis_state_hash(&ledger_hash)?,
            ) {
                trace!("Staking ledger {network} (epoch {epoch}): {ledger_hash}");
                if let Some(genesis_state_hash) = genesis_state_hash {
                    assert_eq!(genesis_hash, *genesis_state_hash);
                }

                let mut staking_ledger = HashMap::new();
                for (key, _) in self
                    .staking_ledger_account_balance_iterator(epoch, Direction::Reverse)
                    .flatten()
                {
                    let (key_epoch, balance, pk) = split_staking_ledger_sort_key(&key)?;
                    if key_epoch != epoch {
                        // no longer the ledger of interest
                        break;
                    }

                    let account = self
                        .get_staking_account(&pk, epoch, genesis_state_hash)?
                        .with_context(|| format!("epoch {epoch}, account {pk}"))
                        .expect("staking account exists");
                    assert_eq!(account.balance, balance);
                    staking_ledger.insert(pk, account);
                }
                return Ok(Some(StakingLedger {
                    epoch,
                    network,
                    ledger_hash,
                    total_currency,
                    staking_ledger,
                    genesis_state_hash: genesis_hash.clone(),
                }));
            }
        }
        Ok(None)
    }

    fn build_aggregated_delegations(
        &self,
        epoch: u32,
        genesis_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>> {
        trace!("Building epoch {epoch} aggregated delegations");
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash)?
        {
            if let (network, Some(genesis_state_hash)) = (
                self.get_current_network()?,
                self.get_genesis_state_hash(&ledger_hash)?,
            ) {
                trace!("Staking ledger {network} (epoch {epoch}): {ledger_hash}");
                let mut delegations = HashMap::new();
                let mut total_delegations = 0;
                for (key, _value) in self
                    .staking_ledger_account_stake_iterator(epoch, Direction::Reverse)
                    .flatten()
                {
                    let (key_epoch, stake, pk) = split_staking_ledger_sort_key(&key)?;
                    if key_epoch != epoch {
                        // no longer the staking ledger of interest
                        break;
                    }

                    let account = self
                        .get_epoch_delegations(&pk, epoch, Some(&genesis_state_hash))?
                        .with_context(|| format!("epoch {epoch}, account {pk}"))?;
                    if let Some(total_delegated) = account.total_delegated {
                        assert_eq!(stake, total_delegated);
                        total_delegations += total_delegated;
                    }
                    delegations.insert(pk, account.clone());
                }
                return Ok(Some(AggregatedEpochStakeDelegations {
                    epoch,
                    network,
                    ledger_hash,
                    delegations,
                    total_delegations,
                    genesis_state_hash: genesis_state_hash.clone(),
                }));
            }
        }
        Ok(None)
    }

    ///////////////
    // Iterators //
    ///////////////

    fn staking_ledger_account_balance_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_> {
        let fstart = staking_ledger_sort_key(epoch, 0, &PublicKey::lower_bound());
        let rstart = staking_ledger_sort_key(epoch, u64::MAX, &PublicKey::upper_bound());
        let mode = match direction {
            Direction::Forward => IteratorMode::From(&fstart, Direction::Forward),
            Direction::Reverse => IteratorMode::From(&rstart, Direction::Reverse),
        };
        self.database
            .iterator_cf(self.staking_ledger_balance_sort_cf(), mode)
    }

    fn staking_ledger_account_stake_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_> {
        let fstart = staking_ledger_sort_key(epoch, 0, &PublicKey::lower_bound());
        let rstart = staking_ledger_sort_key(epoch, u64::MAX, &PublicKey::upper_bound());
        let mode = match direction {
            Direction::Forward => IteratorMode::From(&fstart, Direction::Forward),
            Direction::Reverse => IteratorMode::From(&rstart, Direction::Reverse),
        };
        self.database
            .iterator_cf(self.staking_ledger_stake_sort_cf(), mode)
    }

    fn staking_ledger_epoch_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.staking_ledger_persisted_cf(), mode)
    }
}
