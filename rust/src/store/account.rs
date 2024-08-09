use super::DBUpdate;
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{internal::InternalCommand, Command, Payment},
    constants::MAINNET_GENESIS_HASH,
    ledger::{
        coinbase::Coinbase,
        diff::account::{PaymentDiff, UpdateType},
        public_key::PublicKey,
    },
};
use serde::{Deserialize, Serialize};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

pub trait AccountStore {
    /// Update pk's balance-sorted account balance
    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn reorg_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<DBAccountBalanceUpdate>;

    /// Set the balance updates for a block
    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        balance_updates: &[AccountBalanceUpdate],
    ) -> anyhow::Result<()>;

    /// Get a block's balance updates
    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<AccountBalanceUpdate>>>;

    /// Updates stored account balances
    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: &DBAccountBalanceUpdate,
    ) -> anyhow::Result<()>;

    /// Get pk's account balance
    fn get_account_balance(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

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
    fn account_balance_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountBalanceUpdate {
    Payment(PaymentDiff),
    CreateAccount(PublicKey),
    RemoveAccount(PublicKey),
}

pub type DBAccountBalanceUpdate = DBUpdate<AccountBalanceUpdate>;

impl AccountBalanceUpdate {
    pub fn unapply(self) -> Self {
        match self {
            Self::Payment(diff) => {
                let PaymentDiff {
                    update_type,
                    public_key,
                    amount,
                } = diff;

                // change update type: debit <-> credit
                let update_type = match update_type {
                    UpdateType::Credit => UpdateType::Debit(None),
                    UpdateType::Debit(_) => UpdateType::Credit,
                };
                Self::Payment(PaymentDiff {
                    update_type,
                    public_key,
                    amount,
                })
            }
            Self::CreateAccount(pk) => Self::RemoveAccount(pk),
            Self::RemoveAccount(pk) => Self::CreateAccount(pk),
        }
    }
}

impl DBAccountBalanceUpdate {
    pub fn new(apply: Vec<AccountBalanceUpdate>, unapply: Vec<AccountBalanceUpdate>) -> Self {
        Self { apply, unapply }
    }

    /// Update balances for pk -> Some(bal),
    /// remove accounts for pk -> None
    pub fn balance_updates(&self) -> HashMap<PublicKey, Option<i64>> {
        use AccountBalanceUpdate::*;
        let mut res = <HashMap<PublicKey, Option<i64>>>::new();
        for account_balance_update in self.to_balance_update_vec() {
            match account_balance_update {
                Payment(diff) => {
                    let pk = diff.public_key.clone();
                    let acc = res.remove(&pk).unwrap_or_default().unwrap_or_default();
                    res.insert(
                        pk,
                        match diff.update_type {
                            UpdateType::Credit => Some(acc + diff.amount.0 as i64),
                            UpdateType::Debit(_) => Some(acc - diff.amount.0 as i64),
                        },
                    );
                }
                CreateAccount(pk) => {
                    res.insert(pk.clone(), Some(0));
                }
                RemoveAccount(pk) => {
                    res.insert(pk.clone(), None);
                }
            }
        }
        res
    }

    /// Unapply `self.unapply` & apply `self.apply` diffs
    pub fn to_balance_update_vec(&self) -> Vec<AccountBalanceUpdate> {
        [
            self.unapply
                .iter()
                .cloned()
                .map(|diff| diff.unapply())
                .collect(),
            self.apply.clone(),
        ]
        .concat()
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<AccountBalanceUpdate> {
        // magic mina
        if block.state_hash().0 == MAINNET_GENESIS_HASH {
            return vec![AccountBalanceUpdate::Payment(PaymentDiff {
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
                        nonce: _,
                        ..
                    }) => vec![
                        AccountBalanceUpdate::Payment(PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount,
                        }),
                        AccountBalanceUpdate::Payment(PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: source.clone(),
                            amount,
                        }),
                    ],
                    Command::Delegation(_) => vec![],
                })
                .collect::<Vec<AccountBalanceUpdate>>(),
            vec![AccountBalanceUpdate::Payment(PaymentDiff {
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
                        AccountBalanceUpdate::Payment(PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount: (*amount).into(),
                        }),
                        AccountBalanceUpdate::Payment(PaymentDiff {
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
                .cloned()
                .map(AccountBalanceUpdate::CreateAccount)
                .collect(),
        );
        res
    }
}
