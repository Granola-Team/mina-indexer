use super::{column_families::ColumnFamilyHelpers, from_be_bytes};
use crate::{
    block::{store::BlockStore, BlockHash},
    constants::MAINNET_GENESIS_HASH,
    ledger::{
        account::{Account, Nonce},
        diff::account::{AccountDiff, UpdateType},
        public_key::PublicKey,
        store::{
            best::{BestLedgerStore, DBAccountUpdate},
            staged::StagedLedgerStore,
        },
        Ledger,
    },
    store::{fixed_keys::FixedKeys, to_be_bytes, u64_prefix_key, IndexerStore},
};
use log::trace;
use speedb::{DBIterator, IteratorMode};

impl BestLedgerStore for IndexerStore {
    fn get_best_ledger(&self) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting best ledger");
        self.get_ledger_state_hash(&self.get_best_block_hash()?.expect("best block"), true)
    }

    fn reorg_account_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<DBAccountUpdate> {
        trace!(
            "Getting common ancestor account balance updates:\n  old: {}\n  new: {}",
            old_best_tip,
            new_best_tip
        );

        // follows the old best tip back to the common ancestor
        let mut a = old_best_tip.clone();
        let mut unapply = vec![];

        // follows the new best tip back to the common ancestor
        let mut b = new_best_tip.clone();
        let mut apply = vec![];

        let a_length = self.get_block_height(&a)?.expect("a has a length");
        let b_length = self.get_block_height(&b)?.expect("b has a length");

        // bring b back to the same height as a
        for _ in 0..b_length.saturating_sub(a_length) {
            // check if there's a previous block
            if b.0 == MAINNET_GENESIS_HASH {
                break;
            }

            apply.append(&mut self.get_block_account_diffs(&b)?.unwrap());
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && a.0 != MAINNET_GENESIS_HASH {
            // add blocks to appropriate collection
            unapply.append(&mut self.get_block_account_diffs(&a)?.unwrap());
            apply.append(&mut self.get_block_account_diffs(&b)?.unwrap());

            // descend
            a = a_prev;
            b = b_prev;

            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        apply.reverse();
        Ok(<DBAccountUpdate>::new(
            apply.into_iter().flatten().collect(),
            unapply.into_iter().flatten().collect(),
        ))
    }

    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: &DBAccountUpdate,
    ) -> anyhow::Result<()> {
        use AccountDiff::*;
        trace!("Updating account balances {state_hash}");

        let apply_iter = updates.apply.iter();
        let unapply_iter = updates.unapply.iter();

        let apply_acc = apply_iter.clone().fold(0, |acc, update| match update {
            CreateAccount(_) => acc + 1,
            _ => acc,
        });
        let adjust = unapply_iter
            .clone()
            .fold(apply_acc, |acc, update| match update {
                CreateAccount(_) => acc - 1,
                _ => acc,
            });
        self.update_num_accounts(adjust)?;

        // update accounts
        // unapply
        for diff in unapply_iter.clone() {
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
                    UpdateType::Debit(Some(nonce)) => Some(Account {
                        balance: acct.balance + diff.amount,
                        nonce: if acct.nonce.map(|n| n.0) == Some(0) {
                            None
                        } else {
                            Some(nonce - 1)
                        },
                        ..acct
                    }),
                    UpdateType::Debit(None) => unreachable!(),
                },
                Coinbase(diff) => Some(Account {
                    balance: acct.balance - diff.amount,
                    ..acct
                }),
                CreateAccount(diff) => match diff.update_type {
                    UpdateType::Credit => Some(Account {
                        balance: acct.balance - diff.amount,
                        ..acct
                    }), // decrement balance on the virtual account used for accounting purposes
                    UpdateType::Debit(_) => None, // uncreate the account
                },
                Delegation(diff) => Some(Account {
                    nonce: if acct.nonce.map(|n| n.0) == Some(0) {
                        None
                    } else {
                        Some(diff.nonce - 1)
                    },
                    delegate: diff.delegate.clone(),
                    ..acct
                }),
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

        // apply
        for diff in apply_iter.clone() {
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
                        nonce: acct.nonce.map(|n| nonce.unwrap_or_default().max(n) + 1),
                        ..acct
                    }),
                },
                Coinbase(diff) => Some(Account {
                    balance: acct.balance + diff.amount,
                    ..acct
                }),
                CreateAccount(diff) => match diff.update_type {
                    UpdateType::Credit => Some(Account {
                        balance: acct.balance + diff.amount,
                        ..acct
                    }),
                    UpdateType::Debit(_nonce) => Some(Account {
                        balance: acct.balance - diff.amount,
                        nonce: acct.nonce, // no nonce increment?
                        ..acct
                    }),
                },
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
                        nonce: Some(Nonce::default()),
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

        Ok(())
    }

    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> anyhow::Result<()> {
        trace!("Adding pk {pk} delegate {delegate}");
        let num = self.get_num_pk_delegations(pk)?;

        // update num delegations
        self.database.put_cf(
            self.account_num_delegations(),
            pk.0.as_bytes(),
            to_be_bytes(num + 1),
        )?;

        // append new delegation
        let mut key = pk.clone().to_bytes();
        key.append(&mut to_be_bytes(num));
        self.database
            .put_cf(self.account_delegations(), key, delegate.0.as_bytes())?;
        Ok(())
    }

    fn get_num_pk_delegations(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        Ok(self
            .database
            .get_cf(self.account_num_delegations(), pk.0.as_bytes())?
            .map_or(0, from_be_bytes))
    }

    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> anyhow::Result<Option<PublicKey>> {
        let mut key = pk.clone().to_bytes();
        key.append(&mut to_be_bytes(idx));
        Ok(self
            .database
            .get_cf(self.account_delegations(), key)?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn remove_pk_delegate(&self, pk: PublicKey) -> anyhow::Result<()> {
        trace!("Removing pk {pk} delegate");
        let idx = self.get_num_pk_delegations(&pk)?;
        if idx > 0 {
            // update num delegations
            self.database.put_cf(
                self.account_num_delegations(),
                pk.0.as_bytes(),
                to_be_bytes(idx - 1),
            )?;

            // drop delegation
            let mut key = pk.to_bytes();
            key.append(&mut to_be_bytes(idx - 1));
            self.database.delete_cf(self.account_delegations(), key)?;
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

    fn update_best_account(&self, pk: &PublicKey, account: Option<Account>) -> anyhow::Result<()> {
        if account.is_none() {
            // delete stale data
            if let Some(acct) = self.get_best_account(pk)? {
                self.database
                    .delete_cf(self.accounts_cf(), pk.0.as_bytes())?;
                self.database.delete_cf(
                    self.accounts_balance_sort_cf(),
                    u64_prefix_key(acct.balance.0, &pk.0),
                )?;
            }
            return Ok(());
        }

        // update account
        let account = account.unwrap();
        let balance = account.balance.0;
        let acct = self.get_best_account(pk)?;
        if let Some(acct) = acct.as_ref() {
            // delete stale balance sorting data
            self.database.delete_cf(
                self.accounts_balance_sort_cf(),
                u64_prefix_key(acct.balance.0, &pk.0),
            )?;
        }

        self.database.put_cf(
            self.accounts_cf(),
            pk.0.as_bytes(),
            serde_json::to_vec(&account)?,
        )?;

        // add: {balance}{pk} -> _
        self.database.put_cf(
            self.accounts_balance_sort_cf(),
            u64_prefix_key(balance, &pk.0),
            b"",
        )?;
        Ok(())
    }

    fn get_best_account(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>> {
        trace!("Getting account balance {pk}");
        Ok(self
            .database
            .get_cf(self.accounts_cf(), pk.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.accounts_balance_sort_cf(), mode)
    }
}
