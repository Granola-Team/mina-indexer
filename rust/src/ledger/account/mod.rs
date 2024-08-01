use super::username::Username;
use crate::{
    block::{genesis::GenesisBlock, BlockHash},
    constants::MINA_SCALE,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
    mina_blocks::v2::ZkappAccount,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::{
    fmt::{self, Display},
    ops::{Add, Sub},
};

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

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

impl Sub<Amount> for Amount {
    type Output = Amount;

    fn sub(self, rhs: Amount) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[derive(
    PartialEq, Eq, Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, Ord, Hash,
)]
pub struct Nonce(pub u32);

impl Add<i32> for Nonce {
    type Output = Nonce;

    fn add(self, other: i32) -> Nonce {
        Nonce(self.0.wrapping_add(other as u32))
    }
}

impl From<String> for Nonce {
    fn from(s: String) -> Self {
        Nonce(s.parse::<u32>().expect("nonce is u32"))
    }
}

impl From<Nonce> for serde_json::value::Number {
    fn from(n: Nonce) -> Self {
        Number::from(n.0)
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

    /// Sets the username for the account.
    /// This function updates the account's username to the specified value.
    ///
    /// # Arguments
    ///
    /// * `username` - The new username to be set for the account.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result<()>` indicating success or failure.
    pub fn set_username(&mut self, username: Username) -> anyhow::Result<()> {
        self.username = Some(username);
        Ok(())
    }

    /// Updates the account's balance based on a coinbase reward.
    /// This function takes the current account state (`pre`) and a reward
    /// amount (`amount`), and returns a new account state with the updated
    /// balance.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `amount` - The coinbase reward amount to be added to the account's
    ///   balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    pub fn from_coinbase(pre: Self, amount: Amount) -> Self {
        Account {
            balance: pre.balance + amount,
            ..pre
        }
    }

    /// Updates the account's state based on the This function handles both
    /// credit and debit updates.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `payment_diff` - The `PaymentDiff` containing the update type amount.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated state.
    pub fn from_payment(pre: Self, payment_diff: &PaymentDiff) -> Self {
        use super::UpdateType::*;
        match payment_diff.update_type {
            Credit => Self::from_credit(pre.clone(), payment_diff.amount),
            Debit(nonce) => {
                Self::from_debit(pre.clone(), payment_diff.amount, nonce).unwrap_or(pre.clone())
            }
        }
    }

    /// Updates the account's balance and nonce based on a User Command.
    /// If the `nonce` is `None`, the update originates from an internal
    /// command.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `amount` - The amount to be debited from the account's balance.
    /// * `nonce` - The new nonce for the account. If `None`, the existing nonce
    ///   is retained.
    ///
    /// # Returns
    ///
    /// An `Option` containing the new `Account` state if the debit was
    /// successful, or `None` if the debit amount exceeds the current
    /// balance.
    fn from_debit(pre: Self, amount: Amount, nonce: Option<Nonce>) -> Option<Self> {
        // TODO: Convert this to use an assert. Note: The assertion will fail
        // when it originating from a dangling branch.
        if amount > pre.balance {
            None
        } else {
            Some(Account {
                balance: pre.balance - amount,
                nonce: nonce.or(pre.nonce),
                ..pre
            })
        }
    }

    /// Updates the account's balance by adding the specified `amount`.
    /// This function takes the current account state (`pre`) and returns a new
    /// account state with the updated balance.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `amount` - The amount to be credited to the account's balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    fn from_credit(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance + amount,
            ..pre
        }
    }

    /// Updates the account's delegate and nonce based on a Stake Delegation
    /// transaction. This function takes the current account state (`pre`),
    /// a new delegate (`delegate`), and an updated nonce (`updated_nonce`),
    /// and returns a new account state with these changes.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `delegate` - The new delegate public key for the account.
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated delegate and nonce.
    pub fn from_delegation(pre: Self, delegate: PublicKey, updated_nonce: Nonce) -> Self {
        Account {
            delegate,
            nonce: Some(updated_nonce),
            ..pre
        }
    }

    /// Updates the account's nonce based on a failed transaction.
    /// This function takes the current account state (`pre`) and an updated
    /// nonce (`updated_nonce`), and returns a new account state with the
    /// updated nonce.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated nonce.
    pub fn from_failed_transaction(pre: Self, updated_nonce: Nonce) -> Self {
        Account {
            nonce: Some(updated_nonce),
            ..pre
        }
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

/// Converts Nanomina to Mina, strips any trailing zeros, and converts -0 to 0.
/// This function takes a value in Nanomina, converts it to Mina by adjusting
/// the scale, normalizes the decimal representation to remove trailing zeros,
/// and converts any `-0` representation to `0`.
///
/// # Arguments
///
/// * `nanomina` - The amount in Nanomina to be converted.
///
/// # Returns
///
/// A `String` representing the value in Mina with trailing zeros removed.
pub fn nanomina_to_mina(nanomina: u64) -> String {
    let mut dec = Decimal::from(nanomina);
    dec.set_scale(MINA_SCALE as u32).unwrap();
    dec.normalize().to_string()
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

/// Same as display
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod test {
    use crate::ledger::account::nanomina_to_mina;

    #[test]
    fn test_nanomina_to_mina_conversion() {
        let actual = 1_000_000_001;
        let val = nanomina_to_mina(actual);
        assert_eq!("1.000000001", val);

        let actual = 1_000_000_000;
        let val = nanomina_to_mina(actual);
        assert_eq!("1", val);
    }
}
