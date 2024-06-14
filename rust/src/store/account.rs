use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{internal::InternalCommand, Command, Payment},
    constants::MAINNET_GENESIS_HASH,
    ledger::{
        diff::account::{PaymentDiff, UpdateType},
        public_key::PublicKey,
    },
};
use serde::{Deserialize, Serialize};
use speedb::{DBIterator, IteratorMode};
use std::collections::{HashMap, HashSet};

pub trait AccountStore {
    /// Update pk's balance-sorted account balance
    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn common_ancestor_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<(Vec<PaymentDiff>, HashSet<PublicKey>)>;

    /// Set the balance updates for a block
    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: PublicKey,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()>;

    /// Get a block's balance updates
    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<(PublicKey, Vec<PaymentDiff>)>>;

    /// Updates stored account balances
    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: Vec<PaymentDiff>,
        coinbase_receivers: HashSet<PublicKey>,
    ) -> anyhow::Result<()>;

    /// Get pk's account balance
    fn get_account_balance(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

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

/// Only used for sorting the best ledger
#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdate<T> {
    pub apply: Vec<T>,
    pub unapply: Vec<T>,
}

impl AccountUpdate<PaymentDiff> {
    pub fn new(apply: Vec<PaymentDiff>, unapply: Vec<PaymentDiff>) -> Self {
        Self { apply, unapply }
    }

    pub fn balance_updates(diffs: Vec<PaymentDiff>) -> HashMap<String, i64> {
        let mut res = HashMap::new();
        for diff in diffs {
            let pk = diff.public_key.0;
            let acc = res.remove(&pk).unwrap_or(0);
            res.insert(
                pk,
                match diff.update_type {
                    UpdateType::Credit => acc + diff.amount.0 as i64,
                    UpdateType::Debit(_) => acc - diff.amount.0 as i64,
                },
            );
        }
        res
    }

    /// Unapply `self.unapply` & apply `self.apply` diffs
    pub fn to_diff_vec(self) -> Vec<PaymentDiff> {
        [
            self.unapply
                .into_iter()
                .map(|diff| diff.unapply())
                .collect(),
            self.apply,
        ]
        .concat()
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<PaymentDiff> {
        // magic mina
        if block.state_hash().0 == MAINNET_GENESIS_HASH {
            return vec![PaymentDiff {
                update_type: UpdateType::Credit,
                public_key: block.block_creator(),
                amount: 1000_u64.into(),
            }];
        }

        // otherwise
        [
            Command::from_precomputed(block)
                .into_iter()
                .flat_map(|cmd| match cmd {
                    Command::Payment(Payment {
                        source,
                        amount,
                        receiver,
                        nonce: _,
                    }) => vec![
                        PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: source.clone(),
                            amount,
                        },
                        PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount,
                        },
                    ],
                    Command::Delegation(_) => vec![],
                })
                .collect::<Vec<PaymentDiff>>(),
            InternalCommand::from_precomputed(block)
                .iter()
                .flat_map(|cmd| match cmd {
                    InternalCommand::Coinbase { receiver, amount } => vec![PaymentDiff {
                        update_type: UpdateType::Credit,
                        public_key: receiver.clone(),
                        amount: (*amount).into(),
                    }],
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
                        PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: sender.clone(),
                            amount: (*amount).into(),
                        },
                        PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.clone(),
                            amount: (*amount).into(),
                        },
                    ],
                })
                .collect(),
        ]
        .concat()
    }
}

impl<T> std::fmt::Debug for AccountUpdate<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "apply:   {:#?}\nunapply: {:#?}",
            self.apply, self.unapply
        )
    }
}
