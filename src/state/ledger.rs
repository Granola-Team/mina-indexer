use std::collections::HashMap;

use crate::block_log::BlockLog;

pub type PublicKey = String;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Account {
    public_key: String,
    balance: u64,
}

pub struct AccountUpdate {
    from: Option<PublicKey>,
    to: PublicKey,
    amount: u64,
}

impl AccountUpdate {
    pub fn from_coinbase(coinbase_receiver: PublicKey, supercharge_coinbase: bool) -> Self {
        let amount = match supercharge_coinbase {
            true => 1440,
            false => 720,
        } * (1e9 as u64);
        AccountUpdate {
            from: None,
            to: coinbase_receiver,
            amount,
        }
    }
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

#[derive(Default)]
pub struct Ledger {
    accounts: HashMap<PublicKey, Account>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    // should this be a mutable update or immutable?
    pub fn apply_diff(&mut self, diff: LedgerDiff) -> bool {
        diff.accounts_created.into_iter().for_each(|account| {
            if let None = self.accounts.get(&account.public_key) {
                self.accounts.insert(account.public_key.clone(), account);
            }
        });

        let mut success = true; // change this to a Result<(), CustomError> so we can know exactly where this failed
        diff.account_updates.into_iter().for_each(|update| {
            if let Some(to_pre) = self.accounts.remove(&update.to) {
                if let Some(from) = update.from {
                    if let Some(from_pre) = self.accounts.remove(&from) {
                        let from_post = Account::from_deduction(from_pre, update.amount);
                        self.accounts.insert(from, from_post);
                    }
                }
                let to_post = Account::from_deposit(to_pre, update.amount);
                self.accounts.insert(update.to, to_post);
            } else {
                success = false;
            }
        });
        success
    }
}

pub struct LedgerDiff {
    accounts_created: Vec<Account>,
    account_updates: Vec<AccountUpdate>,
}

impl LedgerDiff {
    // horrendous deserialization function. there has to be a better way. maybe `deserr`?
    pub fn fom_block_log(block_log: BlockLog) -> Option<Self> {
        let mut accounts_created = Vec::new();
        let consensus_state = block_log
            .json
            .as_object()?
            .get("protocol_state")?
            .as_object()?
            .get("body")?
            .as_object()?
            .get("consensus_state")?
            .as_object()?;

        let block_stake_winner = consensus_state
            .get("block_stake_winner")?
            .as_str()?
            .to_string();
        let block_creator = consensus_state.get("block_creator")?.as_str()?.to_string();
        let coinbase_receiver = consensus_state
            .get("coinbase_receiver")?
            .as_str()?
            .to_string();

        let supercharge_coinbase = consensus_state.get("supercharge_coinbase")?.as_bool()?;

        let coinbase_update =
            AccountUpdate::from_coinbase(coinbase_receiver.clone(), supercharge_coinbase);

        accounts_created.append(
            &mut vec![block_stake_winner, block_creator, coinbase_receiver.clone()]
                .into_iter()
                .map(|public_key| Account::empty(public_key))
                .collect(),
        );

        let commands = block_log
            .json
            .as_object()?
            .get("staged_ledger_diff")?
            .as_object()?
            .get("diff")?
            .as_array()?
            .get(0)?
            .as_object()?
            .get("commands")?
            .as_array()?;

        let mut account_updates_fees: Vec<AccountUpdate> = commands
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

                Some(AccountUpdate {
                    from: Some(fee_payer),
                    to: coinbase_receiver.clone(),
                    amount: fee,
                })
            })
            .flatten() // filter out None results
            .collect();

        let mut account_udpates_payments: Vec<AccountUpdate> = commands
            .iter()
            .map(|command| {
                let payload_body = command
                    .as_object()?
                    .get("data")?
                    .as_array()?
                    .get(1)?
                    .as_object()?
                    .get("payload")?
                    .as_object()?
                    .get("body")?
                    .as_array()?
                    .get(1)?
                    .as_object()?;

                let source_pk = payload_body.get("source_pk")?.as_str()?.to_string();

                let receiver_pk = payload_body.get("receiver_pk")?.as_str()?.to_string();

                let amount = payload_body.get("amount")?.as_u64()?;

                Some(AccountUpdate {
                    from: Some(source_pk),
                    to: receiver_pk,
                    amount,
                })
            })
            .flatten()
            .collect();

        let mut account_updates = Vec::new();
        account_updates.append(&mut account_updates_fees);
        account_updates.append(&mut account_udpates_payments);
        account_updates.push(coinbase_update);

        Some(LedgerDiff {
            accounts_created,
            account_updates,
        })
    }

    // potentially make immutable later on
    pub fn append(&mut self, other: Self) {
        other.accounts_created.into_iter().for_each(|account| {
            self.accounts_created.push(account);
        });

        other.account_updates.into_iter().for_each(|update| {
            self.account_updates.push(update);
        });
    }
}
