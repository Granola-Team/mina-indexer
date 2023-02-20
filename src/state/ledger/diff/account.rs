use serde_json::Value;

use crate::state::ledger::{transaction::Transaction, PublicKey};

// add delegations later
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UpdateType {
    Deposit,
    Deduction,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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

    pub fn from_commands_fees(
        coinbase_receiver: PublicKey,
        commands: &[Value],
    ) -> Vec<AccountDiff> {
        commands
            .iter()
            .map(|command| {
                let payload_common = command
                    .as_object()?
                    .get("data")?
                    .as_array()?
                    .get(1)?
                    .as_object()?
                    .get("payload")?
                    .as_object()?
                    .get("common")?
                    .as_object()?;

                let fee = (payload_common.get("fee")?.as_f64()? * 1000000000.0) as u64;
                let fee_payer = payload_common.get("fee_payer_pk")?.as_str()?.to_string();

                Some(vec![
                    AccountDiff {
                        public_key: fee_payer,
                        amount: fee,
                        update_type: crate::state::ledger::diff::account::UpdateType::Deduction,
                    },
                    AccountDiff {
                        public_key: coinbase_receiver.clone(),
                        amount: fee,
                        update_type: crate::state::ledger::diff::account::UpdateType::Deposit,
                    },
                ])
            })
            .flatten()
            .flatten()
            .collect()
    }
}
