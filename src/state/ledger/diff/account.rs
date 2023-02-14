use crate::state::ledger::{transaction::Transaction, PublicKey};


pub enum UpdateType {
    Deposit,
    Deduction,
}

pub struct AccountDiff {
    pub public_key: String,
    pub amount: u64,
    pub update_type: UpdateType,
}

impl AccountDiff {
    pub fn from_transaction(transaction: Transaction) -> Vec<Self> {
        vec![
            AccountDiff {
                public_key: transaction.source,
                amount: transaction.amount,
                update_type: UpdateType::Deduction,
            },
            AccountDiff {
                public_key: transaction.receiver,
                amount: transaction.amount,
                update_type: UpdateType::Deposit,
            },
        ]
    }

    pub fn from_coinbase(coinbase_receiver: PublicKey, supercharge_coinbase: bool) -> Self {
        let amount = match supercharge_coinbase {
            true => 1440,
            false => 720,
        } * (1e9 as u64);
        AccountDiff {
            public_key: coinbase_receiver,
            amount,
            update_type: UpdateType::Deposit,
        }
    }
}