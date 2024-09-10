pub mod account;
pub mod coinbase;
pub mod diff;
pub mod genesis;
pub mod public_key;
pub mod staking;
pub mod store;
pub mod username;

use crate::{
    block::precomputed::PrecomputedBlock,
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{
        account::{Account, Amount, Nonce},
        diff::LedgerDiff,
        public_key::PublicKey,
    },
    protocol::serialization_types::{
        common::{Base58EncodableVersionedType, HashV1},
        version_bytes,
    },
};
use anyhow::bail;
use diff::account::AccountDiff;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub accounts: HashMap<PublicKey, Account>,
}

impl Ledger {
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_account(&mut self, pk: &PublicKey) -> Option<&Account> {
        self.accounts.get(pk)
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NonGenesisLedger {
    pub ledger: Ledger,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerHash(pub String);

impl LedgerHash {
    /// Prefx is one of {"jx", "jw", "jy", "jz"}
    pub const LEN: usize = 51;

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let hash = String::from_utf8(bytes)?;
        if is_valid_ledger_hash(&hash) {
            Ok(Self(hash))
        } else {
            bail!("Invalid ledger hash: {hash}")
        }
    }
}

impl std::str::FromStr for LedgerHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_valid_ledger_hash(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid ledger hash: {s}")
        }
    }
}

impl std::default::Default for LedgerHash {
    fn default() -> Self {
        Self("jxDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into())
    }
}

impl std::fmt::Display for LedgerHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
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

    /// Apply a ledger diff to a mutable ledger
    pub fn _apply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            self._apply_account_diff(acct_diff)?;
        }
        Ok(())
    }

    /// Apply an account diff to a mutable ledger
    pub fn _apply_account_diff(&mut self, acct_diff: &AccountDiff) -> anyhow::Result<()> {
        let pk = acct_diff.public_key();
        if let Some(account) = self
            .accounts
            .remove(&pk)
            .or(Some(Account::empty(pk.clone())))
        {
            self.accounts
                .insert(pk, account.clone().apply_account_diff(acct_diff));
        }
        Ok(())
    }

    /// Unapply a ledger diff to a mutable ledger
    pub fn _unapply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            let pk = acct_diff.public_key();
            if let Some(account_after) = self
                .accounts
                .remove(&pk)
                .or(Some(Account::empty(pk.clone())))
            {
                if let Some(account) = account_after
                    .unapply_account_diff(acct_diff, diff.new_pk_balances.contains_key(&pk))
                {
                    self.accounts.insert(pk, account);
                } else {
                    self.accounts.remove(&pk);
                }
            }
        }
        Ok(())
    }

    pub fn time_locked_amount(&self, curr_global_slot: u32) -> Amount {
        Amount(
            self.accounts
                .values()
                .filter_map(|acct| {
                    acct.timing
                        .as_ref()
                        .map(|_| acct.current_minimum_balance(curr_global_slot))
                })
                .sum(),
        )
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> anyhow::Result<Self> {
        let mut ledger = Ledger::new();
        for (pubkey, balance, nonce, delgation) in value {
            let pk = PublicKey::new(pubkey);
            let delegate = delgation.map(PublicKey::new).unwrap_or(pk.clone());
            ledger.accounts.insert(
                pk.clone(),
                Account {
                    delegate,
                    public_key: pk,
                    balance: balance.into(),
                    nonce: nonce.map(Nonce),
                    ..Default::default()
                },
            );
        }
        Ok(ledger)
    }

    pub fn to_string_pretty(&self) -> String {
        let mut accounts = HashMap::new();
        for (pk, acct) in self.accounts.iter() {
            accounts.insert(pk.to_address(), acct.clone().display());
        }
        serde_json::to_string_pretty(&accounts).unwrap()
    }
}

impl ToString for Ledger {
    fn to_string(&self) -> String {
        let mut accounts = HashMap::new();
        for (pk, acct) in self.accounts.iter() {
            accounts.insert(pk.to_address(), acct.clone().display());
        }
        serde_json::to_string(&accounts).unwrap()
    }
}

impl FromStr for Ledger {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let deser: HashMap<String, Account> = serde_json::from_str(s)?;
        let mut accounts = HashMap::new();
        for (pk, acct) in deser {
            accounts.insert(
                pk.into(),
                Account {
                    // compensate for display deduction
                    balance: acct.balance + MAINNET_ACCOUNT_CREATION_FEE,
                    ..acct.clone()
                },
            );
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
                    self.accounts.get(pk).cloned().map(Account::display),
                    other.accounts.get(pk).cloned().map(Account::display),
                );
                return false;
            }
        }
        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                debug!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk).cloned().map(Account::display),
                    other.accounts.get(pk).cloned().map(Account::display),
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
            writeln!(f, "{pk} -> {}", acct.clone().display().balance.0)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Add<Amount> for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<u64> for Amount {
    type Output = Amount;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<i64> for Amount {
    type Output = Amount;

    fn add(self, rhs: i64) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl Sub<Amount> for Amount {
    type Output = Amount;

    fn sub(self, other: Amount) -> Self::Output {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Sub<u64> for Amount {
    type Output = Amount;

    fn sub(self, other: u64) -> Self::Output {
        Self(self.0.saturating_sub(other))
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value)
    }
}

pub fn is_valid_ledger_hash(input: &str) -> bool {
    let mut chars = input.chars();
    let c0 = chars.next();
    let c1 = chars.next();
    input.len() == LedgerHash::LEN
        && c0 == Some('j')
        && (c1 == Some('w') || c1 == Some('x') || c1 == Some('y') || c1 == Some('z'))
}

#[cfg(test)]
mod tests {
    use super::{
        account::Account,
        diff::{
            account::{AccountDiff, DelegationDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        is_valid_ledger_hash,
        public_key::PublicKey,
        Ledger, LedgerHash,
    };
    use crate::{
        block::BlockHash,
        constants::MINA_SCALE,
        ledger::account::{Amount, Nonce},
    };
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn default_ledger_hash_is_valid_public_key() {
        assert!(is_valid_ledger_hash(&LedgerHash::default().0))
    }

    #[test]
    fn apply_diff_payment() {
        let amount = Amount(42 * MINA_SCALE);
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let account_before = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account_before.clone());

        let ledger_diff = LedgerDiff {
            blockchain_length: 0,
            state_hash: BlockHash::default(),
            new_pk_balances: BTreeMap::new(),
            new_coinbase_receiver: None,
            staged_ledger_hash: LedgerHash::default(),
            public_keys_seen: vec![],
            account_diffs: vec![vec![
                AccountDiff::Payment(PaymentDiff {
                    amount,
                    public_key: public_key.clone(),
                    update_type: UpdateType::Credit,
                }),
                AccountDiff::Payment(PaymentDiff {
                    amount,
                    public_key: public_key.clone(),
                    update_type: UpdateType::Debit(None),
                }),
            ]],
        };
        let ledger = Ledger { accounts }.apply_diff(&ledger_diff).unwrap();
        let account_after = ledger.accounts.get(&public_key).unwrap();
        assert_eq!(*account_after, account_before);
    }

    #[test]
    fn apply_diff_delegation() {
        let prev_nonce = Nonce(42);
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let delegate = PublicKey::new("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6");
        let account_before = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account_before.clone());

        let ledger_diff = LedgerDiff {
            blockchain_length: 0,
            state_hash: BlockHash::default(),
            new_pk_balances: BTreeMap::new(),
            new_coinbase_receiver: None,
            staged_ledger_hash: LedgerHash::default(),
            public_keys_seen: vec![],
            account_diffs: vec![vec![AccountDiff::Delegation(DelegationDiff {
                delegator: public_key.clone(),
                delegate: delegate.clone(),
                nonce: prev_nonce + 1,
            })]],
        };
        let ledger = Ledger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");
        let account_after = ledger.accounts.get(&public_key).expect("account get");
        assert_eq!(
            *account_after,
            Account {
                nonce: Some(prev_nonce + 1),
                delegate,
                ..account_before
            }
        );
    }
}
