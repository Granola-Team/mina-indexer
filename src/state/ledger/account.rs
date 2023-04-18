use mina_serialization_types::v1::PublicKeyV1;
use serde::{Deserialize, Serialize};

use super::PublicKey;

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: u64,
    pub delegate: Option<PublicKey>,
}

impl Account {
    pub fn empty(public_key: PublicKeyV1) -> Self {
        Account {
            public_key: public_key.into(),
            balance: 0,
            delegate: None,
        }
    }

    pub fn from_deduction(pre: Self, amount: u64) -> Self {
        if amount > pre.balance {
            return Account {
                public_key: pre.public_key.clone(),
                balance: 0,
                delegate: pre.delegate,
            };
        }
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance - amount,
            delegate: pre.delegate,
        }
    }

    pub fn from_deposit(pre: Self, amount: u64) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance + amount,
            delegate: pre.delegate,
        }
    }

    pub fn from_delegation(pre: Self, delegate: PublicKeyV1) -> Self {
        Account {
            public_key: pre.public_key,
            balance: pre.balance,
            delegate: Some(delegate.into()),
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

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Account {{ {:?}, {}, {:?} }}",
            self.public_key, self.balance, self.delegate
        )
    }
}
