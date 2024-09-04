use super::{column_families::ColumnFamilyHelpers, from_be_bytes, DbUpdate};
use crate::{
    block::{
        store::{BlockStore, DbBlockUpdate},
        BlockHash,
    },
    ledger::{
        account::Account,
        diff::account::{AccountDiff, UpdateType},
        public_key::PublicKey,
        store::{
            best::{BestLedgerStore, DbAccountUpdate},
            staged::StagedLedgerStore,
        },
        Ledger,
    },
    store::{fixed_keys::FixedKeys, pk_key_prefix, to_be_bytes, u64_prefix_key, IndexerStore},
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::{collections::HashMap, mem::size_of};

impl BestLedgerStore for IndexerStore {
    fn get_best_account(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>> {
        trace!("Getting best ledger account {pk}");
        Ok(self
            .database
            .get_cf(self.best_ledger_accounts_cf(), pk.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice::<Account>(&bytes).ok()))
    }

    fn get_best_account_display(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>> {
        trace!("Display best ledger account {pk}");
        if let Some(best_acct) = self.get_best_account(pk)? {
            return Ok(Some(best_acct.display()));
        }
        Ok(None)
    }

    fn get_best_ledger(&self, memoize: bool) -> anyhow::Result<Option<Ledger>> {
        Ok(self.build_best_ledger()?.map(|best_ledger| {
            if let Ok(Some(state_hash)) = self.get_best_block_hash() {
                if memoize {
                    trace!("Memoizing best ledger (state hash {state_hash})");
                    self.add_staged_ledger_at_state_hash(&state_hash, best_ledger.clone())
                        .ok();
                }
            }
            best_ledger
        }))
    }

    fn update_best_account(&self, pk: &PublicKey, account: Option<Account>) -> anyhow::Result<()> {
        if account.is_none() {
            // delete stale data
            if let Some(acct) = self.get_best_account(pk)? {
                self.database
                    .delete_cf(self.best_ledger_accounts_cf(), pk.0.as_bytes())?;
                self.database.delete_cf(
                    self.best_ledger_accounts_balance_sort_cf(),
                    u64_prefix_key(acct.balance.0, &pk.0),
                )?;
            }
            return Ok(());
        }

        // update best account
        let account = account.unwrap();
        let balance = account.balance.0;
        let acct = self.get_best_account(pk)?;
        if let Some(acct) = acct.as_ref() {
            // delete stale balance sorting data
            self.database.delete_cf(
                self.best_ledger_accounts_balance_sort_cf(),
                u64_prefix_key(acct.balance.0, &pk.0),
            )?;
        }
        self.database.put_cf(
            self.best_ledger_accounts_cf(),
            pk.0.as_bytes(),
            serde_json::to_vec(&account)?,
        )?;
        self.database.put_cf(
            self.best_ledger_accounts_balance_sort_cf(),
            u64_prefix_key(balance, &pk.0),
            serde_json::to_vec(&account)?,
        )?;
        Ok(())
    }

    fn update_block_best_accounts(
        &self,
        state_hash: &BlockHash,
        blocks: &DbBlockUpdate,
    ) -> anyhow::Result<()> {
        let account_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .flat_map(|(a, _)| {
                    let diff = self.get_block_ledger_diff(a).unwrap();
                    diff.map(|d| {
                        (
                            d.account_diffs.into_iter().flatten().collect(),
                            d.new_pk_balances.into_keys().collect(),
                        )
                    })
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .flat_map(|(u, _)| {
                    let diff = self.get_block_ledger_diff(u).unwrap();
                    diff.map(|d| {
                        (
                            d.account_diffs.into_iter().flatten().collect(),
                            d.new_pk_balances.into_keys().collect(),
                        )
                    })
                })
                .collect(),
        };
        self.update_best_accounts(state_hash, &account_updates)
    }

    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: &DbAccountUpdate,
    ) -> anyhow::Result<()> {
        use AccountDiff::*;
        trace!("Updating best ledger accounts for block {state_hash}");

        // count newly applied & unapplied accounts
        let apply_acc = updates
            .apply
            .iter()
            .fold(0, |acc, update| acc + update.1.len() as i32);
        let adjust = updates
            .unapply
            .iter()
            .fold(apply_acc, |acc, update| acc - update.1.len() as i32);
        self.update_num_accounts(adjust)?;

        // update accounts
        // unapply
        for (block_diffs, remove_pks) in updates.unapply.iter() {
            for diff in block_diffs {
                let pk: PublicKey = diff.public_key();
                let acct = self
                    .get_best_account(&pk)?
                    .unwrap_or(Account::empty(pk.clone()));
                let account = match diff {
                    Payment(diff) => match diff.update_type {
                        UpdateType::Credit => Some(Account {
                            balance: acct.balance - diff.amount,
                            ..acct
                        }),
                        UpdateType::Debit(nonce) => Some(Account {
                            balance: acct.balance + diff.amount,
                            nonce: nonce.map_or(acct.nonce, |nonce| {
                                if acct.nonce.map(|n| n.0) == Some(0) {
                                    None
                                } else {
                                    Some(nonce - 1)
                                }
                            }),
                            ..acct
                        }),
                    },
                    Coinbase(diff) => Some(Account {
                        balance: acct.balance - diff.amount,
                        ..acct
                    }),
                    Delegation(diff) => {
                        self.remove_pk_delegate(pk.clone())?;
                        Some(Account {
                            nonce: if acct.nonce.map(|n| n.0) == Some(0) {
                                None
                            } else {
                                Some(diff.nonce - 1)
                            },
                            delegate: diff.delegate.clone(),
                            ..acct
                        })
                    }
                    FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => match diff.update_type {
                        UpdateType::Credit => Some(Account {
                            balance: acct.balance - diff.amount,
                            ..acct
                        }),
                        UpdateType::Debit(_) => Some(Account {
                            balance: acct.balance + diff.amount,
                            ..acct
                        }),
                    },
                    FailedTransactionNonce(diff) => Some(Account {
                        nonce: if acct.nonce.map(|n| n.0) == Some(0) {
                            None
                        } else {
                            Some(diff.nonce - 1)
                        },
                        ..acct
                    }),
                };
                self.update_best_account(&pk, account)?;
            }

            // remove accounts
            for pk in remove_pks.iter() {
                self.update_best_account(pk, None)?;
            }
        }

        // apply
        for (block_apply_diffs, _) in updates.apply.iter() {
            for diff in block_apply_diffs {
                let pk = diff.public_key();
                let acct = self
                    .get_best_account(&pk)?
                    .unwrap_or(Account::empty(pk.clone()));
                let account = match diff {
                    Payment(diff) => match diff.update_type {
                        UpdateType::Credit => Some(Account {
                            balance: acct.balance + diff.amount,
                            ..acct
                        }),
                        UpdateType::Debit(nonce) => Some(Account {
                            balance: acct.balance - diff.amount,
                            nonce: nonce.or(acct.nonce),
                            ..acct
                        }),
                    },
                    Coinbase(diff) => Some(Account {
                        balance: acct.balance + diff.amount,
                        ..acct
                    }),
                    Delegation(diff) => Some(Account {
                        nonce: Some(diff.nonce),
                        delegate: diff.delegate.clone(),
                        ..acct
                    }),
                    FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => match diff.update_type {
                        UpdateType::Credit => Some(Account {
                            balance: acct.balance + diff.amount,
                            ..acct
                        }),
                        UpdateType::Debit(Some(nonce)) => Some(Account {
                            balance: acct.balance - diff.amount,
                            nonce: Some(nonce + 1),
                            ..acct
                        }),
                        UpdateType::Debit(None) => Some(Account {
                            balance: acct.balance - diff.amount,
                            ..acct
                        }),
                    },
                    FailedTransactionNonce(diff) => Some(Account {
                        nonce: Some(diff.nonce),
                        ..acct
                    }),
                };
                self.update_best_account(&pk, account)?;
            }
        }
        Ok(())
    }

    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> anyhow::Result<()> {
        trace!("Adding pk {pk} delegate {delegate}");
        let num = self.get_num_pk_delegations(pk)?;

        // update num delegations
        self.database.put_cf(
            self.best_ledger_accounts_num_delegations_cf(),
            pk.0.as_bytes(),
            to_be_bytes(num + 1),
        )?;

        // append new delegation
        let mut key = pk.clone().to_bytes().to_vec();
        key.append(&mut to_be_bytes(num).to_vec());
        self.database.put_cf(
            self.best_ledger_accounts_delegations_cf(),
            key,
            delegate.0.as_bytes(),
        )?;
        Ok(())
    }

    fn get_num_pk_delegations(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_num_delegations_cf(),
                pk.0.as_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> anyhow::Result<Option<PublicKey>> {
        let mut key = pk.clone().to_bytes().to_vec();
        key.append(&mut to_be_bytes(idx).to_vec());
        Ok(self
            .database
            .get_cf(self.best_ledger_accounts_delegations_cf(), key)?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn remove_pk_delegate(&self, pk: PublicKey) -> anyhow::Result<()> {
        trace!("Removing pk {pk} delegate");
        let idx = self.get_num_pk_delegations(&pk)?;
        if idx > 0 {
            // update num delegations
            self.database.put_cf(
                self.best_ledger_accounts_num_delegations_cf(),
                pk.0.as_bytes(),
                to_be_bytes(idx - 1),
            )?;

            // drop delegation
            let mut key = pk.to_bytes().to_vec();
            key.append(&mut to_be_bytes(idx - 1).to_vec());
            self.database
                .delete_cf(self.best_ledger_accounts_delegations_cf(), key)?;
        }
        Ok(())
    }

    fn update_num_accounts(&self, adjust: i32) -> anyhow::Result<()> {
        use std::cmp::Ordering::*;
        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                let old = self
                    .database
                    .get(Self::TOTAL_NUM_ACCOUNTS_KEY)?
                    .map_or(0, from_be_bytes);
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                let old = self
                    .database
                    .get(Self::TOTAL_NUM_ACCOUNTS_KEY)?
                    .map_or(0, from_be_bytes);
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_sub(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
        }
        Ok(())
    }

    fn get_num_accounts(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .database
            .get(Self::TOTAL_NUM_ACCOUNTS_KEY)?
            .map(from_be_bytes))
    }

    fn build_best_ledger(&self) -> anyhow::Result<Option<Ledger>> {
        trace!("Building best ledger");
        if let (Some(best_block_height), Some(best_block_hash)) =
            (self.get_best_block_height()?, self.get_best_block_hash()?)
        {
            trace!("Best ledger (length {best_block_height}): {best_block_hash}");
            let mut accounts = HashMap::new();
            for (key, value) in self
                .best_ledger_account_balance_iterator(IteratorMode::End)
                .flatten()
            {
                let pk = pk_key_prefix(&key[size_of::<u64>()..]);
                accounts.insert(pk, serde_json::from_slice(&value)?);
            }
            return Ok(Some(Ledger { accounts }));
        }
        Ok(None)
    }

    ///////////////
    // Iterators //
    ///////////////

    fn best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.best_ledger_accounts_balance_sort_cf(), mode)
    }
}
