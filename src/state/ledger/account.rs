use mina_serialization_types::v1::PublicKeyV1;

use super::PublicKey;

#[derive(Debug, PartialEq, Eq, Clone)]
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
