#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Account {
    pub public_key: String,
    pub balance: u64,
}

impl Account {
    pub fn empty(public_key: String) -> Self {
        Account {
            public_key,
            balance: 0,
        }
    }

    pub fn from_deduction(pre: Self, amount: u64) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance - amount,
        }
    }

    pub fn from_deposit(pre: Self, amount: u64) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance + amount,
        }
    }
}
