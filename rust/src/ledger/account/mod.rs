use super::username::Username;
use crate::{
    block::{genesis::GenesisBlock, BlockHash},
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
    pub nonce: Nonce,
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

    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key: public_key.clone(),
            delegate: public_key,
            ..Default::default()
        }
    }

    pub fn set_username(&mut self, username: Username) -> anyhow::Result<()> {
        self.username = Some(username);
        Ok(())
    }

    pub fn from_coinbase(pre: Self, amount: Amount) -> Self {
        Account {
            balance: pre.balance + amount,
            ..pre
        }
    }

    pub fn from_payment(pre: Self, payment_diff: &PaymentDiff) -> Self {
        use super::UpdateType::*;
        match payment_diff.update_type {
            Credit => Self::from_credit(pre.clone(), payment_diff.amount),
            Debit(nonce) => {
                Self::from_debit(pre.clone(), payment_diff.amount, nonce).unwrap_or(pre.clone())
            }
        }
    }

    fn from_debit(pre: Self, amount: Amount, nonce: Option<Nonce>) -> Option<Self> {
        if amount > pre.balance {
            None
        } else {
            Some(Account {
                balance: pre.balance - amount,
                nonce: nonce.unwrap_or(pre.nonce),
                ..pre
            })
        }
    }

    fn from_credit(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance + amount,
            ..pre
        }
    }

    pub fn from_delegation(pre: Self, delegate: PublicKey) -> Self {
        Account { delegate, ..pre }
    }

    pub fn from_failed_transaction(pre: Self, nonce: Nonce) -> Self {
        Account { nonce, ..pre }
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

const MINA_SCALE: u32 = 9;

pub fn nanomina_to_mina(num: u64) -> String {
    let mut dec = Decimal::from(num);
    dec.set_scale(MINA_SCALE).unwrap();
    let mut dec_str = dec.to_string();
    if dec_str.contains('.') {
        while dec_str.ends_with('0') {
            dec_str.pop();
        }
        if dec_str.ends_with('.') {
            dec_str.pop();
        }
    }
    dec_str
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
