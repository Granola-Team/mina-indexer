use super::{date_time_to_scalar, db, get_block_canonicity, PK};
use crate::{
    block::store::BlockStore,
    command::{
        decode_memo,
        signed::{self, SignedCommand, SignedCommandWithData},
        store::{
            user_commands_iterator_state_hash, user_commands_iterator_txn_hash, UserCommandStore,
        },
        CommandStatusData,
    },
    ledger::public_key::PublicKey,
    protocol::serialization_types::staged_ledger_diff::{
        SignedCommandPayloadBody, StakeDelegation,
    },
    store::{pk_of_key, pk_txn_sort_key_prefix, to_be_bytes, txn_hash_of_key, IndexerStore},
    web::graphql::{gen::TransactionQueryInput, DateTime},
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use speedb::{Direction, IteratorMode};
use std::sync::Arc;

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
        let epoch_num_user_commands = db.get_user_commands_epoch_count(None)?;
        let total_num_user_commands = db.get_user_commands_total_count()?;

        if let Some(hash) = query.hash {
            if signed::is_valid_tx_hash(&hash) {
                return Ok(db.get_user_command(&hash, 0)?.map(|cmd| {
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands)
                }));
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
    ) -> Result<Vec<Transaction>> {
        let db = db(ctx);
        let epoch_num_user_commands = db.get_user_commands_epoch_count(None)?;
        let total_num_user_commands = db.get_user_commands_total_count()?;

        // transaction filtered by state hash
        if let Some(state_hash) = query
            .block
            .as_ref()
            .and_then(|block| block.state_hash.clone())
        {
            let mut transactions: Vec<Transaction> = db
                .get_block(&state_hash.into())?
                .into_iter()
                .flat_map(|b| SignedCommandWithData::from_precomputed(&b))
                .map(|cmd| {
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands)
                })
                .filter(|txn| query.matches(txn))
                .collect();
            reorder_asc(&mut transactions, sort_by);
            transactions.truncate(limit);
            return Ok(transactions);
        }
        // block height query
        if let Some(block_height) = query.block_height {
            let mut transactions: Vec<Transaction> = db
                .get_blocks_at_height(block_height)?
                .into_iter()
                .flat_map(|b| SignedCommandWithData::from_precomputed(&b))
                .map(|cmd| {
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands)
                })
                .filter(|txn| query.matches(txn))
                .collect();

            reorder_asc(&mut transactions, sort_by);
            transactions.truncate(limit);
            return Ok(transactions);
        }

        // block height bounded query
        if query.block_height_gt.is_some()
            || query.block_height_gte.is_some()
            || query.block_height_lt.is_some()
            || query.block_height_lte.is_some()
        {
            let (min, max) = {
                let TransactionQueryInput {
                    block_height_gt,
                    block_height_gte,
                    block_height_lt,
                    block_height_lte,
                    ..
                } = query;
                let min_bound = match (block_height_gte, block_height_gt) {
                    (Some(gte), Some(gt)) => std::cmp::max(gte, gt + 1),
                    (Some(gte), None) => gte,
                    (None, Some(gt)) => gt + 1,
                    (None, None) => 1,
                };

                let max_bound = match (block_height_lte, block_height_lt) {
                    (Some(lte), Some(lt)) => std::cmp::min(lte, lt - 1),
                    (Some(lte), None) => lte,
                    (None, Some(lt)) => lt - 1,
                    (None, None) => db.get_best_block()?.unwrap().blockchain_length(),
                };
                (min_bound, max_bound)
            };

            let mut block_heights: Vec<u32> = (min..=max).collect();
            if sort_by == TransactionSortByInput::BlockHeightDesc {
                block_heights.reverse();
            }
            let mut transactions: Vec<Transaction> = Vec::with_capacity(limit);

            'outer: for height in block_heights {
                for block in db.get_blocks_at_height(height)? {
                    for cmd in SignedCommandWithData::from_precomputed(&block) {
                        let txn = Transaction::new(
                            cmd,
                            db,
                            epoch_num_user_commands,
                            total_num_user_commands,
                        );

                        if query.matches(&txn) {
                            transactions.push(txn);

                            if transactions.len() == limit {
                                break 'outer;
                            }
                        }
                    }
                }
            }
            // reorder_asc(&mut transactions, sort_by);
            return Ok(transactions);
        }
        // iterator mode & direction determined by desired sorting
        let mut transactions: Vec<Transaction> = Vec::with_capacity(limit);
        let (start_slot, direction) = match sort_by {
            TransactionSortByInput::BlockHeightAsc | TransactionSortByInput::DateTimeAsc => {
                (0, Direction::Forward)
            }
            TransactionSortByInput::BlockHeightDesc | TransactionSortByInput::DateTimeDesc => {
                (u32::MAX, Direction::Reverse)
            }
        };

        // from/to account (sender/receiver) query
        if query.from.as_ref().or(query.to.as_ref()).is_some() {
            let pk = query.from.as_ref().or(query.to.as_ref()).unwrap();
            let start = pk_txn_sort_key_prefix((pk as &str).into(), start_slot);
            let mode = IteratorMode::From(&start, direction);
            let txn_iter = if query.from.is_some() {
                db.txn_from_height_iterator(mode).flatten()
            } else {
                db.txn_to_height_iterator(mode).flatten()
            };

            for (key, _) in txn_iter {
                // public key bytes
                let txn_pk = pk_of_key(&key);
                if txn_pk.0 != *pk {
                    break;
                }

                // txn hash bytes
                let txn_hash = txn_hash_of_key(&key);
                let cmd = db
                    .get_user_command(&txn_hash, 0)?
                    .expect("command at txn hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);

                // include matching txns
                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() == limit {
                        break;
                    }
                };
            }
            return Ok(transactions);
        }

        let (start, direction) = match sort_by {
            TransactionSortByInput::BlockHeightAsc | TransactionSortByInput::DateTimeAsc => {
                (to_be_bytes(0), Direction::Forward)
            }
            TransactionSortByInput::BlockHeightDesc | TransactionSortByInput::DateTimeDesc => {
                (to_be_bytes(u32::MAX), Direction::Reverse)
            }
        };
        for (key, _) in db
            .user_commands_slot_iterator(IteratorMode::From(&start, direction))
            .flatten()
        {
            if query.hash.is_some() && query.hash != user_commands_iterator_txn_hash(&key).ok() {
                continue;
            }

            // Only add transactions that satisfy the input query
            let txn_hash = user_commands_iterator_txn_hash(&key)?;
            let state_hash = user_commands_iterator_state_hash(&key)?;
            let txn = Transaction::new(
                db.get_user_command_state_hash(&txn_hash, &state_hash)?
                    .unwrap(),
                db,
                epoch_num_user_commands,
                total_num_user_commands,
            );

            if query.matches(&txn) {
                transactions.push(txn);

                if transactions.len() == limit {
                    break;
                }
            };
        }

        Ok(transactions)
    }
}

fn reorder_asc<T>(values: &mut [T], sort_by: TransactionSortByInput) {
    match sort_by {
        TransactionSortByInput::BlockHeightAsc | TransactionSortByInput::DateTimeAsc => (),
        TransactionSortByInput::BlockHeightDesc | TransactionSortByInput::DateTimeDesc => {
            values.reverse()
        }
    }
}

impl Transaction {
    fn new(
        cmd: SignedCommandWithData,
        db: &Arc<IndexerStore>,
        epoch_num_user_commands: u32,
        total_num_user_commands: u32,
    ) -> Transaction {
        let block_state_hash = cmd.state_hash.to_owned();
        let block_date_time = date_time_to_scalar(cmd.date_time as i64);
        Transaction {
            transaction: TransactionWithoutBlock::new(
                cmd,
                get_block_canonicity(db, &block_state_hash.0),
                epoch_num_user_commands,
                total_num_user_commands,
            ),
            block: TransactionBlock {
                date_time: block_date_time,
                state_hash: block_state_hash.0.to_owned(),
            },
        }
    }
}

impl TransactionWithoutBlock {
    pub fn new(
        cmd: SignedCommandWithData,
        canonical: bool,
        epoch_num_user_commands: u32,
        total_num_user_commands: u32,
    ) -> Self {
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
                let nonce = common.nonce.t.t as u32;
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
                Self {
                    amount,
                    block_height: cmd.blockchain_length,
                    canonical,
                    failure_reason,
                    fee,
                    from: PublicKey::from(sender).0,
                    hash: cmd.tx_hash,
                    kind: kind.to_string(),
                    memo: decode_memo(&common.memo.t.0),
                    nonce,
                    receiver: PK {
                        public_key: receiver.to_owned(),
                    },
                    to: receiver,
                    token: Some(token_id),
                    epoch_num_user_commands,
                    total_num_user_commands,
                }
            }
        }
    }
}

impl TransactionQueryInput {
    fn matches(&self, transaction_with_block: &Transaction) -> bool {
        let mut matches = true;
        let transaction = transaction_with_block.transaction.clone();
        let TransactionQueryInput {
            hash,
            canonical,
            kind,
            memo,
            from,
            to,
            fee,
            fee_gt,
            fee_gte,
            fee_lt,
            fee_lte,
            fee_token,
            amount,
            amount_gt,
            amount_gte,
            amount_lte,
            amount_lt,
            block_height,
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            date_time,
            date_time_gt,
            date_time_gte,
            date_time_lt,
            date_time_lte,
            nonce,
            nonce_lte,
            nonce_gt,
            nonce_lt,
            nonce_gte,
            and,
            or,
            // TODO
            block,
            fee_payer: _,
            source: _,
            from_account: _,
            receiver: _,
            to_account: _,
            token: _,
            is_delegation: _,
        } = self;
        if let Some(state_hash) = block.as_ref().and_then(|b| b.state_hash.clone()) {
            matches &= transaction_with_block.block.state_hash == state_hash;
        }
        if let Some(hash) = hash {
            matches &= transaction.hash == *hash;
        }
        if let Some(kind) = kind {
            matches &= transaction.kind == *kind;
        }
        if let Some(canonical) = canonical {
            matches &= transaction.canonical == *canonical;
        }
        if let Some(from) = from {
            matches &= transaction.from == *from;
        }
        if let Some(to) = to {
            matches &= transaction.to == *to;
        }
        if let Some(memo) = memo {
            matches &= transaction.memo == *memo;
        }
        if let Some(fee_token) = fee_token {
            matches &= transaction.token == Some(*fee_token);
        }
        if let Some(query) = and {
            matches &= query.iter().all(|and| and.matches(transaction_with_block));
        }
        if let Some(query) = or {
            if !query.is_empty() {
                matches &= query.iter().any(|or| or.matches(transaction_with_block));
            }
        }

        // amount
        if let Some(amount) = amount {
            matches &= transaction_with_block.transaction.amount == *amount;
        }
        if let Some(amount_gt) = amount_gt {
            matches &= transaction_with_block.transaction.amount == *amount_gt;
        }
        if let Some(amount_gte) = amount_gte {
            matches &= transaction_with_block.transaction.amount == *amount_gte;
        }
        if let Some(amount_lt) = amount_lt {
            matches &= transaction_with_block.transaction.amount == *amount_lt;
        }
        if let Some(amount_lte) = amount_lte {
            matches &= transaction_with_block.transaction.amount == *amount_lte;
        }

        // fee
        if let Some(fee) = fee {
            matches &= transaction.fee == *fee;
        }
        if let Some(fee_gt) = fee_gt {
            matches &= transaction_with_block.transaction.fee == *fee_gt;
        }
        if let Some(fee_gte) = fee_gte {
            matches &= transaction_with_block.transaction.fee == *fee_gte;
        }
        if let Some(fee_lt) = fee_lt {
            matches &= transaction_with_block.transaction.fee == *fee_lt;
        }
        if let Some(fee_lte) = fee_lte {
            matches &= transaction_with_block.transaction.fee == *fee_lte;
        }

        // block height
        if let Some(block_height) = block_height {
            matches &= transaction_with_block.transaction.block_height == *block_height;
        }
        if let Some(block_height_gt) = block_height_gt {
            matches &= transaction_with_block.transaction.block_height > *block_height_gt;
        }
        if let Some(block_height_gte) = block_height_gte {
            matches &= transaction_with_block.transaction.block_height >= *block_height_gte;
        }
        if let Some(block_height_lt) = block_height_lt {
            matches &= transaction_with_block.transaction.block_height < *block_height_lt;
        }
        if let Some(block_height_lte) = block_height_lte {
            matches &= transaction_with_block.transaction.block_height <= *block_height_lte;
        }

        // date time
        if let Some(date_time) = date_time {
            matches &= transaction_with_block.block.date_time == *date_time;
        }
        if let Some(date_time_gt) = date_time_gt {
            matches &= transaction_with_block.block.date_time >= *date_time_gt;
        }
        if let Some(date_time_gte) = date_time_gte {
            matches &= transaction_with_block.block.date_time <= *date_time_gte;
        }
        if let Some(date_time_lt) = date_time_lt {
            matches &= transaction_with_block.block.date_time >= *date_time_lt;
        }
        if let Some(date_time_lte) = date_time_lte {
            matches &= transaction_with_block.block.date_time <= *date_time_lte;
        }

        // nonce
        if let Some(nonce) = nonce {
            matches &= transaction_with_block.transaction.nonce == *nonce;
        }
        if let Some(nonce_gt) = nonce_gt {
            matches &= transaction_with_block.transaction.nonce == *nonce_gt;
        }
        if let Some(nonce_gte) = nonce_gte {
            matches &= transaction_with_block.transaction.nonce == *nonce_gte;
        }
        if let Some(nonce_lt) = nonce_lt {
            matches &= transaction_with_block.transaction.nonce == *nonce_lt;
        }
        if let Some(nonce_lte) = nonce_lte {
            matches &= transaction_with_block.transaction.nonce == *nonce_lte;
        }

        matches
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum TransactionSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,

    #[graphql(name = "DATETIME_ASC")]
    DateTimeAsc,
    #[graphql(name = "DATETIME_DESC")]
    DateTimeDesc,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct TransactionWithoutBlock {
    amount: u64,
    block_height: u32,
    canonical: bool,
    failure_reason: String,
    fee: u64,
    from: String,
    hash: String,
    kind: String,
    memo: String,
    nonce: u32,
    /// The receiver's public key
    receiver: PK,
    to: String,
    token: Option<u64>,

    #[graphql(name = "epoch_num_user_commands")]
    epoch_num_user_commands: u32,

    #[graphql(name = "total_num_user_commands")]
    total_num_user_commands: u32,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct Transaction {
    block: TransactionBlock,

    #[graphql(flatten)]
    transaction: TransactionWithoutBlock,
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
struct TransactionBlock {
    date_time: DateTime,
    state_hash: String,
}
