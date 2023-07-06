use serde::{Deserialize, Serialize};

use super::PublicKey;

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

#[derive(PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub struct Nonce(pub u32);

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: Amount,
    pub nonce: Nonce,
    pub delegate: Option<PublicKey>,
}

impl Account {
    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key,
            balance: Amount::default(),
            nonce: Nonce::default(),
            delegate: None,
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
            delegate: Some(delegate),
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
        let pk = self.public_key.to_address();
        let delegate = self
            .delegate
            .as_ref()
            .map(|pk| pk.to_address())
            .unwrap_or(pk.clone());
        writeln!(f, "{{")?;
        writeln!(f, "  pk:       {pk}")?;
        writeln!(f, "  balance:  {}", self.balance.0)?;
        writeln!(f, "  nonce:    {}", self.nonce.0)?;
        writeln!(f, "  delegate: {delegate}")?;
        writeln!(f, "}}")
    }
}
