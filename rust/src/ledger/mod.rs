pub mod account;
pub mod amount;
pub mod coinbase;
pub mod diff;
pub mod genesis;
pub mod hash;
pub mod nonce;
pub mod public_key;
pub mod staking;
pub mod store;
pub mod token;
pub mod username;

use crate::{
    block::precomputed::PrecomputedBlock,
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{
        account::Account,
        amount::Amount,
        diff::{account::AccountDiff, LedgerDiff},
        nonce::Nonce,
        public_key::PublicKey,
        token::TokenAddress,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

// re-export [hash::LedgerHash]
pub type LedgerHash = hash::LedgerHash;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub tokens: HashMap<TokenAddress, TokenLedger>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct TokenLedger {
    pub accounts: HashMap<PublicKey, Account>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NonGenesisLedger {
    pub ledger: TokenLedger,
}

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
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a token account from the corresponding token ledger
    pub fn get_account(&self, pk: &PublicKey, token: &TokenAddress) -> Option<&Account> {
        self.tokens
            .get(token)
            .and_then(|token_ledger| token_ledger.accounts.get(pk))
    }

    /// Get a mutable token account from the corresponding token ledger
    pub fn get_mut_account(
        &mut self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> Option<&mut Account> {
        self.tokens
            .get_mut(token)
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
            self._apply_account_diff(acct_diff)?;
        }
        Ok(())
    }

    /// Apply an account diff to a mutable ledger
    pub fn _apply_account_diff(&mut self, acct_diff: &AccountDiff) -> anyhow::Result<()> {
        let pk = acct_diff.public_key();
        let token = acct_diff.token_address();

        if let Some(account) = self
            .tokens
            .get_mut(&token)
            .and_then(|token_ledger| token_ledger.accounts.remove(&pk))
            .or(Some(Account::empty(pk.clone(), token.clone())))
        {
            self.insert_account(account.apply_account_diff(acct_diff), &token);
        }

        Ok(())
    }

    /// Unapply a ledger diff to a mutable ledger
    pub fn _unapply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            let pk = acct_diff.public_key();
            let token = acct_diff.token_address();

            if let Some(account_after) = self
                .tokens
                .get_mut(&token)
                .and_then(|token_ledger| token_ledger.accounts.remove(&pk))
                .or(Some(Account::empty(pk.clone(), token.clone())))
            {
                if let Some(account) = account_after
                    .unapply_account_diff(acct_diff, diff.new_pk_balances.contains_key(&pk))
                {
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
        Amount(
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
                    .sum()
                })
                .expect("MINA ledger exists"),
        )
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> anyhow::Result<Self> {
        let mut ledger = TokenLedger::new();
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

        Ok(Self::from_mina_ledger(ledger))
    }

    pub fn to_string_pretty(&self) -> String {
        let mut tokens = HashMap::new();

        for (token, token_ledger) in self.tokens.iter() {
            let mut accounts = HashMap::new();

            for (pk, acct) in token_ledger.accounts.iter() {
                accounts.insert(pk.to_address(), acct.clone().display());
            }

            tokens.insert(token.0.to_owned(), accounts);
        }

        serde_json::to_string_pretty(&tokens).unwrap()
    }
}

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

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (token, token_ledger) in self.tokens.iter() {
            writeln!(f, "{token}:")?;

            for (pk, acct) in token_ledger.accounts.iter() {
                writeln!(f, "  {pk} -> {}", acct.clone().display().balance.0)?;
            }
        }

        writeln!(f)
    }
}

impl TokenLedger {
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
            self._apply_account_diff(acct_diff)?;
        }
        Ok(())
    }

    /// Apply an account diff to a mutable ledger
    pub fn _apply_account_diff(&mut self, acct_diff: &AccountDiff) -> anyhow::Result<()> {
        let pk = acct_diff.public_key();
        let token = acct_diff.token_address();

        if let Some(account) = self
            .accounts
            .remove(&pk)
            .or(Some(Account::empty(pk.clone(), token)))
        {
            self.accounts
                .insert(pk, account.apply_account_diff(acct_diff));
        }
        Ok(())
    }

    /// Unapply a ledger diff to a mutable ledger
    pub fn _unapply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        for acct_diff in diff.account_diffs.iter().flatten() {
            let pk = acct_diff.public_key();
            let token = acct_diff.token_address();

            if let Some(account_after) = self
                .accounts
                .remove(&pk)
                .or(Some(Account::empty(pk.clone(), token)))
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
        let mut ledger = Self::new();
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

impl std::fmt::Display for TokenLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut accounts = HashMap::new();
        for (pk, acct) in self.accounts.iter() {
            accounts.insert(pk.to_address(), acct.clone().display());
        }
        write!(f, "{}", serde_json::to_string(&accounts).unwrap())
    }
}

impl FromStr for TokenLedger {
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

        Ok(Self { accounts })
    }
}

impl PartialEq for TokenLedger {
    fn eq(&self, other: &Self) -> bool {
        for pk in self.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
                    "[TokenLedger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk).cloned().map(Account::display),
                    other.accounts.get(pk).cloned().map(Account::display),
                );

                return false;
            }
        }

        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
                    "[TokenLedger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk).cloned().map(Account::display),
                    other.accounts.get(pk).cloned().map(Account::display),
                );

                return false;
            }
        }

        true
    }
}

impl Eq for TokenLedger {}

impl std::fmt::Debug for TokenLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (pk, acct) in &self.accounts {
            writeln!(f, "{pk} -> {}", acct.clone().display().balance.0)?;
        }
        writeln!(f)?;
        Ok(())
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
        hash::is_valid_ledger_hash,
        public_key::PublicKey,
        Amount, LedgerHash,
    };
    use crate::{
        block::BlockHash,
        constants::MINA_SCALE,
        ledger::{nonce::Nonce, token::TokenAddress, TokenLedger},
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
        let account_before = Account::empty(public_key.clone(), TokenAddress::default());

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
                    token: TokenAddress::default(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    amount,
                    public_key: PublicKey::default(),
                    update_type: UpdateType::Debit(None),
                    token: TokenAddress::default(),
                }),
            ]],
        };
        let ledger = TokenLedger { accounts }.apply_diff(&ledger_diff).unwrap();
        let account_after = ledger.accounts.get(&public_key).unwrap();
        assert_eq!(
            *account_after,
            Account {
                balance: account_before.balance + amount,
                ..account_before
            }
        );
    }

    #[test]
    fn apply_diff_delegation() {
        let prev_nonce = Nonce(42);
        let public_key = PublicKey::new("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");
        let delegate = PublicKey::new("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6");
        let account_before = Account::empty(public_key.clone(), TokenAddress::default());

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
        let ledger = TokenLedger { accounts }
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
