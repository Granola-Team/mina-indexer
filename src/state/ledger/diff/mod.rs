use crate::block_log::BlockLog;

use account::AccountDiff;

use super::{account::Account, transaction::Transaction};

pub mod account;

pub struct LedgerDiff {
    pub accounts_created: Vec<Account>,
    pub account_updates: Vec<AccountDiff>,
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
            AccountDiff::from_coinbase(coinbase_receiver.clone(), supercharge_coinbase);

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

        let mut account_updates_fees: Vec<AccountDiff> = commands
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
                        update_type: account::UpdateType::Deduction,
                    },
                    AccountDiff {
                        public_key: coinbase_receiver.clone(),
                        amount: fee,
                        update_type: account::UpdateType::Deposit,
                    },
                ])
            })
            .flatten() // filter out None results
            .flatten()
            .collect();

        let mut account_udpates_payments: Vec<AccountDiff> = commands
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

                let transaction = Transaction {
                    source: source_pk,
                    receiver: receiver_pk,
                    amount,
                };

                Some(AccountDiff::from_transaction(transaction))
            })
            .flatten()
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
