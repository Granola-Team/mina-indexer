use super::{date_time_to_scalar, db, get_block_canonicity, PK};
use crate::{
    block::store::BlockStore,
    command::{
        signed::{self, SignedCommand, SignedCommandWithData},
        store::CommandStore,
        CommandStatusData,
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
use std::sync::Arc;

#[derive(Default)]
pub struct TransactionsQueryRoot;

#[Object]
impl TransactionsQueryRoot {
    pub async fn transaction(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
    ) -> Result<Option<TransactionWithBlock>> {
        let db = db(ctx);
        if let Some(hash) = query.hash {
            if signed::is_valid_tx_hash(&hash) {
                return Ok(db
                    .get_command_by_hash(&hash)?
                    .map(|cmd| TransactionWithBlock::new(cmd, db)));
            }
        }

        Ok(None)
    }

    pub async fn transactions(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
        #[graphql(default = 100)] limit: usize,
        sort_by: TransactionSortByInput,
    ) -> Result<Vec<TransactionWithBlock>> {
        let db = db(ctx);
        let mut transactions: Vec<TransactionWithBlock> = Vec::with_capacity(limit);
        let mode = match sort_by {
            TransactionSortByInput::BlockheightAsc | TransactionSortByInput::DatetimeAsc => {
                speedb::IteratorMode::Start
            }
            TransactionSortByInput::BlockheightDesc | TransactionSortByInput::DatetimeDesc => {
                speedb::IteratorMode::End
            }
        };

        // block height query
        if let Some(block_height) = query.block_height {
            let mut transactions: Vec<TransactionWithBlock> = db
                .get_blocks_at_height(block_height as u32)?
                .into_iter()
                .flat_map(|b| SignedCommandWithData::from_precomputed(&b))
                .map(|cmd| TransactionWithBlock::new(cmd, db))
                .filter_map(|txn| if query.matches(&txn) { Some(txn) } else { None })
                .collect();
            reorder_asc(&mut transactions, sort_by);
            transactions.truncate(limit);
            return Ok(transactions);
        }

        // TODO bound query search space if given any inputs

        for entry in user_commands_iterator(db, mode) {
            if query.hash.is_some() && query.hash != user_commands_iterator_txn_hash(&entry).ok() {
                continue;
            }

            // Only add transactions that satisfy the input query
            let cmd = user_commands_iterator_signed_command(&entry)?;
            let transaction = TransactionWithBlock::new(cmd, db);

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

fn reorder_asc<T>(values: &mut [T], sort_by: TransactionSortByInput) {
    match sort_by {
        TransactionSortByInput::BlockheightAsc | TransactionSortByInput::DatetimeAsc => {
            values.reverse()
        }
        TransactionSortByInput::BlockheightDesc | TransactionSortByInput::DatetimeDesc => (),
    }
}

impl TransactionWithBlock {
    fn new(cmd: SignedCommandWithData, db: &Arc<IndexerStore>) -> TransactionWithBlock {
        let block_state_hash = cmd.state_hash.to_owned();
        let block_date_time = date_time_to_scalar(cmd.date_time as i64);
        TransactionWithBlock {
            transaction: Transaction::new(cmd, get_block_canonicity(db, &block_state_hash.0)),
            block: TransactionBlock {
                date_time: block_date_time,
                state_hash: block_state_hash.0.to_owned(),
            },
        }
    }
}

pub fn decode_memo(bytes: Vec<u8>) -> anyhow::Result<String> {
    let encoded_memo = bs58::encode(bytes)
        .with_check_version(version_bytes::USER_COMMAND_MEMO)
        .into_string();
    Ok(encoded_memo)
}

impl Transaction {
    pub fn new(cmd: SignedCommandWithData, canonical: bool) -> Self {
        let failure_reason = match cmd.status {
            CommandStatusData::Applied { .. } => "".to_owned(),
            CommandStatusData::Failed(failed_types, _) => failed_types
                .first()
                .map_or("".to_owned(), |f| f.to_string()),
        };
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
                    amount,
                    block_height: cmd.blockchain_length as i64,
                    canonical,
                    failure_reason,
                    fee,
                    from: PublicKey::from(sender).0,
                    hash: cmd.tx_hash,
                    kind: kind.to_string(),
                    memo,
                    nonce: nonce as i64,
                    receiver: PK {
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
    fn matches(&self, transaction_with_block: &TransactionWithBlock) -> bool {
        let mut matches = true;
        let transaction = transaction_with_block.transaction.clone();
        if let Some(hash) = &self.hash {
            matches = matches && &transaction.hash == hash;
        }
        if let Some(fee) = self.fee {
            matches = matches && transaction.fee == fee;
        }
        if let Some(ref kind) = &self.kind {
            matches = matches && &transaction.kind == kind;
        }
        if let Some(canonical) = self.canonical {
            matches = matches && transaction.canonical == canonical;
        }
        if let Some(ref from) = self.from {
            matches = matches && &transaction.from == from;
        }
        if let Some(to) = &self.to {
            matches = matches && &transaction.to == to;
        }
        if let Some(memo) = &self.memo {
            matches = matches && &transaction.memo == memo;
        }
        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(transaction_with_block));
        }
        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(transaction_with_block));
            }
        }
        if let Some(__) = &self.date_time_gte {
            matches = matches && transaction_with_block.block.date_time >= *__;
        }
        if let Some(__) = &self.date_time_lte {
            matches = matches && transaction_with_block.block.date_time <= *__;
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
    amount: u64,
    block_height: i64,
    canonical: bool,
    failure_reason: String,
    fee: u64,
    from: String,
    hash: String,
    kind: String,
    memo: String,
    nonce: i64,
    /// The receiver's public key
    receiver: PK,
    to: String,
    token: Option<i64>,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct TransactionWithBlock {
    block: TransactionBlock,
    #[graphql(flatten)]
    transaction: Transaction,
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
struct TransactionBlock {
    date_time: DateTime,
    state_hash: String,
}
