//! Store of the best ledger

use crate::{
    block::BlockHash,
    ledger::{
        account::Account,
        diff::account::{AccountDiff, UpdateType},
        public_key::PublicKey,
        Ledger,
    },
    store::DBUpdate,
};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

pub trait BestLedgerStore {
    /// Get the best ledger (associated with the best block)
    fn get_best_ledger(&self) -> anyhow::Result<Option<Ledger>>;

    /// Update pk's account
    fn update_best_account(&self, pk: &PublicKey, account: Option<Account>) -> anyhow::Result<()>;

    /// Updates balance-sorted accounts
    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: &DBAccountUpdate,
    ) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn reorg_account_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<DBAccountUpdate>;

    /// Get pk's account balance
    fn get_best_account(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>>;

    /// Remove pk delegation
    fn remove_pk_delegate(&self, pk: PublicKey) -> anyhow::Result<()>;

    /// Add pk delegation
    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> anyhow::Result<()>;

    /// Get pk's number of delegations
    fn get_num_pk_delegations(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Get `pk`'s `idx`-th delegation
    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> anyhow::Result<Option<PublicKey>>;

    /// Update best ledger accounts count
    fn update_num_accounts(&self, adjust: i32) -> anyhow::Result<()>;

    /// Get best ledger accounts count
    fn get_num_accounts(&self) -> anyhow::Result<Option<u32>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted accounts
    /// `{balance}{pk} -> _`
    /// ```
    /// - balance: 8 BE bytes
    /// - pk:      [PublicKey::LEN] bytes
    fn account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}

pub type DBAccountUpdate = DBUpdate<AccountDiff>;

impl DBAccountUpdate {
    pub fn new(apply: Vec<AccountDiff>, unapply: Vec<AccountDiff>) -> Self {
        Self { apply, unapply }
    }

    /// Update account for pk -> Some(bal),
    /// remove account for pk -> None
    pub fn account_updates(&self) -> HashMap<PublicKey, Option<(i64, i32, Option<PublicKey>)>> {
        use AccountDiff::*;
        let mut res = <HashMap<PublicKey, Option<(i64, i32, Option<PublicKey>)>>>::new();

        // unapply
        for account_diff in &self.unapply {
            let pk = account_diff.public_key();
            match account_diff {
                Payment(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk,
                        match diff.update_type {
                            UpdateType::Credit => Some((
                                acc_balance - diff.amount.0 as i64,
                                acc_nonce,
                                acc_delegation,
                            )),
                            UpdateType::Debit(nonce) => Some((
                                acc_balance + diff.amount.0 as i64,
                                nonce.map_or(acc_nonce, |n| n.0 as i32 - 1),
                                None,
                            )),
                        },
                    );
                }
                Coinbase(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();

                    res.insert(
                        pk,
                        Some((
                            acc_balance - diff.amount.0 as i64,
                            acc_nonce,
                            acc_delegation,
                        )),
                    );
                }
                CreateAccount(diff) => {
                    res.insert(diff.public_key.clone(), None);
                }
                Delegation(diff) => {
                    let (acc_balance, acc_nonce, _) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk.clone(),
                        Some((acc_balance, acc_nonce, Some(diff.delegate.clone()))),
                    );
                }
                FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();

                    res.insert(
                        pk.clone(),
                        match diff.update_type {
                            UpdateType::Credit => Some((
                                acc_balance - diff.amount.0 as i64,
                                acc_nonce,
                                acc_delegation,
                            )),
                            UpdateType::Debit(_) => {
                                Some((acc_balance + diff.amount.0 as i64, acc_nonce, None))
                            }
                        },
                    );
                }
                FailedTransactionNonce(diff) => {
                    let (acc_balance, _, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk.clone(),
                        Some((acc_balance, diff.nonce.0 as i32, acc_delegation)),
                    );
                }
            }
        }

        // apply
        for account_diff in self.apply.iter() {
            let pk = account_diff.public_key();
            match account_diff {
                Payment(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk,
                        match diff.update_type {
                            UpdateType::Credit => Some((
                                acc_balance + diff.amount.0 as i64,
                                acc_nonce,
                                acc_delegation,
                            )),
                            UpdateType::Debit(nonce) => Some((
                                acc_balance - diff.amount.0 as i64,
                                nonce.map_or(0, |n| (n.0 + 1) as i32),
                                None,
                            )),
                        },
                    );
                }
                Coinbase(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk,
                        Some((
                            acc_balance + diff.amount.0 as i64,
                            acc_nonce,
                            acc_delegation,
                        )),
                    );
                }
                CreateAccount(diff) => {
                    res.insert(diff.public_key.clone(), Some((0, 0, None)));
                }
                Delegation(diff) => {
                    let (acc_balance, acc_nonce, _) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk.clone(),
                        Some((acc_balance, acc_nonce, Some(diff.delegate.clone()))),
                    );
                }
                FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                    let (acc_balance, acc_nonce, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();

                    let (acc_balance, acc_nonce) = match diff.update_type {
                        UpdateType::Credit => (acc_balance + diff.amount.0 as i64, acc_nonce),
                        UpdateType::Debit(_) => (acc_balance - diff.amount.0 as i64, acc_nonce),
                    };
                    res.insert(pk.clone(), Some((acc_balance, acc_nonce, acc_delegation)));
                }
                FailedTransactionNonce(diff) => {
                    let (acc_balance, _, acc_delegation) =
                        res.remove(&pk).unwrap_or_default().unwrap_or_default();

                    res.insert(
                        pk.clone(),
                        Some((acc_balance, diff.nonce.0 as i32, acc_delegation)),
                    );
                }
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{
        account::{Amount, Nonce},
        diff::account::{
            AccountDiff, CoinbaseDiff, DelegationDiff, FailedTransactionNonceDiff, PaymentDiff,
            UpdateType,
        },
        public_key::PublicKey,
    };

    fn public_key(_n: u8) -> PublicKey {
        PublicKey::new("B62qj9mXCjVYfsbshbGJsB62qz9mXCjVYfsbshbGJsj9mXCjV")
    }

    #[test]
    fn test_account_updates_unapply_payment_credit() {
        let pk = public_key(1);
        let account_diff = AccountDiff::Payment(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(100),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((-100, 0, None))));
    }

    #[test]
    fn test_account_updates_unapply_payment_debit() {
        let pk = public_key(1);

        let account_diff = AccountDiff::Payment(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(50),
            update_type: UpdateType::Debit(Some(Nonce(2))),
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        // Unapply the debit: should revert to balance = 100, nonce = 1
        assert_eq!(updates.get(&pk), Some(&Some((50, 1, None))));
    }

    #[test]
    fn test_account_updates_apply_payment_credit() {
        let pk = public_key(1);
        let account_diff = AccountDiff::Payment(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(100),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((100, 0, None))));
    }

    #[test]
    fn test_account_updates_apply_payment_debit() {
        let pk = public_key(1);
        let account_diff = AccountDiff::Payment(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(50),
            update_type: UpdateType::Debit(Some(Nonce(2))),
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((-50, 3, None))));
    }

    #[test]
    fn test_account_updates_unapply_coinbase() {
        let pk = public_key(2);
        let account_diff = AccountDiff::Coinbase(CoinbaseDiff {
            public_key: pk.clone(),
            amount: Amount(500),
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((-500, 0, None))));
    }

    #[test]
    fn test_account_updates_apply_coinbase() {
        let pk = public_key(2);
        let account_diff = AccountDiff::Coinbase(CoinbaseDiff {
            public_key: pk.clone(),
            amount: Amount(500),
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((500, 0, None))));
    }

    #[test]
    fn test_account_updates_unapply_create_account() {
        let pk = public_key(3);
        let account_diff = AccountDiff::CreateAccount(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(0),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&None));
    }

    #[test]
    fn test_account_updates_apply_create_account() {
        let pk = public_key(3);
        let account_diff = AccountDiff::CreateAccount(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(0),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((0, 0, None))));
    }

    #[test]
    fn test_account_updates_unapply_delegation() {
        let pk = public_key(4);
        let delegate_pk = public_key(5);
        let account_diff = AccountDiff::Delegation(DelegationDiff {
            nonce: Nonce(0),
            delegate: delegate_pk.clone(),
            delegator: pk.clone(),
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(
            updates.get(&pk),
            Some(&Some((0, 0, Some(delegate_pk.clone()))))
        );
    }

    #[test]
    fn test_account_updates_apply_delegation() {
        let pk = public_key(4);
        let delegate_pk = public_key(5);
        let account_diff = AccountDiff::Delegation(DelegationDiff {
            nonce: Nonce(0),
            delegate: delegate_pk.clone(),
            delegator: pk.clone(),
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(
            updates.get(&pk),
            Some(&Some((0, 0, Some(delegate_pk.clone()))))
        );
    }

    #[test]
    fn test_account_updates_unapply_fee_transfer() {
        let pk = public_key(6);
        let account_diff = AccountDiff::FeeTransfer(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(200),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((-200, 0, None))));
    }

    #[test]
    fn test_account_updates_apply_fee_transfer() {
        let pk = public_key(6);
        let account_diff = AccountDiff::FeeTransfer(PaymentDiff {
            public_key: pk.clone(),
            amount: Amount(200),
            update_type: UpdateType::Credit,
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((200, 0, None))));
    }

    #[test]
    fn test_account_updates_unapply_failed_transaction_nonce() {
        let pk = public_key(7);
        let account_diff = AccountDiff::FailedTransactionNonce(FailedTransactionNonceDiff {
            public_key: pk.clone(),
            nonce: Nonce(3),
        });

        let db_account_update = DBAccountUpdate::new(vec![], vec![account_diff]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((0, 3, None))));
    }

    #[test]
    fn test_account_updates_apply_failed_transaction_nonce() {
        let pk = public_key(7);
        let account_diff = AccountDiff::FailedTransactionNonce(FailedTransactionNonceDiff {
            public_key: pk.clone(),
            nonce: Nonce(3),
        });

        let db_account_update = DBAccountUpdate::new(vec![account_diff], vec![]);

        let updates = db_account_update.account_updates();

        assert_eq!(updates.get(&pk), Some(&Some((0, 3, None))));
    }
}
