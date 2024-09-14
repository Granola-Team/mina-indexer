use super::{
    diff::{
        account::{AccountDiff, UpdateType},
        LedgerDiff,
    },
    username::Username,
};
use crate::{
    block::{genesis::GenesisBlock, BlockHash},
    constants::{MAINNET_ACCOUNT_CREATION_FEE, MINA_SCALE},
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
    mina_blocks::v2::ZkappAccount,
    utility::functions::nanomina_to_mina,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    ops::{Add, Sub},
};

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

impl Amount {
    pub fn new(amount: u64) -> Self {
        Self(amount * MINA_SCALE)
    }
}

impl ToString for Amount {
    fn to_string(&self) -> String {
        nanomina_to_mina(self.0)
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

    fn sub(self, rhs: Amount) -> Self::Output {
        Self(self.0.checked_sub(rhs.0).unwrap())
    }
}

impl Sub<u64> for Amount {
    type Output = Amount;

    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0.checked_sub(rhs).unwrap())
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value)
    }
}

#[derive(
    PartialEq, Eq, Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, Ord, Hash,
)]
pub struct Nonce(pub u32);

impl Add<u32> for Nonce {
    type Output = Nonce;

    fn add(self, other: u32) -> Nonce {
        Self(self.0.saturating_add(other))
    }
}

impl Sub<u32> for Nonce {
    type Output = Nonce;

    fn sub(self, other: u32) -> Nonce {
        Self(self.0.saturating_sub(other))
    }
}

impl Add<i32> for Nonce {
    type Output = Nonce;

    fn add(self, other: i32) -> Nonce {
        let abs = other.unsigned_abs();
        if other > 0 {
            Self(self.0.saturating_add(abs))
        } else {
            Self(self.0.saturating_sub(abs))
        }
    }
}

impl From<u32> for Nonce {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<String> for Nonce {
    fn from(value: String) -> Self {
        Self(value.parse::<u32>().expect("nonce is u32"))
    }
}

impl From<Nonce> for serde_json::value::Number {
    fn from(value: Nonce) -> Self {
        Self::from(value.0)
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: Amount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<Nonce>,
    pub delegate: PublicKey,
    pub genesis_account: bool,

    // optional
    pub token: Option<u64>,
    pub token_permissions: Option<TokenPermissions>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<BlockHash>,
    pub permissions: Option<Permissions>,
    pub timing: Option<Timing>,

    // for zkapp accounts
    pub zkapp: Option<ZkappAccount>,

    // for mina search
    pub username: Option<Username>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    stake: bool,
    edit_state: Permission,
    send: Permission,
    set_delegate: Permission,
    set_permissions: Permission,
    set_verification_key: Permission,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    #[default]
    Signature,
    Proof,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub initial_minimum_balance: u64,
    pub cliff_time: u32,
    pub cliff_amount: u64,
    pub vesting_period: u32,
    pub vesting_increment: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPermissions {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChainHash(pub String);

impl Account {
    /// Display view of account
    pub fn display(self) -> Self {
        Self {
            balance: self.balance - MAINNET_ACCOUNT_CREATION_FEE,
            ..self
        }
    }

    /// Time-locked balance (subtracted from circulating supply)
    /// as per https://docs.minaprotocol.com/mina-protocol/time-locked-accounts
    pub fn current_minimum_balance(&self, curr_global_slot: u32) -> u64 {
        self.timing.as_ref().map_or(0, |t| {
            if curr_global_slot < t.cliff_time {
                t.initial_minimum_balance
            } else {
                t.initial_minimum_balance.saturating_sub(
                    ((curr_global_slot - t.cliff_time) / t.vesting_period) as u64
                        * t.vesting_increment,
                )
            }
        })
    }

    /// Creates a new empty account with the specified public key.
    /// This function initializes the account with the given public key and sets
    /// the delegate to the same public key. Other fields are set to their
    /// default values.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key to be associated with the new account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the specified public key and default
    /// values for other fields.
    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key: public_key.clone(),
            delegate: public_key,
            ..Default::default()
        }
    }

    /// Sets the username for the account
    pub fn set_username(&mut self, username: Username) -> anyhow::Result<()> {
        self.username = Some(username);
        Ok(())
    }

    /// Updates the account's balance based on applying a coinbase reward.
    /// This function takes the current account state (`pre`) and a reward
    /// amount (`amount`), and returns a new account state with the updated
    /// balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The coinbase reward amount to be added to the account's
    ///   balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    pub fn coinbase(self, amount: Amount) -> Self {
        Account {
            balance: self.balance + amount,
            ..self
        }
    }

    pub fn coinbase_unapply(self, amount: Amount) -> Self {
        Account {
            balance: self.balance - amount,
            ..self
        }
    }

    /// Updates the account's state based on applying a payment.
    /// This function handles both credit and debit updates.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `payment_diff` - The `PaymentDiff` containing the update type amount.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated state.
    pub fn payment(self, payment_diff: &PaymentDiff) -> Self {
        match payment_diff.update_type {
            UpdateType::Credit => self.credit(payment_diff.amount),
            UpdateType::Debit(nonce) => self.clone().debit(payment_diff.amount, nonce),
        }
    }

    /// Updates the account's state based on unapplying a payment.
    /// This function handles both credit and debit updates.
    ///
    /// # Arguments
    ///
    /// * `payment_diff` - The `PaymentDiff` containing the update type amount.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated state.
    pub fn payment_unapply(self, payment_diff: &PaymentDiff) -> Self {
        match payment_diff.update_type {
            UpdateType::Credit => Account {
                balance: self.balance - payment_diff.amount,
                ..self
            },
            UpdateType::Debit(nonce) => Self {
                balance: self.balance + payment_diff.amount,
                nonce: nonce.or(self.nonce).map(|n| n - 1),
                ..self
            },
        }
    }

    /// Updates the account's balance and nonce based on applying a debit.
    /// If the `nonce` is `None`, the update originates from an internal
    /// command.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to be debited from the account's balance.
    /// * `nonce` - The new nonce for the account. If `None`, the existing nonce
    ///   is retained.
    ///
    /// # Returns
    ///
    /// An `Option` containing the new `Account` state if the debit was
    /// successful, or `None` if the debit amount exceeds the current
    /// balance.
    fn debit(self, amount: Amount, nonce: Option<Nonce>) -> Self {
        Account {
            balance: self.balance - amount,
            nonce: nonce.or(self.nonce),
            ..self
        }
    }

    /// Updates the account's balance based on applying a credit.
    /// This function takes the current account state and returns a new
    /// account state with the updated balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to be credited to the account's balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    fn credit(self, amount: Amount) -> Self {
        Account {
            balance: self.balance + amount,
            ..self
        }
    }

    /// Updates the account's delegate and nonce based on a Stake Delegation
    /// transaction. This function takes the current account state (`pre`),
    /// a new delegate (`delegate`), and an updated nonce (`updated_nonce`),
    /// and returns a new account state with these changes.
    ///
    /// # Arguments
    ///
    /// * `delegate` - The new delegate public key for the account.
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated delegate and nonce.
    pub fn delegation(self, delegate: PublicKey, updated_nonce: Nonce) -> Self {
        Account {
            delegate,
            nonce: Some(updated_nonce),
            ..self
        }
    }

    /// Updates the account's nonce based on a failed transaction.
    /// This function takes the current account state (`pre`) and an updated
    /// nonce (`updated_nonce`), and returns a new account state with the
    /// updated nonce.
    ///
    /// # Arguments
    ///
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated nonce.
    pub fn failed_transaction(self, updated_nonce: Nonce) -> Self {
        Account {
            nonce: Some(updated_nonce),
            ..self
        }
    }

    pub fn failed_transaction_unapply(self, nonce: Option<Nonce>) -> Self {
        Account { nonce, ..self }
    }

    pub fn delegation_unapply(self, delegate: PublicKey, nonce: Option<Nonce>) -> Self {
        Account {
            delegate,
            nonce,
            ..self
        }
    }

    /// Apply an account diff to an account
    pub fn apply_account_diff(self, diff: &AccountDiff) -> Self {
        match diff {
            AccountDiff::Payment(payment_diff) => self.payment(payment_diff),
            AccountDiff::Delegation(delegation_diff) => {
                assert_eq!(self.public_key, delegation_diff.delegator);
                self.delegation(delegation_diff.delegate.clone(), delegation_diff.nonce)
            }
            AccountDiff::Coinbase(coinbase_diff) => self.coinbase(coinbase_diff.amount),
            AccountDiff::FeeTransfer(fee_transfer_diff) => self.payment(fee_transfer_diff),
            AccountDiff::FeeTransferViaCoinbase(fee_transfer_diff) => {
                self.payment(fee_transfer_diff)
            }
            AccountDiff::FailedTransactionNonce(failed_diff) => {
                self.failed_transaction(failed_diff.nonce)
            }
        }
    }

    /// Unapply an account diff to an account
    pub fn unapply_account_diff(self, diff: &AccountDiff, remove: bool) -> Option<Self> {
        if remove {
            return None;
        }

        Some(match diff {
            AccountDiff::Payment(payment_diff) => self.payment_unapply(payment_diff),
            AccountDiff::Delegation(delegation_diff) => {
                // TODO get previous delegate?
                self.delegation_unapply(
                    delegation_diff.delegate.clone(),
                    Some(delegation_diff.nonce),
                )
            }
            AccountDiff::Coinbase(coinbase_diff) => self.coinbase_unapply(coinbase_diff.amount),
            AccountDiff::FeeTransfer(fee_transfer_diff) => self.payment_unapply(fee_transfer_diff),
            AccountDiff::FeeTransferViaCoinbase(fee_transfer_diff) => {
                self.payment_unapply(fee_transfer_diff)
            }
            AccountDiff::FailedTransactionNonce(diff) => {
                self.failed_transaction_unapply(if diff.nonce.0 > 0 {
                    Some(diff.nonce - 1)
                } else {
                    None
                })
            }
        })
    }

    /// Apply a ledger diff to an account
    pub fn apply_ledger_diff(self, diff: &LedgerDiff) -> Self {
        let pk = self.public_key.clone();
        let mut acct = self;
        for acct_diff in diff.account_diffs.iter().flatten() {
            if acct_diff.public_key() == pk {
                acct = acct.apply_account_diff(acct_diff);
            }
        }
        acct
    }
}

impl From<GenesisBlock> for Account {
    fn from(value: GenesisBlock) -> Self {
        // magic mina
        let block_creator = value.0.block_creator();
        Account {
            public_key: block_creator.clone(),
            balance: Amount(1000_u64),
            delegate: block_creator,
            genesis_account: true,
            ..Default::default()
        }
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let balance_cmp = self.balance.cmp(&other.balance);
        if balance_cmp == std::cmp::Ordering::Equal {
            self.public_key.cmp(&other.public_key)
        } else {
            balance_cmp
        }
    }
}

/// Deduct account creation fee
impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let deducted = Account {
            balance: self.balance - MAINNET_ACCOUNT_CREATION_FEE,
            ..self.clone()
        };
        match serde_json::to_string_pretty(&deducted) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

/// Same as display
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

#[cfg(test)]
mod test {
    use super::{Account, Amount};

    #[test]
    fn test_account_display() -> anyhow::Result<()> {
        let ledger_account = Account {
            balance: Amount::new(100),
            ..Default::default()
        };
        let deduct_account = ledger_account.clone().display();

        // account display & debug => deduct "creation fee"
        assert_eq!(
            format!("{ledger_account}"),
            serde_json::to_string_pretty(&deduct_account)?
        );
        assert_eq!(
            format!("{ledger_account:?}"),
            serde_json::to_string_pretty(&deduct_account)?
        );

        // same account display & debug
        assert_eq!(format!("{ledger_account}"), format!("{ledger_account:?}"));
        Ok(())
    }
}
