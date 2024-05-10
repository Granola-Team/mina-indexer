use crate::{
    block::{genesis::GenesisBlock, BlockHash},
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
};
use anyhow::bail;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

impl ToString for Amount {
    fn to_string(&self) -> String {
        nanomina_to_mina(self.0)
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Nonce(pub u32);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: Amount,
    pub nonce: Nonce,
    pub delegate: PublicKey,

    // optional
    pub token: Option<u32>,
    pub token_permissions: Option<TokenPermissions>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<BlockHash>,
    pub permissions: Option<Permissions>,
    pub timing: Option<Timing>,
    pub zkapp: Option<String>,

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub initial_minimum_balance: u64,
    pub cliff_time: u64,
    pub cliff_amount: u64,
    pub vesting_period: u64,
    pub vesting_increment: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPermissions {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChainHash(pub String);

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Username(pub String);

impl Account {
    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key: public_key.clone(),
            balance: Amount::default(),
            nonce: Nonce::default(),
            delegate: public_key,
            username: None,
            token: None,
            token_permissions: None,
            receipt_chain_hash: None,
            voting_for: None,
            permissions: None,
            timing: None,
            zkapp: None,
        }
    }

    pub fn set_username(&mut self, username: String) -> anyhow::Result<()> {
        const MAX_USERNAME_LENGTH: usize = 32;
        if username.len() <= MAX_USERNAME_LENGTH {
            self.username = Some(Username(username));
            return Ok(());
        }
        bail!(
            "Invalid username (length == {} > {MAX_USERNAME_LENGTH})",
            username.len()
        )
    }

    pub fn from_coinbase(pre: Self, amount: Amount) -> Self {
        Account {
            balance: if pre.balance.0 == 0 {
                amount.sub(&MAINNET_ACCOUNT_CREATION_FEE)
            } else {
                pre.balance.add(&amount)
            },
            ..pre
        }
    }

    pub fn from_payment(pre: Self, payment_diff: &PaymentDiff) -> Self {
        use super::UpdateType;

        match payment_diff.update_type {
            UpdateType::Credit => Self::from_credit(pre.clone(), payment_diff.amount),
            UpdateType::Debit(nonce) => {
                Account::from_debit(pre.clone(), payment_diff.amount, nonce).unwrap_or(pre.clone())
            }
        }
    }

    fn from_debit(pre: Self, amount: Amount, nonce: Option<u32>) -> Option<Self> {
        if amount > pre.balance {
            None
        } else {
            Some(Account {
                balance: pre.balance.sub(&amount),
                nonce: Nonce(nonce.unwrap_or(pre.nonce.0)),
                ..pre
            })
        }
    }

    fn from_credit(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance.add(&amount),
            nonce: Nonce(pre.nonce.0 + 1),
            ..pre
        }
    }

    pub fn from_delegation(pre: Self, delegate: PublicKey) -> Self {
        Account {
            nonce: Nonce(pre.nonce.0 + 1),
            delegate,
            ..pre
        }
    }

    pub fn from_failed_transaction(pre: Self, nonce: u32) -> Self {
        Account {
            nonce: Nonce(nonce + 1),
            ..pre
        }
    }
}

impl std::default::Default for Username {
    fn default() -> Self {
        Self("Unknown".to_string())
    }
}

impl From<GenesisBlock> for Account {
    fn from(value: GenesisBlock) -> Self {
        let block_creator = value.0.block_creator();
        Account {
            public_key: block_creator.clone(),
            balance: Amount(1000_u64),
            delegate: block_creator,
            nonce: Nonce::default(),
            username: None,
            token: None,
            token_permissions: None,
            receipt_chain_hash: None,
            voting_for: None,
            permissions: None,
            timing: None,
            zkapp: None,
        }
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.public_key.cmp(&other.public_key))
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.public_key.cmp(&other.public_key)
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
