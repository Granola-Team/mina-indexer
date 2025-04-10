//! Token ledger representation

use super::account::TokenAccount;
use crate::{
    base::{amount::Amount, nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    block::precomputed::PrecomputedBlock,
    constants::{MAINNET_ACCOUNT_CREATION_FEE, MINA_TOKEN_ADDRESS},
    ledger::{
        account::Account,
        diff::{account::AccountDiff, LedgerDiff},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct TokenLedger {
    pub accounts: HashMap<PublicKey, Account>,
}

//////////
// impl //
//////////

impl TokenLedger {
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub fn get_account(&mut self, pk: &PublicKey) -> Option<&Account> {
        self.accounts.get(pk)
    }

    pub fn new() -> Self {
        Self {
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
            self._apply_account_diff(acct_diff, &diff.state_hash)?;
        }

        Ok(())
    }

    /// Apply an account diff to a mutable ledger
    pub fn _apply_account_diff(
        &mut self,
        acct_diff: &AccountDiff,
        state_hash: &StateHash,
    ) -> anyhow::Result<()> {
        let pk = acct_diff.public_key();
        let token = acct_diff.token();

        if let Some(account) = self
            .accounts
            .remove(&pk)
            .or_else(|| Some(Account::empty(pk.clone(), token, acct_diff.is_zkapp_diff())))
        {
            self.accounts
                .insert(pk, account.apply_account_diff(acct_diff, state_hash));
        }

        Ok(())
    }

    /// Unapply a ledger diff to a mutable ledger
    pub fn _unapply_diff(
        &mut self,
        state_hash: &StateHash,
        diff: &LedgerDiff,
    ) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            let pk = acct_diff.public_key();
            let token = acct_diff.token();

            if let Some(account_after) = self
                .accounts
                .remove(&pk)
                .or_else(|| Some(Account::empty(pk.clone(), token, acct_diff.is_zkapp_diff())))
            {
                if let Some(account) = account_after.unapply_account_diff(
                    acct_diff,
                    state_hash,
                    diff.new_pk_balances.contains_key(&pk),
                ) {
                    self.accounts.insert(pk, account);
                } else {
                    self.accounts.remove(&pk);
                }
            }
        }

        Ok(())
    }

    pub fn time_locked_amount(&self, curr_global_slot: u32) -> Amount {
        self.accounts
            .values()
            .filter_map(|acct| {
                acct.timing
                    .as_ref()
                    .map(|_| acct.current_minimum_balance(curr_global_slot))
            })
            .sum::<u64>()
            .into()
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> anyhow::Result<Self> {
        let mut ledger = Self::new();

        for (pubkey, balance, nonce, delgation) in value {
            let pk = PublicKey::new(pubkey)?;
            let delegate = delgation
                .map(PublicKey::new)
                .unwrap_or_else(|| Ok(pk.clone()))?;
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
            accounts.insert(
                pk.to_string(),
                acct.clone().deduct_mina_account_creation_fee(),
            );
        }

        serde_json::to_string_pretty(&accounts).unwrap()
    }
}

/////////////////
// conversions //
/////////////////

impl std::str::FromStr for TokenLedger {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let deser: HashMap<String, Account> = serde_json::from_str(s)?;
        let mut accounts = HashMap::new();

        for (pk, acct) in deser {
            accounts.insert(
                pk.into(),
                Account {
                    balance: if acct
                        .token
                        .as_ref()
                        .is_some_and(|t| t.0 != MINA_TOKEN_ADDRESS)
                    {
                        acct.balance
                    } else {
                        // compensate for MINA display deduction
                        acct.balance + MAINNET_ACCOUNT_CREATION_FEE
                    },
                    ..acct.clone()
                },
            );
        }

        Ok(Self { accounts })
    }
}

impl PartialEq for TokenLedger {
    fn eq(&self, other: &Self) -> bool {
        for pk in self.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
                    "[TokenLedger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts
                        .get(pk)
                        .cloned()
                        .map(Account::deduct_mina_account_creation_fee),
                    other
                        .accounts
                        .get(pk)
                        .cloned()
                        .map(Account::deduct_mina_account_creation_fee),
                );

                return false;
            }
        }

        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
                    "[TokenLedger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts
                        .get(pk)
                        .cloned()
                        .map(Account::deduct_mina_account_creation_fee),
                    other
                        .accounts
                        .get(pk)
                        .cloned()
                        .map(Account::deduct_mina_account_creation_fee),
                );

                return false;
            }
        }

        true
    }
}

impl Eq for TokenLedger {}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Display for TokenLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_pretty())
    }
}

impl std::fmt::Debug for TokenLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (pk, acct) in &self.accounts {
            writeln!(
                f,
                "{pk} -> {}",
                acct.clone().deduct_mina_account_creation_fee().balance.0
            )?;
        }

        writeln!(f)?;
        Ok(())
    }
}
