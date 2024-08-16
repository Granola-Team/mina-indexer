//! Store of the best ledger

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{internal::InternalCommand, Command, Payment},
    constants::{MAINNET_ACCOUNT_CREATION_FEE, MAINNET_GENESIS_HASH},
    ledger::{
        account::Account,
        coinbase::Coinbase,
        diff::account::{AccountDiff, PaymentDiff, UpdateType},
        public_key::PublicKey,
        Ledger,
    },
    store::DBUpdate,
};
use speedb::{DBIterator, IteratorMode};
use std::{collections::HashMap, ops::Sub as _};

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
                    if let Some((acc_balance, acc_nonce, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
                        res.insert(
                            pk,
                            Some((
                                acc_balance - diff.amount.0 as i64,
                                acc_nonce,
                                acc_delegation,
                            )),
                        );
                    }
                }
                CreateAccount(diff) => {
                    res.insert(diff.public_key.clone(), None);
                }
                Delegation(diff) => {
                    if let Some((acc_balance, acc_nonce, _)) = res.remove(&pk).unwrap_or_default() {
                        res.insert(
                            pk.clone(),
                            Some((acc_balance, acc_nonce, Some(diff.delegate.clone()))),
                        );
                    }
                }
                FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                    if let Some((acc_balance, acc_nonce, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
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
                }
                FailedTransactionNonce(diff) => {
                    if let Some((acc_balance, _, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
                        res.insert(
                            pk.clone(),
                            Some((acc_balance, diff.nonce.0 as i32, acc_delegation)),
                        );
                    }
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
                    if let Some((acc_balance, acc_nonce, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
                        res.insert(
                            pk,
                            Some((
                                acc_balance + diff.amount.0 as i64,
                                acc_nonce,
                                acc_delegation,
                            )),
                        );
                    }
                }
                CreateAccount(diff) => {
                    res.insert(diff.public_key.clone(), Some((0, 0, None)));
                }
                Delegation(diff) => {
                    if let Some((acc_balance, acc_nonce, _)) = res.remove(&pk).unwrap_or_default() {
                        res.insert(
                            pk.clone(),
                            Some((acc_balance, acc_nonce, Some(diff.delegate.clone()))),
                        );
                    }
                }
                FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                    if let Some((acc_balance, acc_nonce, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
                        let (acc_balance, acc_nonce) = match diff.update_type {
                            UpdateType::Credit => (acc_balance + diff.amount.0 as i64, acc_nonce),
                            UpdateType::Debit(_) => (acc_balance - diff.amount.0 as i64, acc_nonce),
                        };
                        res.insert(pk.clone(), Some((acc_balance, acc_nonce, acc_delegation)));
                    }
                }
                FailedTransactionNonce(diff) => {
                    if let Some((acc_balance, _, acc_delegation)) =
                        res.remove(&pk).unwrap_or_default()
                    {
                        res.insert(
                            pk.clone(),
                            Some((acc_balance, diff.nonce.0 as i32, acc_delegation)),
                        );
                    }
                }
            }
        }
        res
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<AccountDiff> {
        // magic mina
        if block.state_hash().0 == MAINNET_GENESIS_HASH {
            return vec![AccountDiff::Payment(PaymentDiff {
                update_type: UpdateType::Credit,
                public_key: block.block_creator(),
                amount: 1000_u64.into(),
            })];
        }

        // otherwise
        let coinbase = Coinbase::from_precomputed(block);
        let mut res = [
            Command::from_precomputed(block)
                .into_iter()
                .flat_map(|cmd| match cmd {
                    Command::Payment(Payment {
                        source,
                        amount,
                        receiver,
                        is_new_receiver_account,
                        nonce: _,
                    }) => vec![
                        AccountDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: source.clone(),
                            amount,
                        }),
                        AccountDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount: if is_new_receiver_account {
                                amount.sub(MAINNET_ACCOUNT_CREATION_FEE)
                            } else {
                                amount
                            },
                        }),
                        AccountDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: source.clone(),
                            amount,
                        }),
                    ],
                    Command::Delegation(_) => vec![],
                })
                .collect::<Vec<AccountDiff>>(),
            vec![AccountDiff::Payment(PaymentDiff {
                update_type: UpdateType::Credit,
                public_key: coinbase.receiver.clone(),
                amount: coinbase.amount().into(),
            })],
            InternalCommand::from_precomputed(block)
                .iter()
                .flat_map(|cmd| match cmd {
                    InternalCommand::Coinbase { .. } => vec![],
                    InternalCommand::FeeTransfer {
                        sender,
                        receiver,
                        amount,
                    }
                    | InternalCommand::FeeTransferViaCoinbase {
                        sender,
                        receiver,
                        amount,
                    } => vec![
                        AccountDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount: (*amount).into(),
                        }),
                        AccountDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: sender.clone(),
                            amount: (*amount).into(),
                        }),
                    ],
                })
                .collect(),
        ]
        .concat();

        res.append(
            &mut block
                .accounts_created()
                .0
                .keys()
                .flat_map(AccountDiff::account_creation_payment_diff)
                .collect(),
        );
        res
    }
}
