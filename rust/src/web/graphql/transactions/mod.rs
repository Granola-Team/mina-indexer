use super::{date_time_to_scalar, db};
use crate::{
    block::BlockHash,
    canonicity::{store::CanonicityStore, Canonicity},
    command::{
        signed::{self, SignedCommand, SignedCommandWithData},
        store::CommandStore,
    },
    ledger::public_key::PublicKey,
    protocol::serialization_types::staged_ledger_diff::{
        SignedCommandPayloadBody, StakeDelegation,
    },
    store::{
        user_commands_iterator, user_commands_iterator_signed_command,
        user_commands_iterator_txn_hash, IndexerStore,
    },
    web::graphql::{gen::TransactionQueryInput, DateTime},
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(Default)]
pub struct TransactionsQueryRoot;

const NANO_F64: f64 = 1_000_000_000_f64;

#[Object]
impl TransactionsQueryRoot {
    pub async fn transaction(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        if let Some(hash) = query.hash {
            if signed::is_valid_tx_hash(&hash) {
                let db = db(ctx);
                return Ok(db
                    .get_command_by_hash(&hash)?
                    .map(|cmd| txn_from_hash(cmd, db)));
            }
        }
        Ok(None)
    }

    pub async fn transactions(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
        limit: Option<usize>,
        sort_by: TransactionSortByInput,
    ) -> Result<Vec<Option<Transaction>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);

        let mut transactions: Vec<Option<Transaction>> = Vec::new();

        // sort mode
        let mode = match sort_by {
            TransactionSortByInput::BlockheightAsc | TransactionSortByInput::DatetimeAsc => {
                speedb::IteratorMode::Start
            }
            TransactionSortByInput::BlockheightDesc | TransactionSortByInput::DatetimeDesc => {
                speedb::IteratorMode::End
            }
        };

        let iter = user_commands_iterator(db, mode).take(limit);

        for entry in iter {
            let txn_hash = user_commands_iterator_txn_hash(&entry)?;

            if let Some(query_txn_hash) = query.hash.to_owned() {
                if txn_hash != query_txn_hash {
                    continue;
                }
            }

            let cmd = user_commands_iterator_signed_command(&entry)?;

            let transaction = txn_from_hash(cmd, db);

            // Only add transactions that satisfy the input query
            if query.matches(&transaction) {
                transactions.push(Some(transaction));
            };
        }

        Ok(transactions)
    }
}

fn txn_from_hash(cmd: SignedCommandWithData, db: &Arc<IndexerStore>) -> Transaction {
    let block_state_hash = cmd.state_hash.to_owned();
    let block_date_time = date_time_to_scalar(cmd.date_time as i64);

    let canonical = match db
        .get_block_canonicity(&block_state_hash.to_owned())
        .unwrap()
    {
        Some(canonicity) => canonicity == Canonicity::Canonical,
        None => false,
    };

    Transaction::from_cmd(cmd, block_date_time, &block_state_hash, canonical)
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
                let mut memo = String::from_utf8(common.memo.t.0).unwrap();
                // ignore memos with nonsense unicode
                if memo.starts_with('\u{0001}') {
                    memo = String::new();
                };

                Self {
                    amount: amount as f64 / NANO_F64,
                    block: TransactionBlock {
                        date_time: block_date_time,
                        state_hash: block_state_hash.0.to_owned(),
                    },
                    block_height: cmd.blockchain_length as i64,
                    canonical,
                    fee: fee as f64 / NANO_F64,
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
