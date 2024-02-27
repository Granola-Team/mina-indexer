pub mod account;
pub mod coinbase;
pub mod diff;
pub mod genesis;
pub mod post_balances;
pub mod public_key;
pub mod store;

use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::{
        account::{Account, Amount, Nonce},
        coinbase::Coinbase,
        diff::{
            account::{AccountDiff, UpdateType},
            LedgerDiff,
        },
        post_balances::{FeeTransferUpdate, PostBalance, PostBalanceUpdate},
        public_key::PublicKey,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub accounts: HashMap<PublicKey, Account>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NonGenesisLedger {
    pub ledger: Ledger,
}

#[derive(Debug, Clone)]
pub enum LedgerError {
    AccountNotFound,
    InvalidDelegation,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    fn apply_delegation(&mut self, delegator: PublicKey, new_delegate: PublicKey) {
        if let Some(account) = self.accounts.get_mut(&delegator) {
            account.delegate = new_delegate;
        }
    }

    fn apply_balance_update(&mut self, balance_update: PostBalance, nonce: Option<u32>) {
        if let Some(account) = self.accounts.get_mut(&balance_update.public_key) {
            if let Some(nonce) = nonce {
                account.nonce = Nonce(nonce + 1);
            }
            account.balance = Amount(balance_update.balance);
        } else {
            let public_key = balance_update.public_key.clone();
            let new_account = Account {
                public_key: public_key.clone(),
                balance: Amount(balance_update.balance),
                nonce: Nonce(nonce.unwrap_or(0)),
                delegate: public_key,
            };
            self.accounts.insert(balance_update.public_key, new_account);
        }
    }

    pub fn apply_post_balances(&mut self, precomputed_block: &PrecomputedBlock) {
        let balance_updates = PostBalanceUpdate::from_precomputed(precomputed_block);
        for balance_update in balance_updates {
            trace!(
                "Applying {}: {}",
                precomputed_block.blockchain_length,
                precomputed_block.state_hash
            );

            match balance_update {
                PostBalanceUpdate::Coinbase(coinbase_update) => {
                    trace!(
                        "(coinbase) {:?} {:?}",
                        coinbase_update.public_key,
                        coinbase_update.balance,
                    );

                    if Coinbase::from_precomputed(precomputed_block).is_coinbase_applied() {
                        self.apply_balance_update(coinbase_update, None)
                    }
                }
                PostBalanceUpdate::FeeTransfer(fee_update) => match fee_update {
                    FeeTransferUpdate::One(fee_update) => {
                        trace!(
                            "(fee tx 1) {:?} {:?}",
                            fee_update.public_key,
                            fee_update.balance,
                        );
                        self.apply_balance_update(fee_update, None);
                    }
                    FeeTransferUpdate::Two(fee_update1, fee_update2) => {
                        trace!(
                            "(fee tx 1) {:?}: {:?}",
                            fee_update1.public_key,
                            fee_update1.balance,
                        );
                        trace!(
                            "(fee tx 2) {:?}: {:?}",
                            fee_update2.public_key,
                            fee_update2.balance,
                        );
                        self.apply_balance_update(fee_update1, None);
                        self.apply_balance_update(fee_update2, None);
                    }
                },
                PostBalanceUpdate::User(user_update) => {
                    trace!(
                        "(fee payer) {:?} {:?}",
                        user_update.fee_payer.public_key,
                        user_update.fee_payer.balance,
                    );
                    trace!(
                        "(receiver) {:?}: {:?}",
                        user_update.receiver.public_key,
                        user_update.receiver.balance,
                    );
                    trace!(
                        "(source) {:?}: {:?}",
                        user_update.source.public_key,
                        user_update.source.balance,
                    );

                    if user_update.is_delegation() {
                        self.apply_delegation(
                            user_update.source.public_key.clone(),
                            user_update.receiver.public_key.clone(),
                        );
                    }

                    self.apply_balance_update(user_update.fee_payer, None);
                    self.apply_balance_update(user_update.receiver, None);
                    self.apply_balance_update(user_update.source, Some(user_update.source_nonce));
                }
            }
        }
    }

    pub fn apply_diff_from_precomputed(self, block: &PrecomputedBlock) -> anyhow::Result<Self> {
        let diff = LedgerDiff::from_precomputed(block);
        self.apply_diff(&diff)
    }

    /// Apply a ledger diff
    pub fn apply_diff(self, diff: &LedgerDiff) -> anyhow::Result<Self> {
        let mut ledger = self;
        ledger._apply_diff(diff)?;
        Ok(ledger)
    }

    pub fn _apply_diff_from_precomputed(&mut self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let diff = LedgerDiff::from_precomputed(block);
        self._apply_diff(&diff)?;
        Ok(())
    }

    /// Apply a ledger diff to a mutable ledger
    pub fn _apply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        let ledger_diff = diff.clone();
        let keys: Vec<PublicKey> = ledger_diff
            .account_diffs
            .iter()
            .map(|diff| diff.public_key())
            .collect();

        keys.into_iter().for_each(|public_key| {
            if self.accounts.get(&public_key).is_none() {
                self.accounts
                    .insert(public_key.clone(), Account::empty(public_key));
            }
        });

        for diff in ledger_diff.account_diffs {
            match self.accounts.remove(&diff.public_key()) {
                Some(account_before) => {
                    let account_after = match &diff {
                        AccountDiff::Payment(payment_diff) => match &payment_diff.update_type {
                            UpdateType::Deposit => {
                                Account::from_deposit(account_before.clone(), payment_diff.amount)
                            }
                            UpdateType::Deduction => {
                                Account::from_deduction(account_before.clone(), payment_diff.amount)
                                    .unwrap_or(account_before.clone())
                            }
                        },
                        AccountDiff::Delegation(delegation_diff) => {
                            assert_eq!(account_before.public_key, delegation_diff.delegator);
                            Account::from_delegation(
                                account_before.clone(),
                                delegation_diff.delegate.clone(),
                            )
                        }
                        AccountDiff::Coinbase(coinbase_diff) => {
                            Account::from_coinbase(account_before, coinbase_diff.amount)
                        }
                    };

                    self.accounts.insert(diff.public_key(), account_after);
                }
                None => {
                    return match diff {
                        AccountDiff::Coinbase(_) => Ok(()),
                        AccountDiff::Payment(_) => Err(LedgerError::AccountNotFound.into()),
                        AccountDiff::Delegation(_) => Err(LedgerError::InvalidDelegation.into()),
                    };
                }
            }
        }

        Ok(())
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> anyhow::Result<Self> {
        let mut ledger = Ledger::new();
        for (pubkey, balance, nonce, delgation) in value {
            let pk = PublicKey::new(pubkey);
            let delegate = delgation.map(PublicKey::new).unwrap_or(pk.clone());

            ledger.accounts.insert(
                pk.clone(),
                Account {
                    public_key: pk,
                    balance: balance.into(),
                    nonce: Nonce(nonce.unwrap_or_default()),
                    delegate,
                },
            );
        }
        Ok(ledger)
    }

    pub fn to_string_pretty(&self) -> String {
        let mut accounts = HashMap::new();
        for (pk, acct) in &self.accounts {
            accounts.insert(pk.to_address(), acct.clone());
        }

        serde_json::to_string_pretty(&accounts).unwrap()
    }
}

impl ToString for Ledger {
    fn to_string(&self) -> String {
        let mut accounts = HashMap::new();
        for (pk, acct) in &self.accounts {
            accounts.insert(pk.to_address(), acct.clone());
        }

        serde_json::to_string(&accounts).unwrap()
    }
}

impl std::str::FromStr for Ledger {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let deser: HashMap<String, Account> = serde_json::from_str(s)?;
        let mut accounts = HashMap::new();
        for (pk, acct) in deser {
            accounts.insert(PublicKey(pk), acct);
        }

        Ok(Ledger { accounts })
    }
}

impl PartialEq for Ledger {
    fn eq(&self, other: &Self) -> bool {
        for pk in self.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                debug!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk),
                    other.accounts.get(pk)
                );
                return false;
            }
        }
        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                debug!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk),
                    other.accounts.get(pk)
                );
                return false;
            }
        }
        true
    }
}

impl Eq for Ledger {}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (pk, acct) in &self.accounts {
            writeln!(f, "{} -> {}", pk.to_address(), acct.balance.0)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::AccountNotFound => write!(f, "Account not found in ledger: payment error"),
            LedgerError::InvalidDelegation => {
                write!(f, "Invalid data or parameters: delegation error")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

impl Amount {
    pub fn add(&self, other: &Amount) -> Amount {
        Self(self.0 + other.0)
    }

    pub fn sub(&self, other: &Amount) -> Amount {
        Self(self.0 - other.0)
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value)
    }
}

pub fn is_valid_hash(input: &str) -> bool {
    input.starts_with("jx") && input.len() == 51
}

#[cfg(test)]
mod tests {
    use super::{
        account::{Account, Amount},
        diff::{
            account::{AccountDiff, DelegationDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        public_key::PublicKey,
        Ledger,
    };
    use std::collections::HashMap;

    #[test]
    fn apply_diff_payment() {
        let diff_amount = 1.into();
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let account = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();

        accounts.insert(public_key.clone(), account);

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Payment(PaymentDiff {
                public_key: public_key.clone(),
                amount: diff_amount,
                update_type: UpdateType::Deposit,
            })],
        };
        let ledger = Ledger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_after = ledger.accounts.get(&public_key).expect("account get");
        assert_eq!(account_after.balance, diff_amount);
    }

    #[test]
    fn apply_diff_delegation() {
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let delegate_key =
            PublicKey::new("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6");
        let account = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();

        accounts.insert(public_key.clone(), account);

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Delegation(DelegationDiff {
                delegator: public_key.clone(),
                delegate: delegate_key.clone(),
            })],
        };
        let ledger = Ledger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_after = ledger.accounts.get(&public_key).expect("account get");
        assert_eq!(account_after.delegate, delegate_key);
    }

    #[test]
    fn apply_diff_payment_with_post_balance() {
        let diff_amount = 1.into();
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let mut account = Account::empty(public_key.clone());

        account.balance = Amount(10); // Set the balance explicitly

        let account_before = account.clone();
        let mut accounts = HashMap::new();

        accounts.insert(public_key.clone(), account);

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Payment(PaymentDiff {
                public_key: public_key.clone(),
                amount: diff_amount,
                update_type: UpdateType::Deposit,
            })],
        };
        let ledger = Ledger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_after = ledger.accounts.get(&public_key).expect("account get");
        assert_eq!(
            account_after.balance,
            account_before.balance.add(&diff_amount)
        );
    }

    #[test]
    fn apply_diff_delegation_with_post_balance() {
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let delegate_key =
            PublicKey::new("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6");

        let mut account = Account::empty(public_key.clone());

        account.balance = Amount(20);

        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account.clone());

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Delegation(DelegationDiff {
                delegator: public_key.clone(),
                delegate: delegate_key.clone(),
            })],
        };
        let ledger = Ledger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_before = account;
        let account_after = ledger.accounts.get(&public_key).expect("account get");

        assert_eq!(account_before.balance, account_after.balance);
        assert_eq!(account_after.delegate, delegate_key);
    }
}
