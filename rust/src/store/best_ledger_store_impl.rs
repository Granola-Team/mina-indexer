use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, DbUpdate, IndexerStore};
use crate::{
    block::{
        store::{BlockStore, BlockUpdate, DbBlockUpdate},
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
        token::TokenAddress,
        Ledger, TokenLedger,
    },
    utility::store::{from_be_bytes, ledger::best::*, pk_index_key},
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

impl BestLedgerStore for IndexerStore {
    fn get_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Getting best ledger account {pk}");
        Ok(self
            .database
            .get_cf(self.best_ledger_accounts_cf(), best_account_key(token, pk))?
            .and_then(|bytes| serde_json::from_slice::<Account>(&bytes).ok()))
    }

    fn get_best_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Display best ledger account {pk}");
        if let Some(best_acct) = self.get_best_account(pk, token)? {
            return Ok(Some(best_acct.display()));
        }
        Ok(None)
    }

    fn get_best_ledger(&self, memoize: bool) -> anyhow::Result<Option<Ledger>> {
        Ok(self.build_best_ledger()?.inspect(|best_ledger| {
            if let Ok(Some(state_hash)) = self.get_best_block_hash() {
                if memoize {
                    trace!("Memoizing best ledger (state hash {state_hash})");
                    self.add_staged_ledger_at_state_hash(&state_hash, best_ledger.clone())
                        .ok();
                }
            }
        }))
    }

    fn update_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        account: Option<Account>,
    ) -> anyhow::Result<()> {
        // remove account
        if account.is_none() {
            if let Some(acct) = self.get_best_account(pk, token)? {
                self.database
                    .delete_cf(self.best_ledger_accounts_cf(), best_account_key(token, pk))?;
                self.database.delete_cf(
                    self.best_ledger_accounts_balance_sort_cf(),
                    best_account_sort_key(token, acct.balance.0, pk),
                )?;
            }
            return Ok(());
        }

        // update best account
        let account = account.unwrap();
        let balance = account.balance.0;
        let acct = self.get_best_account(pk, token)?;

        if let Some(acct) = acct.as_ref() {
            // delete stale balance sorting data
            self.database.delete_cf(
                self.best_ledger_accounts_balance_sort_cf(),
                best_account_sort_key(token, acct.balance.0, pk),
            )?;
        }

        // write new account
        self.database.put_cf(
            self.best_ledger_accounts_cf(),
            best_account_key(token, pk),
            serde_json::to_vec(&account)?,
        )?;

        if let Some(_zkapp) = account.zkapp {
            // populate index for best_ledger_zkapps_balance_sort_cf
            // populate index for best_ledger_tokens_balance_sort_cf
        } else {
            self.database.put_cf(
                self.best_ledger_accounts_balance_sort_cf(),
                best_account_sort_key(token, balance, pk),
                serde_json::to_vec(&account)?,
            )?;
        }

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
                .flat_map(|BlockUpdate { state_hash: a, .. }| {
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
                .flat_map(|BlockUpdate { state_hash: u, .. }| {
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
                let token = diff.token_address();

                let acct = self
                    .get_best_account(&pk, &token)?
                    .unwrap_or(Account::empty(pk.clone(), token.clone()));

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
                            delegate: acct.public_key.clone(),
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
                    Zkapp(_diff) => todo!("zkapp diff unapply update_best_accounts"),
                };

                self.update_best_account(&pk, &token, account)?;
            }

            // TODO generalize over tokens
            // remove accounts
            for pk in remove_pks.iter() {
                self.update_best_account(pk, &TokenAddress::default(), None)?;
            }
        }

        // apply
        for (block_apply_diffs, _) in updates.apply.iter() {
            for diff in block_apply_diffs {
                let pk = diff.public_key();
                let token = diff.token_address();

                let acct = self
                    .get_best_account(&pk, &token)?
                    .unwrap_or(Account::empty(pk.clone(), token.clone()));

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
                    Zkapp(_diff) => todo!("zkapp diff apply update_best_accounts"),
                };

                self.update_best_account(&pk, &token, account)?;
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
            (num + 1).to_be_bytes(),
        )?;

        // append new delegation
        self.database.put_cf(
            self.best_ledger_accounts_delegations_cf(),
            pk_index_key(pk, num),
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
        trace!("Getting pk {pk} delegation index {idx}");
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_delegations_cf(),
                pk_index_key(pk, idx),
            )?
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
                (idx - 1).to_be_bytes(),
            )?;

            // drop delegation
            self.database.delete_cf(
                self.best_ledger_accounts_delegations_cf(),
                pk_index_key(&pk, idx - 1),
            )?;
        }
        Ok(())
    }

    fn update_num_accounts(&self, adjust: i32) -> anyhow::Result<()> {
        use std::cmp::Ordering::*;
        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                let old = self.get_num_accounts().ok().flatten().unwrap_or(0);
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                let old = self.get_num_accounts().ok().flatten().unwrap_or(0);
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

            for (_, value) in self
                .best_ledger_account_balance_iterator(IteratorMode::End)
                .flatten()
            {
                let account: Account = serde_json::from_slice(&value)?;
                accounts.insert(account.public_key.clone(), account);
            }

            return Ok(Some(Ledger::from_mina_ledger(TokenLedger { accounts })));
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
