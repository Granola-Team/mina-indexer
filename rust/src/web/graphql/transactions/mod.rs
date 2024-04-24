use super::{date_time_to_scalar, db, get_block_canonicity};
use crate::{
    block::BlockHash,
    command::{
        signed::{self, SignedCommand, SignedCommandWithData},
        store::CommandStore,
    },
    ledger::public_key::PublicKey,
    protocol::serialization_types::{
        staged_ledger_diff::{SignedCommandPayloadBody, StakeDelegation},
        version_bytes,
    },
    store::{
        user_commands_iterator, user_commands_iterator_signed_command,
        user_commands_iterator_txn_hash, IndexerStore,
    },
    web::graphql::{gen::TransactionQueryInput, DateTime},
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::sync::Arc;

const MINA_SCALE: u32 = 9;

#[derive(Default)]
pub struct TransactionsQueryRoot;

#[Object]
impl TransactionsQueryRoot {
    pub async fn transaction(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        let db = db(ctx);
        if let Some(hash) = query.hash {
            if signed::is_valid_tx_hash(&hash) {
                return Ok(db
                    .get_command_by_hash(&hash)?
                    .map(|cmd| txn_from_hash(cmd, db)));
            }
        } else {
            // no query filter => return the most recent transaction
            return Ok(user_commands_iterator(db, speedb::IteratorMode::End)
                .next()
                .and_then(|entry| {
                    let txn_hash = user_commands_iterator_txn_hash(&entry).unwrap();
                    db.get_command_by_hash(&txn_hash)
                        .unwrap()
                        .map(|cmd| txn_from_hash(cmd, db))
                }));
        }

        Ok(None)
    }

    pub async fn transactions(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
        #[graphql(default = 100)] limit: usize,
        sort_by: TransactionSortByInput,
    ) -> Result<Vec<Transaction>> {
        let db = db(ctx);
        let mut transactions: Vec<Transaction> = Vec::with_capacity(limit);
        let mode = match sort_by {
            TransactionSortByInput::BlockheightAsc | TransactionSortByInput::DatetimeAsc => {
                speedb::IteratorMode::Start
            }
            TransactionSortByInput::BlockheightDesc | TransactionSortByInput::DatetimeDesc => {
                speedb::IteratorMode::End
            }
        };

        // TODO bound query search space if given any inputs

        for entry in user_commands_iterator(db, mode) {
            if query.hash.is_some() && query.hash != user_commands_iterator_txn_hash(&entry).ok() {
                continue;
            }

            // Only add transactions that satisfy the input query
            let cmd = user_commands_iterator_signed_command(&entry)?;
            let transaction = txn_from_hash(cmd, db);

            if query.matches(&transaction) {
                transactions.push(transaction);
            };

            if transactions.len() == limit {
                break;
            }
        }

        Ok(transactions)
    }
}

fn txn_from_hash(cmd: SignedCommandWithData, db: &Arc<IndexerStore>) -> Transaction {
    let block_state_hash = cmd.state_hash.to_owned();
    let block_date_time = date_time_to_scalar(cmd.date_time as i64);
    Transaction::from_cmd(
        cmd,
        block_date_time,
        &block_state_hash,
        get_block_canonicity(db, &block_state_hash.0),
    )
}

pub fn decode_memo(bytes: Vec<u8>) -> anyhow::Result<String> {
    let encoded_memo = bs58::encode(bytes)
        .with_check_version(version_bytes::USER_COMMAND_MEMO)
        .into_string();
    Ok(encoded_memo)
}

pub fn nanomina_to_mina_f64(num: u64) -> f64 {
    let mut dec = Decimal::from(num);
    dec.set_scale(MINA_SCALE).unwrap();

    dec.to_f64().expect("converted to f64")
}

impl Transaction {
    pub fn from_cmd(
        cmd: SignedCommandWithData,
        block_date_time: DateTime,
        block_state_hash: &BlockHash,
        canonical: bool,
    ) -> Self {
        match cmd.command {
            SignedCommand(signed_cmd) => {
                let payload = signed_cmd.t.t.payload;
                let common = payload.t.t.common.t.t.t;
                let token = common.fee_token.t.t.t;
                let nonce = common.nonce.t.t;
                let fee = common.fee.t.t;
                let (sender, receiver, kind, token_id, amount) = {
                    match payload.t.t.body.t.t {
                        SignedCommandPayloadBody::PaymentPayload(payload) => (
                            payload.t.t.source_pk,
                            payload.t.t.receiver_pk,
                            "PAYMENT",
                            token,
                            payload.t.t.amount.t.t,
                        ),
                        SignedCommandPayloadBody::StakeDelegation(payload) => {
                            let StakeDelegation::SetDelegate {
                                delegator,
                                new_delegate,
                            } = payload.t;
                            (delegator, new_delegate, "STAKE_DELEGATION", token, 0)
                        }
                    }
                };

                let receiver = PublicKey::from(receiver).0;
                let memo = decode_memo(common.memo.t.0).expect("decoded memo");

                Self {
                    amount: nanomina_to_mina_f64(amount),
                    block: TransactionBlock {
                        date_time: block_date_time,
                        state_hash: block_state_hash.0.to_owned(),
                    },
                    block_height: cmd.blockchain_length as i64,
                    canonical,
                    fee: nanomina_to_mina_f64(fee),
                    from: Some(PublicKey::from(sender).0),
                    hash: cmd.tx_hash,
                    kind: Some(kind.to_string()),
                    memo,
                    nonce: nonce as i64,
                    receiver: TransactionReceiver {
                        public_key: receiver.to_owned(),
                    },
                    to: receiver,
                    token: Some(token_id as i64),
                }
            }
        }
    }
}

impl TransactionQueryInput {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let mut matches = true;
        if let Some(hash) = &self.hash {
            matches = matches && &transaction.hash == hash;
        }
        if let Some(fee) = self.fee {
            matches = matches && transaction.fee == fee;
        }
        if self.kind.is_some() {
            matches = matches && transaction.kind == self.kind;
        }
        if let Some(canonical) = self.canonical {
            matches = matches && transaction.canonical == canonical;
        }
        if self.from.is_some() {
            matches = matches && transaction.from == self.from;
        }
        if let Some(to) = &self.to {
            matches = matches && &transaction.to == to;
        }
        if let Some(memo) = &self.memo {
            matches = matches && &transaction.memo == memo;
        }
        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(transaction));
        }
        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(transaction));
            }
        }
        if let Some(__) = &self.date_time_gte {
            matches = matches && transaction.block.date_time >= *__;
        }
        if let Some(__) = &self.date_time_lte {
            matches = matches && transaction.block.date_time <= *__;
        }

        // TODO: implement matches for all the other optional vars
        matches
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum TransactionSortByInput {
    BlockheightAsc,
    BlockheightDesc,
    DatetimeAsc,
    DatetimeDesc,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct Transaction {
    pub amount: f64,
    pub block: TransactionBlock,
    pub block_height: i64,
    pub canonical: bool,
    pub fee: f64,
    pub from: Option<String>,
    pub hash: String,
    pub kind: Option<String>,
    pub memo: String,
    pub nonce: i64,
    pub receiver: TransactionReceiver,
    pub to: String,
    pub token: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct TransactionBlock {
    pub date_time: DateTime,
    pub state_hash: String,
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct TransactionReceiver {
    pub public_key: String,
}
