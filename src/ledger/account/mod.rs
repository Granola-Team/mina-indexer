use crate::{constants::MAINNET_ACCOUNT_CREATION_FEE, ledger::public_key::PublicKey};
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

#[derive(PartialEq, Eq, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Nonce(pub u32);

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub delegate: PublicKey,
    pub balance: Amount,
    pub nonce: Nonce,
}

impl Account {
    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key: public_key.clone(),
            balance: Amount::default(),
            nonce: Nonce::default(),
            delegate: public_key,
        }
    }

    pub fn from_coinbase(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: if pre.balance.0 == 0 {
                amount.sub(&MAINNET_ACCOUNT_CREATION_FEE)
            } else {
                pre.balance.add(&amount)
            },
            nonce: pre.nonce,
            delegate: pre.delegate,
        }
    }

    pub fn from_deduction(pre: Self, amount: Amount) -> Option<Self> {
        if amount > pre.balance {
            None
        } else {
            Some(Account {
                public_key: pre.public_key.clone(),
                balance: pre.balance.sub(&amount),
                nonce: Nonce(pre.nonce.0 + 1),
                delegate: pre.delegate,
            })
        }
    }

    pub fn from_deposit(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance.add(&amount),
            nonce: Nonce(pre.nonce.0 + 1),
            delegate: pre.delegate,
        }
    }

    pub fn from_delegation(pre: Self, delegate: PublicKey) -> Self {
        Account {
            public_key: pre.public_key,
            balance: pre.balance,
            nonce: Nonce(pre.nonce.0 + 1),
            delegate,
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
