//! Mina ledger representation

pub mod account;
pub mod coinbase;
pub mod diff;
pub mod genesis;
pub mod hash;
pub mod staking;
pub mod store;
pub mod token;

use crate::{
    base::{amount::Amount, nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    block::{post_hardfork::account_accessed::AccountAccessed, precomputed::PrecomputedBlock},
    ledger::{
        account::Account,
        diff::{account::AccountDiff, LedgerDiff},
        token::{account::TokenAccount, TokenAddress},
    },
};
use anyhow::Context;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use token::ledger::TokenLedger;

// re-export [hash::LedgerHash]
pub type LedgerHash = hash::LedgerHash;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub tokens: HashMap<TokenAddress, TokenLedger>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NonGenesisLedger {
    pub ledger: TokenLedger,
}

//////////
// impl //
//////////

impl Ledger {
    /// Creates a full ledger from a MINA token ledger
    pub fn from_mina_ledger(ledger: TokenLedger) -> Self {
        Self {
            tokens: HashMap::from([(
                TokenAddress::default(),
                TokenLedger {
                    accounts: ledger.accounts,
                },
            )]),
        }
    }

    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.tokens.iter().fold(0, |acc, (_, token_ledger)| {
            acc + token_ledger.accounts.len()
        })
    }

    pub fn mina_account_len(&self) -> usize {
        self.get_token_ledger(&TokenAddress::default())
            .expect("MINA ledger")
            .len()
    }

    pub fn zkapp_account_len(&self) -> usize {
        self.tokens.iter().fold(0, |acc, (_, token_ledger)| {
            acc + token_ledger
                .accounts
                .values()
                .filter(|acct| acct.is_zkapp_account())
                .count()
        })
    }

    pub fn zkapp_mina_account_len(&self) -> usize {
        self.get_token_ledger(&TokenAddress::default())
            .expect("MINA ledger")
            .accounts
            .values()
            .filter(|acct| acct.is_zkapp_account())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get a token ledger from the corresponding ledger
    pub fn get_token_ledger(&self, token: &TokenAddress) -> Option<&TokenLedger> {
        self.tokens.get(token)
    }

    /// Get a mutable token ledger from the corresponding ledger
    pub fn get_mut_token_ledger(&mut self, token: &TokenAddress) -> Option<&mut TokenLedger> {
        self.tokens.get_mut(token)
    }

    /// Get a token account from the corresponding token ledger
    pub fn get_account(&self, pk: &PublicKey, token: &TokenAddress) -> Option<&Account> {
        self.get_token_ledger(token)
            .and_then(|token_ledger| token_ledger.accounts.get(pk))
    }

    /// Get a mutable token account from the corresponding token ledger
    pub fn get_mut_account(
        &mut self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> Option<&mut Account> {
        self.get_mut_token_ledger(token)
            .and_then(|token_ledger| token_ledger.accounts.get_mut(pk))
    }

    /// Insert a token account into the corresponding token ledger
    pub fn insert_account(&mut self, account: Account, token: &TokenAddress) {
        if let Some(token_ledger) = self.tokens.get_mut(token) {
            // insert account into existing token ledger
            token_ledger
                .accounts
                .insert(account.public_key.to_owned(), account);
        } else {
            // create new token ledger
            let mut token_ledger = TokenLedger::new();

            token_ledger
                .accounts
                .insert(account.public_key.to_owned(), account);
            self.tokens.insert(token.to_owned(), token_ledger);
        }
    }

    /// Apply the ledger diff from a PCB
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
            .tokens
            .get_mut(&token)
            .and_then(|token_ledger| token_ledger.accounts.remove(&pk))
            .or_else(|| {
                Some(Account::empty(
                    pk,
                    token.to_owned(),
                    acct_diff.creation_fee_paid(),
                ))
            })
        {
            self.insert_account(account.apply_account_diff(acct_diff, state_hash), &token);
        }

        Ok(())
    }

    pub fn _apply_diff_check(
        &mut self,
        diff: &LedgerDiff,
        accounts_accessed: &[AccountAccessed],
    ) -> anyhow::Result<()> {
        info!("Checking {}", diff.summary());
        self._apply_diff(diff)?;

        // check accessed accounts
        for accessed_account in accounts_accessed {
            let pk = &accessed_account.account.public_key;
            let token = accessed_account.account.token.clone().unwrap_or_default();

            // check ledger account
            let account = self
                .get_account(pk, &token)
                .cloned()
                .with_context(|| format!("account {} token {}", pk, token))
                .unwrap();
            let account = account.deduct_mina_account_creation_fee();

            accessed_account.assert_eq_account(
                &account,
                &format!(
                    "Error applying {}\n     pk: {}\n  token: {}",
                    diff.summary(),
                    pk,
                    token
                ),
            );
        }

        Ok(())
    }

    /// Unapply a ledger diff to a mutable ledger
    pub fn _unapply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            let pk = acct_diff.public_key();
            let token = acct_diff.token();

            if let Some(account_after) = self
                .tokens
                .get_mut(&token)
                .and_then(|token_ledger| token_ledger.accounts.remove(&pk))
                .or_else(|| {
                    Some(Account::empty(
                        pk.to_owned(),
                        token.to_owned(),
                        acct_diff.creation_fee_paid(),
                    ))
                })
            {
                if let Some(account) = account_after.unapply_account_diff(
                    acct_diff,
                    &diff.state_hash,
                    diff.new_pk_balances.contains_key(&pk),
                ) {
                    self.tokens
                        .get_mut(&token)
                        .and_then(|token_ledger| token_ledger.accounts.insert(pk, account));
                } else {
                    self.tokens
                        .get_mut(&token)
                        .and_then(|token_ledger| token_ledger.accounts.remove(&pk));
                }
            }
        }
        Ok(())
    }

    pub fn time_locked_amount(&self, curr_global_slot: u32) -> Amount {
        self.tokens
            .get(&TokenAddress::default())
            .map(|mina_ledger| {
                {
                    mina_ledger.accounts.values().filter_map(|acct| {
                        acct.timing
                            .as_ref()
                            .map(|_| acct.current_minimum_balance(curr_global_slot))
                    })
                }
                .sum::<u64>()
            })
            .expect("MINA ledger exists")
            .into()
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> anyhow::Result<Self> {
        let mut ledger = TokenLedger::new();

        for (pubkey, balance, nonce, delgation) in value {
            let pk = PublicKey::new(pubkey)?;
            let delegate = delgation.map(PublicKey::new).unwrap_or(Ok(pk.clone()))?;

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

        Ok(Self::from_mina_ledger(ledger))
    }

    pub fn to_string_pretty(&self) -> String {
        let mut tokens = HashMap::new();

        for (token, token_ledger) in self.tokens.iter() {
            let mut accounts = HashMap::new();

            for (pk, acct) in token_ledger.accounts.iter() {
                accounts.insert(
                    pk.to_string(),
                    acct.clone().deduct_mina_account_creation_fee(),
                );
            }

            tokens.insert(token.to_string(), accounts);
        }

        serde_json::to_string_pretty(&tokens).unwrap()
    }
}

/////////////////
// comparisons //
/////////////////

impl PartialEq for Ledger {
    fn eq(&self, other: &Self) -> bool {
        for token in self.tokens.keys() {
            if self.tokens.get(token) != other.tokens.get(token) {
                println!("[Ledger.eq mismatch] token {token}");

                return false;
            }
        }

        for token in other.tokens.keys() {
            if self.tokens.get(token) != other.tokens.get(token) {
                println!("[Ledger.eq mismatch] token {token}");

                return false;
            }
        }

        true
    }
}

impl Eq for Ledger {}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Display for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.to_string_pretty())
    }
}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (token, token_ledger) in self.tokens.iter() {
            writeln!(f, "{token}:")?;

            for (pk, acct) in token_ledger.accounts.iter() {
                writeln!(
                    f,
                    "  {pk} -> {}",
                    acct.clone().deduct_mina_account_creation_fee().balance
                )?;
            }
        }

        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        account::Account,
        diff::{
            account::{AccountDiff, DelegationDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        genesis::GenesisLedger,
        Amount, Ledger, LedgerHash,
    };
    use crate::{
        base::{nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
        block::post_hardfork::account_accessed::AccountAccessed,
        command::TxnHash,
        constants::MINA_SCALE,
        ledger::{token::TokenAddress, TokenLedger},
        utility::functions::nanomina_to_mina,
    };
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn ledger_balance_display() -> anyhow::Result<()> {
        let ledger: Ledger = GenesisLedger::new_v1()?.into();
        let ledger = ledger.get_token_ledger(&TokenAddress::default()).unwrap();

        // make the serialized ledger
        let mut ser_ledger = HashMap::new();
        for (pk, account) in ledger.accounts.iter() {
            ser_ledger.insert(
                pk.to_string(),
                account.clone().deduct_mina_account_creation_fee(),
            );
        }

        // check balance serialization
        for (_, account) in ser_ledger.iter() {
            let expect = format!("{:?}", nanomina_to_mina(account.balance.0));

            assert!(expect.chars().filter(|c| *c == '.').count() <= 1);
            assert_eq!(expect, serde_json::to_string(&account.balance)?);
        }

        Ok(())
    }

    #[test]
    fn default_ledger_hash_is_valid_public_key() {
        assert!(LedgerHash::is_valid(&LedgerHash::default().0))
    }

    #[test]
    fn apply_diff_payment() -> anyhow::Result<()> {
        let amount = Amount(42 * MINA_SCALE);
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")?;
        let account_before = Account::empty(public_key.clone(), TokenAddress::default(), false);

        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account_before.clone());
        accounts.insert(
            PublicKey::default(),
            Account {
                public_key: PublicKey::default(),
                delegate: PublicKey::default(),
                balance: Amount(1_000_000_000_000_000_000u64),
                ..Default::default()
            },
        );

        let account_accessed = AccountAccessed {
            index: 0,
            account: Account {
                balance: account_before.balance + amount,
                ..account_before.clone()
            },
        };
        let ledger_diff = LedgerDiff {
            blockchain_length: 0,
            state_hash: StateHash::default(),
            new_pk_balances: BTreeMap::new(),
            new_coinbase_receiver: None,
            staged_ledger_hash: LedgerHash::default(),
            public_keys_seen: vec![],
            account_diffs: vec![vec![
                AccountDiff::Payment(PaymentDiff {
                    amount,
                    public_key: public_key.clone(),
                    update_type: UpdateType::Credit,
                    txn_hash: None,
                    token: None,
                }),
                AccountDiff::Payment(PaymentDiff {
                    amount,
                    public_key: PublicKey::default(),
                    update_type: UpdateType::Debit(None),
                    txn_hash: None,
                    token: None,
                }),
            ]],
            token_diffs: vec![],
            accounts_created: vec![],
        };

        let ledger = TokenLedger { accounts }.apply_diff(&ledger_diff).unwrap();
        let account_after = ledger.accounts.get(&public_key).unwrap();

        account_accessed.assert_eq_account(account_after, "");
        Ok(())
    }

    #[test]
    fn apply_diff_delegation() -> anyhow::Result<()> {
        let prev_nonce = Nonce(42);
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")?;
        let delegate = PublicKey::new("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6")?;
        let account_before = Account::empty(public_key.clone(), TokenAddress::default(), true);

        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account_before.clone());

        let account_accessed = AccountAccessed {
            index: 0,
            account: Account {
                nonce: Some(prev_nonce + 1),
                delegate: delegate.clone(),
                ..account_before
            },
        };
        let ledger_diff = LedgerDiff {
            blockchain_length: 0,
            state_hash: StateHash::default(),
            new_pk_balances: BTreeMap::new(),
            new_coinbase_receiver: None,
            staged_ledger_hash: LedgerHash::default(),
            public_keys_seen: vec![],
            account_diffs: vec![vec![AccountDiff::Delegation(DelegationDiff {
                delegator: public_key.clone(),
                delegate,
                nonce: prev_nonce + 1,
                txn_hash: TxnHash::default(),
            })]],
            token_diffs: vec![],
            accounts_created: vec![],
        };

        let ledger = TokenLedger { accounts }
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");
        let account_after = ledger.accounts.get(&public_key).expect("account get");

        account_accessed.assert_eq_account(account_after, "");
        Ok(())
    }
}
