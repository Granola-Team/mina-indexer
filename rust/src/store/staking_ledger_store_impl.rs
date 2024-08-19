use super::{
    balance_key_prefix, column_families::ColumnFamilyHelpers, from_u64_be_bytes, pk_key_prefix,
};
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
    store::{from_be_bytes, to_be_bytes, IndexerStore},
};
use anyhow::{bail, Context};
use log::{error, trace};
use speedb::{DBIterator, Direction, IteratorMode};
use std::{collections::HashMap, mem::size_of};

impl StakingLedgerStore for IndexerStore {
    fn get_staking_account(
        &self,
        pk: PublicKey,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingAccount>> {
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash.clone())?
        {
            let genesis_state_hash = genesis_state_hash
                .clone()
                .or(self.get_best_block_genesis_hash()?)
                .unwrap();
            let key = staking_ledger_account_key(
                genesis_state_hash,
                epoch,
                ledger_hash.clone(),
                pk.clone(),
            );
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
        pk: PublicKey,
        epoch: u32,
        ledger_hash: LedgerHash,
        genesis_state_hash: BlockHash,
        staking_account_with_delegation: StakingAccountWithEpochDelegation,
    ) -> anyhow::Result<()> {
        trace!("Setting staking account {pk}");
        self.database.put_cf(
            self.staking_ledger_accounts_cf(),
            staking_ledger_account_key(
                genesis_state_hash.clone(),
                epoch,
                ledger_hash.clone(),
                pk.clone(),
            ),
            serde_json::to_vec(&staking_account_with_delegation.account)?,
        )?;
        self.database.put_cf(
            self.staking_delegations_cf(),
            staking_ledger_account_key(
                genesis_state_hash.clone(),
                epoch,
                ledger_hash.clone(),
                pk.clone(),
            ),
            serde_json::to_vec(&staking_account_with_delegation.delegation)?,
        )?;

        // add balance/stake-sort data
        self.database.put_cf(
            self.staking_ledger_balance_sort_cf(),
            staking_ledger_sort_key(
                epoch,
                staking_account_with_delegation.account.balance,
                &pk.0,
            ),
            serde_json::to_vec(&staking_account_with_delegation)?,
        )?;
        self.database.put_cf(
            self.staking_ledger_stake_sort_cf(),
            staking_ledger_sort_key(
                epoch,
                staking_account_with_delegation
                    .delegation
                    .total_delegated
                    .unwrap_or_default(),
                &pk.0,
            ),
            serde_json::to_vec(&staking_account_with_delegation)?,
        )?;
        Ok(())
    }

    fn get_staking_ledger(
        &self,
        ledger_hash: &LedgerHash,
        epoch: Option<u32>,
        genesis_state_hash: &Option<BlockHash>,
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
                        error!("Invalid ledger hash for epoch")
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
            genesis_state_hash.clone(),
            staking_ledger.epoch,
            &staking_ledger.ledger_hash,
        );
        let is_new = self
            .database
            .get_cf(self.staking_ledger_persisted_cf(), key.clone())?
            .is_none();

        // persist new staking ledger
        if is_new {
            self.database
                .put_cf(self.staking_ledger_persisted_cf(), key, b"")?;
        }

        // additional indices
        let ledger_hash = staking_ledger.ledger_hash.clone();
        self.set_staking_ledger_hash_epoch_pair(
            &ledger_hash,
            epoch,
            Some(genesis_state_hash.clone()),
        )?;
        self.set_staking_ledger_hash_genesis_pair(&ledger_hash, genesis_state_hash)?;
        self.set_total_currency(&ledger_hash, staking_ledger.total_currency)?;

        // add staking ledger count at epoch
        let count = staking_ledger.staking_ledger.len();
        self.set_staking_ledger_accounts_count_epoch(
            epoch,
            genesis_state_hash.clone(),
            count as u32,
        )?;

        // add staking ledger accounts & per epoch balance-sorted data
        let aggregated_delegations = staking_ledger.aggregate_delegations()?;
        for (pk, account) in staking_ledger.staking_ledger {
            let delegation = aggregated_delegations
                .delegations
                .get(&pk)
                .cloned()
                .expect("delegation exists");
            self.set_staking_account(
                pk,
                epoch,
                ledger_hash.clone(),
                genesis_state_hash.clone(),
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
        pk: PublicKey,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<EpochStakeDelegation>> {
        trace!("Getting epoch {epoch} aggregated delegations for {pk}");
        let ledger_hash = self
            .get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash.clone())?
            .expect("staking ledger hash");
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or_else(|| self.get_best_block_genesis_hash().ok().flatten().unwrap());
        if let Some(bytes) = self.database.get_cf(
            self.staking_delegations_cf(),
            staking_ledger_account_key(genesis_state_hash, epoch, ledger_hash, pk.clone()),
        )? {
            return Ok(Some(serde_json::from_slice(&bytes)?));
        }
        Ok(None)
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
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<LedgerHash>> {
        trace!("Getting staking ledger hash for epoch {epoch}");
        let genesis_state_hash = genesis_state_hash
            .or(self.get_best_block_genesis_hash()?)
            .unwrap();
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_epoch_to_hash_cf(),
                staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            )?
            .and_then(|bytes| LedgerHash::from_bytes(bytes).ok()))
    }

    fn set_staking_ledger_hash_epoch_pair(
        &self,
        ledger_hash: &LedgerHash,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<()> {
        trace!("Setting epoch {epoch} for staking ledger {ledger_hash}");
        let genesis_state_hash = genesis_state_hash
            .or(self.get_best_block_genesis_hash()?)
            .unwrap();
        self.database.put_cf(
            self.staking_ledger_epoch_to_hash_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash.clone(), epoch),
            ledger_hash.0.as_bytes(),
        )?;
        Ok(self.database.put_cf(
            self.staking_ledger_hash_to_epoch_cf(),
            ledger_hash.0.as_bytes(),
            to_be_bytes(epoch),
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
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
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
            .map(from_u64_be_bytes))
    }

    fn get_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
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
        genesis_state_hash: BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting staking ledger accounts count for epoch {epoch} {genesis_state_hash:?}: {count}");
        Ok(self.database.put_cf(
            self.staking_ledger_accounts_count_epoch_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            to_be_bytes(count),
        )?)
    }

    fn build_staking_ledger(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Building staking ledger epoch {epoch}");
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash.clone())?
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
                    if let Some((key_epoch, balance, pk)) = split_staking_ledger_sort_key(&key) {
                        if key_epoch != epoch {
                            // no longer the ledger of interest
                            break;
                        }
                        let account = self
                            .get_staking_account(pk.clone(), epoch, genesis_state_hash)?
                            .with_context(|| format!("epoch {epoch}, account {pk}"))
                            .expect("staking account exists");

                        assert_eq!(account.balance, balance);
                        staking_ledger.insert(pk, account);
                    } else {
                        bail!("Invalid staking ledger account balance sort key");
                    }
                }
                return Ok(Some(StakingLedger {
                    epoch,
                    network,
                    ledger_hash,
                    total_currency,
                    staking_ledger,
                    genesis_state_hash: genesis_hash,
                }));
            }
        }
        Ok(None)
    }

    fn build_aggregated_delegations(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>> {
        trace!("Building epoch {epoch} aggregated delegations");
        if let Some(ledger_hash) =
            self.get_staking_ledger_hash_by_epoch(epoch, genesis_state_hash.clone())?
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
                    if let Some((key_epoch, stake, pk)) = split_staking_ledger_sort_key(&key) {
                        if key_epoch != epoch {
                            // no longer the staking ledger of interest
                            break;
                        }
                        let account = self
                            .get_epoch_delegations(
                                pk.clone(),
                                epoch,
                                Some(genesis_state_hash.clone()),
                            )?
                            .with_context(|| format!("epoch {epoch}, account {pk}"))
                            .expect("staking account exists");
                        if let Some(total_delegated) = account.total_delegated {
                            assert_eq!(stake, total_delegated);
                            total_delegations += total_delegated;
                        }
                        delegations.insert(pk, account.clone());
                    } else {
                        panic!("Invalid staking ledger account balance sort key");
                    }
                }
                return Ok(Some(AggregatedEpochStakeDelegations {
                    epoch,
                    network,
                    ledger_hash,
                    genesis_state_hash,
                    delegations,
                    total_delegations,
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
        let fstart = staking_ledger_sort_key(epoch, 0, "");
        let rstart = staking_ledger_sort_key(epoch, u64::MAX, "C");
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
        let fstart = staking_ledger_sort_key(epoch, 0, "");
        let rstart = staking_ledger_sort_key(epoch, u64::MAX, "C");
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

/// Split [staking_ledger_epoch_key] into constituent parts
pub fn split_staking_ledger_epoch_key(key: &[u8]) -> Option<(BlockHash, u32, LedgerHash)> {
    if key.len() == BlockHash::LEN + size_of::<u32>() + LedgerHash::LEN {
        let genesis_state_hash =
            BlockHash::from_bytes(&key[..BlockHash::LEN]).expect("genesis state hash key prefix");
        let epoch = from_be_bytes(key[BlockHash::LEN..][..size_of::<u32>()].to_vec());
        let ledger_hash = LedgerHash::from_bytes(key[BlockHash::LEN + size_of::<u32>()..].to_vec())
            .expect("ledger hash key suffix");
        return Some((genesis_state_hash, epoch, ledger_hash));
    }
    None
}

/// Split [staking_ledger_sort_key] into constituent parts
pub fn split_staking_ledger_sort_key(key: &[u8]) -> Option<(u32, u64, PublicKey)> {
    if key.len() == size_of::<u32>() + size_of::<u64>() + PublicKey::LEN {
        let epoch = from_be_bytes(key[..size_of::<u32>()].to_vec());
        let balance_or_stake = balance_key_prefix(&key[size_of::<u32>()..]);
        let pk = pk_key_prefix(&key[size_of::<u32>() + size_of::<u64>()..]);
        return Some((epoch, balance_or_stake, pk));
    }
    None
}

/// Staking ledger amount sort key
/// ```
/// {epoch}{amount}{suffix}
/// where
/// - epoch:  4 BE bytes
/// - amount: 8 BE bytes
/// - suffix: [PublicKey::LEN] bytes
pub fn staking_ledger_sort_key(epoch: u32, amount: u64, suffix: &str) -> Vec<u8> {
    let mut key = to_be_bytes(epoch);
    key.append(&mut amount.to_be_bytes().to_vec());
    key.append(&mut suffix.as_bytes().to_vec());
    key
}

/// Staking ledger account key
/// ```
/// {genesis_hash}{epoch}{ledger_hash}{pk}
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        4 BE bytes
/// - ledger_hash:  [LedgerHash::LEN] bytes
/// - pk:           [PublicKey::LEN] bytes
fn staking_ledger_account_key(
    genesis_state_hash: BlockHash,
    epoch: u32,
    ledger_hash: LedgerHash,
    pk: PublicKey,
) -> Vec<u8> {
    let mut key = staking_ledger_epoch_key_prefix(genesis_state_hash, epoch);
    key.append(&mut ledger_hash.0.into_bytes());
    key.append(&mut pk.to_bytes());
    key
}

/// Staking ledger epoch key
/// ```
/// {genesis_hash}{epoch}{ledger_hash}
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        4 BE bytes
/// - ledger_hash:  [LedgerHash::LEN] bytes
fn staking_ledger_epoch_key(
    genesis_state_hash: BlockHash,
    epoch: u32,
    ledger_hash: &LedgerHash,
) -> Vec<u8> {
    let mut key = staking_ledger_epoch_key_prefix(genesis_state_hash, epoch);
    key.append(&mut ledger_hash.0.clone().into_bytes());
    key
}

/// Prefix of [staking_ledger_epoch_key]
/// ```
/// - key: {genesis_hash}{epoch}
/// - val: aggregated epoch delegations serde bytes
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        4 BE bytes
fn staking_ledger_epoch_key_prefix(genesis_state_hash: BlockHash, epoch: u32) -> Vec<u8> {
    let mut key = genesis_state_hash.to_bytes();
    key.append(&mut to_be_bytes(epoch));
    key
}
