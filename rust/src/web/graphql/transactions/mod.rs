use super::{date_time_to_scalar, db, get_block_canonicity, PK};
use crate::{
    block::store::BlockStore,
    command::{
        signed::{SignedCommandWithData, TxnHash},
        store::UserCommandStore,
        CommandStatusData,
    },
    constants::millis_to_global_slot,
    ledger::public_key::PublicKey,
    store::IndexerStore,
    utility::store::{
        command::user::{
            pk_txn_sort_key_prefix, txn_hash_of_key, user_commands_iterator_state_hash,
            user_commands_iterator_txn_hash,
        },
        state_hash_suffix, U32_LEN,
    },
    web::graphql::{gen::TransactionQueryInput, DateTime},
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use speedb::{Direction, IteratorMode};
use std::sync::Arc;

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

    #[graphql(name = "GLOBALSLOT_ASC")]
    GlobalSlotAsc,
    #[graphql(name = "GLOBALSLOT_DESC")]
    GlobalSlotDesc,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct TransactionWithoutBlock {
    amount: u64,
    block_height: u32,
    global_slot: u32,
    canonical: bool,
    failure_reason: Option<String>,
    is_applied: bool,
    fee: u64,
    from: String,
    hash: String,
    kind: String,
    memo: String,
    nonce: u32,
    receiver: PK,
    to: String,
    token: Option<u64>,

    /// Total number of user commands in the given epoch
    /// (default: current epoch)
    #[graphql(name = "epoch_num_user_commands")]
    epoch_num_user_commands: u32,

    /// Total number of user commands
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
            let hash = TxnHash::from(hash);
            if hash.is_valid() {
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
        query: Option<TransactionQueryInput>,
        #[graphql(default = 100)] limit: usize,
        sort_by: Option<TransactionSortByInput>,
    ) -> Result<Vec<Transaction>> {
        use TransactionSortByInput::*;

        let db = db(ctx);
        let epoch_num_user_commands = db.get_user_commands_epoch_count(None)?;
        let total_num_user_commands = db.get_user_commands_total_count()?;
        let sort_by = sort_by.unwrap_or(TransactionSortByInput::BlockHeightDesc);
        let mut transactions = vec![];

        // state hash query
        if let Some(state_hash) = query
            .as_ref()
            .and_then(|input| input.block.as_ref())
            .and_then(|block| block.state_hash.clone())
        {
            let query = query.expect("query input to exists");
            let block_height = db
                .get_block_height(&state_hash.into())?
                .expect("block height");
            let (min, max) = match sort_by {
                BlockHeightAsc | BlockHeightDesc => (block_height, block_height + 1),
                GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                    let min_slots = db
                        .get_block_global_slots_from_height(block_height)?
                        .expect("global slots at min height");
                    let max_slots = db
                        .get_block_global_slots_from_height(block_height + 1)?
                        .expect("global slots at max height");
                    (
                        min_slots.iter().min().copied().unwrap_or_default(),
                        max_slots.iter().max().copied().unwrap_or(u32::MAX),
                    )
                }
            };
            let iter = match sort_by {
                BlockHeightAsc => db.user_commands_height_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                BlockHeightDesc => db.user_commands_height_iterator(IteratorMode::From(
                    &max.to_be_bytes(),
                    Direction::Reverse,
                )),
                GlobalSlotAsc | DateTimeAsc => db.user_commands_slot_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                GlobalSlotDesc | DateTimeDesc => db.user_commands_slot_iterator(
                    IteratorMode::From(&max.to_be_bytes(), Direction::Reverse),
                ),
            };

            for (key, _) in iter.flatten() {
                if key[..U32_LEN] < *min.to_be_bytes().as_slice()
                    || key[..U32_LEN] > *max.to_be_bytes().as_slice()
                {
                    // check if we've gone beyond the desired bound
                    break;
                }

                let txn_hash = user_commands_iterator_txn_hash(&key)?;
                let state_hash = state_hash_suffix(&key)?;
                let cmd = db
                    .get_user_command_state_hash(&txn_hash, &state_hash)?
                    .expect("txn at hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);
                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(transactions);
        }

        // txn hash query (no state hash)
        if let Some(txn_hash) = query.as_ref().and_then(|input| input.hash.clone()) {
            let txn_hash = TxnHash::from(txn_hash);
            let query = query.expect("query input to exists");
            if let Some(state_hashes) = db.get_user_command_state_hashes(&txn_hash)? {
                for state_hash in state_hashes.iter() {
                    if let Some(cmd) = db.get_user_command_state_hash(&txn_hash, state_hash)? {
                        let txn = Transaction::new(
                            cmd,
                            db,
                            epoch_num_user_commands,
                            total_num_user_commands,
                        );
                        if query.matches(&txn) {
                            transactions.push(txn);
                        }

                        if transactions.len() >= limit {
                            break;
                        }
                    }
                }
            }
            return Ok(transactions);
        }

        // block height query
        if let Some(block_height) = query.as_ref().and_then(|input| input.block_height) {
            let query = query.expect("query input to exists");
            let (min, max) = match sort_by {
                BlockHeightAsc | BlockHeightDesc => (block_height, block_height + 1),
                GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                    let min_slots = db
                        .get_block_global_slots_from_height(block_height)?
                        .expect("global slots at min height");
                    let max_slots = db
                        .get_block_global_slots_from_height(block_height + 1)?
                        .expect("global slots at max height");
                    (
                        min_slots.iter().min().copied().unwrap_or_default(),
                        max_slots.iter().max().copied().unwrap_or(u32::MAX),
                    )
                }
            };
            let iter = match sort_by {
                BlockHeightAsc => db.user_commands_height_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                BlockHeightDesc => db.user_commands_height_iterator(IteratorMode::From(
                    &max.to_be_bytes(),
                    Direction::Reverse,
                )),
                GlobalSlotAsc | DateTimeAsc => db.user_commands_slot_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                GlobalSlotDesc | DateTimeDesc => db.user_commands_slot_iterator(
                    IteratorMode::From(&max.to_be_bytes(), Direction::Reverse),
                ),
            };

            for (key, _) in iter.flatten() {
                if key[..U32_LEN] != block_height.to_be_bytes() {
                    // we've gone beyond the desired block height
                    break;
                }

                let state_hash = state_hash_suffix(&key)?;
                let canonical = get_block_canonicity(db, &state_hash);
                if let Some(query_canonicity) = query.canonical {
                    if canonical != query_canonicity {
                        continue;
                    }
                }

                let txn_hash = user_commands_iterator_txn_hash(&key)?;
                let cmd = db
                    .get_user_command_state_hash(&txn_hash, &state_hash)?
                    .expect("txn at hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);
                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(transactions);
        }

        // iterator mode & direction determined by desired sorting
        let (start, direction) = match sort_by {
            BlockHeightAsc | DateTimeAsc | GlobalSlotAsc => (0, Direction::Forward),
            BlockHeightDesc | DateTimeDesc | GlobalSlotDesc => (u32::MAX, Direction::Reverse),
        };

        // from/to account (sender/receiver) query
        if query
            .as_ref()
            .map_or(false, |q| q.from.as_ref().or(q.to.as_ref()).is_some())
        {
            let query = query.expect("query input exists");
            let pk = query
                .from
                .as_ref()
                .or(query.to.as_ref())
                .expect("pk to exist");
            let start = pk_txn_sort_key_prefix(&(pk as &str).into(), start);
            let mode = IteratorMode::From(&start, direction);
            let txn_iter = if query.from.is_some() {
                db.txn_from_height_iterator(mode).flatten()
            } else {
                db.txn_to_height_iterator(mode).flatten()
            };
            for (key, _) in txn_iter {
                if key[..PublicKey::LEN] != *pk.as_bytes() {
                    // we've gone beyond the desired public key
                    break;
                }

                let state_hash = state_hash_suffix(&key)?;
                let canonical = get_block_canonicity(db, &state_hash);
                if let Some(query_canonicity) = query.canonical {
                    if canonical != query_canonicity {
                        continue;
                    }
                }

                let txn_hash = txn_hash_of_key(&key);
                let cmd = db
                    .get_user_command_state_hash(&txn_hash, &state_hash)?
                    .expect("command at txn hash and state hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);

                // include matching txns
                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() >= limit {
                        break;
                    }
                };
            }
            return Ok(transactions);
        }

        // block height bounded query
        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let query = query.expect("query input to exists");

            let (min, max) = {
                let (min_bound, max_bound) = calculate_inclusive_height_bounds(
                    query.block_height_gte,
                    query.block_height_gt,
                    query.block_height_lte,
                    query.block_height_lt,
                    db.get_best_block_height()?.expect("best block height"),
                )?;

                match sort_by {
                    BlockHeightAsc | BlockHeightDesc => (min_bound, max_bound),
                    GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                        let min_slots = db
                            .get_block_global_slots_from_height(min_bound)?
                            .expect("global slots at min height");
                        let max_slots = db
                            .get_block_global_slots_from_height(max_bound)?
                            .expect("global slots at max height");
                        (
                            min_slots.iter().min().copied().unwrap_or_default(),
                            max_slots.iter().max().copied().unwrap_or(u32::MAX),
                        )
                    }
                }
            };

            // reverse is exclusive so we increment
            let iter = match sort_by {
                BlockHeightAsc => db.user_commands_height_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                BlockHeightDesc => db.user_commands_height_iterator(IteratorMode::From(
                    &max.saturating_add(1).to_be_bytes(),
                    Direction::Reverse,
                )),
                GlobalSlotAsc | DateTimeAsc => db.user_commands_slot_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                GlobalSlotDesc | DateTimeDesc => db.user_commands_slot_iterator(
                    IteratorMode::From(&max.saturating_add(1).to_be_bytes(), Direction::Reverse),
                ),
            };

            for (key, _) in iter.flatten() {
                if key[..U32_LEN] > *max.to_be_bytes().as_slice()
                    || key[..U32_LEN] < *min.to_be_bytes().as_slice()
                {
                    // we've gone beyond the query bounds
                    break;
                }

                let state_hash = state_hash_suffix(&key)?;
                let canonical = get_block_canonicity(db, &state_hash);
                if let Some(query_canonicity) = query.canonical {
                    if canonical != query_canonicity {
                        continue;
                    }
                }

                let txn_hash = user_commands_iterator_txn_hash(&key)?;
                let cmd = db
                    .get_user_command_state_hash(&txn_hash, &state_hash)?
                    .expect("txn at hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);

                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(transactions);
        }

        // date time/global slot bounded query
        if query.as_ref().map_or(false, |q| {
            q.global_slot_gt.is_some()
                || q.global_slot_gte.is_some()
                || q.global_slot_lt.is_some()
                || q.global_slot_lte.is_some()
                || q.date_time_gt.is_some()
                || q.date_time_gte.is_some()
                || q.date_time_lt.is_some()
                || q.date_time_lte.is_some()
        }) {
            let query = query.expect("query input to exists");
            let (min, max) = {
                let (min_bound, max_bound) = calculate_inclusive_slot_bounds(
                    db,
                    query.global_slot_gt,
                    query.global_slot_gte,
                    query.global_slot_lt,
                    query.global_slot_lte,
                    &query.date_time_gt,
                    &query.date_time_gte,
                    &query.date_time_lt,
                    &query.date_time_lte,
                )?;

                match sort_by {
                    BlockHeightAsc | BlockHeightDesc => {
                        let min_heights = db
                            .get_block_heights_from_global_slot(min_bound)?
                            .expect("heights at min slot");
                        let max_heights = db
                            .get_block_heights_from_global_slot(max_bound)?
                            .expect("heights at max slot");
                        (
                            min_heights.iter().min().copied().unwrap_or_default(),
                            max_heights.iter().max().copied().unwrap_or(u32::MAX),
                        )
                    }
                    GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                        (min_bound, max_bound)
                    }
                }
            };

            // reverse is exclusive so we increment
            let iter = match sort_by {
                BlockHeightAsc => db.user_commands_height_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                BlockHeightDesc => db.user_commands_height_iterator(IteratorMode::From(
                    &max.saturating_add(1).to_be_bytes(),
                    Direction::Reverse,
                )),
                GlobalSlotAsc | DateTimeAsc => db.user_commands_slot_iterator(IteratorMode::From(
                    &min.to_be_bytes(),
                    Direction::Forward,
                )),
                GlobalSlotDesc | DateTimeDesc => db.user_commands_slot_iterator(
                    IteratorMode::From(&max.saturating_add(1).to_be_bytes(), Direction::Reverse),
                ),
            };

            for (key, _) in iter.flatten() {
                if key[..U32_LEN] > *max.to_be_bytes().as_slice()
                    || key[..U32_LEN] < *min.to_be_bytes().as_slice()
                {
                    // we've gone beyond the query bounds
                    break;
                }

                let state_hash = state_hash_suffix(&key)?;
                let canonical = get_block_canonicity(db, &state_hash);
                if let Some(query_canonicity) = query.canonical {
                    if canonical != query_canonicity {
                        continue;
                    }
                }

                let txn_hash = user_commands_iterator_txn_hash(&key)?;
                let cmd = db
                    .get_user_command_state_hash(&txn_hash, &state_hash)?
                    .expect("txn at hash");
                let txn =
                    Transaction::new(cmd, db, epoch_num_user_commands, total_num_user_commands);

                if query.matches(&txn) {
                    transactions.push(txn);

                    if transactions.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(transactions);
        }

        let iter = match sort_by {
            BlockHeightAsc => db.user_commands_height_iterator(IteratorMode::Start),
            BlockHeightDesc => db.user_commands_height_iterator(IteratorMode::End),
            DateTimeAsc | GlobalSlotAsc => db.user_commands_slot_iterator(IteratorMode::Start),
            DateTimeDesc | GlobalSlotDesc => db.user_commands_slot_iterator(IteratorMode::End),
        };
        for (key, _) in iter.flatten() {
            if let Some(ref q) = query {
                // early exit if txn hashes don't match if we're filtering by it
                if q.hash.is_some()
                    && q.hash
                        != user_commands_iterator_txn_hash(&key)
                            .ok()
                            .map(|t| t.to_string())
                {
                    continue;
                }
            }

            let state_hash = user_commands_iterator_state_hash(&key)?;
            let canonical = get_block_canonicity(db, &state_hash);
            if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn_hash = user_commands_iterator_txn_hash(&key)?;
            let txn = Transaction::new(
                db.get_user_command_state_hash(&txn_hash, &state_hash)?
                    .unwrap(),
                db,
                epoch_num_user_commands,
                total_num_user_commands,
            );

            if query.as_ref().map_or(true, |q| q.matches(&txn)) {
                transactions.push(txn);

                if transactions.len() >= limit {
                    break;
                }
            };
        }

        Ok(transactions)
    }
}

fn calculate_inclusive_height_bounds(
    block_height_gte: Option<u32>,
    block_height_gt: Option<u32>,
    block_height_lte: Option<u32>,
    block_height_lt: Option<u32>,
    best_block_height: u32,
) -> Result<(u32, u32)> {
    // Ensure min bound is inclusive
    let min_bound = match (block_height_gte, block_height_gt) {
        (Some(gte), Some(gt)) => gte.max(gt.saturating_add(1)),
        (Some(gte), None) => gte,
        (None, Some(gt)) => gt.saturating_add(1),
        (None, None) => 1,
    };

    // Ensure max bound is inclusive
    let max_bound = match (block_height_lte, block_height_lt) {
        (Some(lte), Some(lt)) => lte.min(lt.saturating_sub(1)),
        (Some(lte), None) => lte,
        (None, Some(lt)) => lt.saturating_sub(1),
        (None, None) => best_block_height,
    };

    Ok((min_bound, max_bound))
}

#[allow(clippy::too_many_arguments)]
fn calculate_inclusive_slot_bounds(
    db: &Arc<IndexerStore>,
    global_slot_gt: Option<u32>,
    global_slot_gte: Option<u32>,
    global_slot_lt: Option<u32>,
    global_slot_lte: Option<u32>,
    date_time_gt: &Option<DateTime>,
    date_time_gte: &Option<DateTime>,
    date_time_lt: &Option<DateTime>,
    date_time_lte: &Option<DateTime>,
) -> Result<(u32, u32)> {
    let min_bound = match (
        global_slot_gte.or(date_time_gte
            .as_ref()
            .map(|dt| millis_to_global_slot(dt.timestamp_millis()))),
        global_slot_gt.or(date_time_gt
            .as_ref()
            .map(|dt| millis_to_global_slot(dt.timestamp_millis()))),
    ) {
        (Some(gte), Some(gt)) => gte.max(gt.saturating_add(1)),
        (Some(gte), None) => gte,
        (None, Some(gt)) => gt.saturating_add(1),
        (None, None) => 0,
    };
    let min_bound = db
        .get_next_global_slot_produced(min_bound)?
        .expect("min global slot produced");

    let max_bound = match (
        global_slot_lte.or(date_time_lte
            .as_ref()
            .map(|dt| millis_to_global_slot(dt.timestamp_millis()))),
        global_slot_lt.or(date_time_lt
            .as_ref()
            .map(|dt| millis_to_global_slot(dt.timestamp_millis()))),
    ) {
        (Some(lte), Some(lt)) => lte.min(lt.saturating_sub(1)),
        (Some(lte), None) => lte,
        (None, Some(lt)) => lt.saturating_sub(1),
        (None, None) => db
            .get_best_block_global_slot()?
            .expect("best block global slot"),
    };
    let max_bound = db.get_prev_global_slot_produced(max_bound)?;

    Ok((min_bound, max_bound))
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
                get_block_canonicity(db, &block_state_hash),
                epoch_num_user_commands,
                total_num_user_commands,
            ),
            block: TransactionBlock {
                date_time: block_date_time,
                state_hash: block_state_hash.0,
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
        let receiver = cmd.command.receiver_pk();
        let failure_reason = match cmd.status {
            CommandStatusData::Applied { .. } => None,
            CommandStatusData::Failed(failed_types, _) => {
                failed_types.first().map(|f| f.to_string())
            }
        };
        let is_applied = failure_reason.is_none();

        Self {
            canonical,
            is_applied,
            failure_reason,
            amount: cmd.command.amount(),
            block_height: cmd.blockchain_length,
            global_slot: cmd.global_slot_since_genesis,
            fee: cmd.command.fee(),
            from: cmd.command.source_pk().0,
            hash: cmd.tx_hash.to_string(),
            kind: cmd.command.kind().to_string(),
            memo: cmd.command.memo(),
            nonce: cmd.command.nonce().0,
            receiver: PK {
                public_key: receiver.0.to_owned(),
            },
            to: receiver.0,
            token: cmd.command.fee_token(),
            epoch_num_user_commands,
            total_num_user_commands,
        }
    }
}

impl TransactionQueryInput {
    fn matches(&self, transaction: &Transaction) -> bool {
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
            global_slot,
            global_slot_gt,
            global_slot_gte,
            global_slot_lt,
            global_slot_lte,
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
            block,
            failure_reason,
            is_applied,
            fee_payer: _,
            source: _,
            from_account: _,
            receiver: _,
            to_account: _,
            token: _,
            is_delegation: _,
        } = self;
        if let Some(state_hash) = block.as_ref().and_then(|b| b.state_hash.as_ref()) {
            if transaction.block.state_hash != *state_hash {
                return false;
            }
        }
        if let Some(hash) = hash {
            if transaction.transaction.hash != *hash {
                return false;
            }
        }
        if let Some(kind) = kind {
            if transaction.transaction.kind != *kind {
                return false;
            }
        }
        if let Some(canonical) = canonical {
            if transaction.transaction.canonical != *canonical {
                return false;
            }
        }
        if let Some(from) = from {
            if transaction.transaction.from != *from {
                return false;
            }
        }
        if let Some(to) = to {
            if transaction.transaction.to != *to {
                return false;
            }
        }
        if let Some(memo) = memo {
            if transaction.transaction.memo != *memo {
                return false;
            }
        }
        if let Some(fee_token) = fee_token {
            if transaction.transaction.token != Some(*fee_token) {
                return false;
            }
        }

        // failed/applied
        if let Some(failure_reason) = failure_reason {
            if transaction.transaction.failure_reason.as_ref() != Some(failure_reason) {
                return false;
            }
        }
        if let Some(is_applied) = is_applied {
            if transaction.transaction.failure_reason.is_none() != *is_applied {
                return false;
            }
        }

        // boolean
        if let Some(query) = and {
            if query.iter().all(|and| and.matches(transaction)) {
                return false;
            }
        }
        if let Some(query) = or {
            if !query.is_empty() && query.iter().any(|or| or.matches(transaction)) {
                return false;
            }
        }

        // amount
        if let Some(amount) = amount {
            if transaction.transaction.amount != *amount {
                return false;
            }
        }
        if let Some(amount_gt) = amount_gt {
            if transaction.transaction.amount <= *amount_gt {
                return false;
            }
        }
        if let Some(amount_gte) = amount_gte {
            if transaction.transaction.amount < *amount_gte {
                return false;
            }
        }
        if let Some(amount_lt) = amount_lt {
            if transaction.transaction.amount >= *amount_lt {
                return false;
            }
        }
        if let Some(amount_lte) = amount_lte {
            if transaction.transaction.amount > *amount_lte {
                return false;
            }
        }

        // fee
        if let Some(fee) = fee {
            if transaction.transaction.fee != *fee {
                return false;
            }
        }
        if let Some(fee_gt) = fee_gt {
            if transaction.transaction.fee <= *fee_gt {
                return false;
            }
        }
        if let Some(fee_gte) = fee_gte {
            if transaction.transaction.fee < *fee_gte {
                return false;
            }
        }
        if let Some(fee_lt) = fee_lt {
            if transaction.transaction.fee >= *fee_lt {
                return false;
            }
        }
        if let Some(fee_lte) = fee_lte {
            if transaction.transaction.fee > *fee_lte {
                return false;
            }
        }

        // block height
        if let Some(block_height) = block_height {
            if transaction.transaction.block_height != *block_height {
                return false;
            }
        }
        if let Some(block_height_gt) = block_height_gt {
            if transaction.transaction.block_height <= *block_height_gt {
                return false;
            }
        }
        if let Some(block_height_gte) = block_height_gte {
            if transaction.transaction.block_height < *block_height_gte {
                return false;
            }
        }
        if let Some(block_height_lt) = block_height_lt {
            if transaction.transaction.block_height >= *block_height_lt {
                return false;
            }
        }
        if let Some(block_height_lte) = block_height_lte {
            if transaction.transaction.block_height > *block_height_lte {
                return false;
            }
        }

        // global slot
        if let Some(global_slot) = global_slot {
            if transaction.transaction.global_slot != *global_slot {
                return false;
            }
        }
        if let Some(global_slot_gt) = global_slot_gt {
            if transaction.transaction.global_slot <= *global_slot_gt {
                return false;
            }
        }
        if let Some(global_slot_gte) = global_slot_gte {
            if transaction.transaction.global_slot < *global_slot_gte {
                return false;
            }
        }
        if let Some(global_slot_lt) = global_slot_lt {
            if transaction.transaction.global_slot >= *global_slot_lt {
                return false;
            }
        }
        if let Some(global_slot_lte) = global_slot_lte {
            if transaction.transaction.global_slot > *global_slot_lte {
                return false;
            }
        }

        // date time
        let txn_date_time_millis = transaction.block.date_time.timestamp_millis();
        if let Some(date_time) = date_time {
            if txn_date_time_millis != (*date_time).timestamp_millis() {
                return false;
            }
        }
        if let Some(date_time_gt) = date_time_gt {
            if txn_date_time_millis <= (*date_time_gt).timestamp_millis() {
                return false;
            }
        }
        if let Some(date_time_gte) = date_time_gte {
            if txn_date_time_millis < (*date_time_gte).timestamp_millis() {
                return false;
            }
        }
        if let Some(date_time_lt) = date_time_lt {
            if txn_date_time_millis >= (*date_time_lt).timestamp_millis() {
                return false;
            }
        }
        if let Some(date_time_lte) = date_time_lte {
            if txn_date_time_millis > (*date_time_lte).timestamp_millis() {
                return false;
            }
        }

        // nonce
        if let Some(nonce) = nonce {
            if transaction.transaction.nonce != *nonce {
                return false;
            }
        }
        if let Some(nonce_gt) = nonce_gt {
            if transaction.transaction.nonce <= *nonce_gt {
                return false;
            }
        }
        if let Some(nonce_gte) = nonce_gte {
            if transaction.transaction.nonce < *nonce_gte {
                return false;
            }
        }
        if let Some(nonce_lt) = nonce_lt {
            if transaction.transaction.nonce >= *nonce_lt {
                return false;
            }
        }
        if let Some(nonce_lte) = nonce_lte {
            if transaction.transaction.nonce > *nonce_lte {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_with_both_gte_and_gt() {
        let gte = Some(10);
        let gt = Some(8);
        let lte = None;
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 10);
        assert_eq!(max_bound, best_block_height);
    }

    #[test]
    fn test_bounds_with_only_gte() {
        let gte = Some(15);
        let gt = None;
        let lte = None;
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 15);
        assert_eq!(max_bound, best_block_height);
    }

    #[test]
    fn test_bounds_with_only_gt() {
        let gt = Some(12);
        let gte = None;
        let lte = None;
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 13);
        assert_eq!(max_bound, best_block_height);
    }

    #[test]
    fn test_bounds_with_both_lte_and_lt() {
        let gte = None;
        let gt = None;
        let lte = Some(20);
        let lt = Some(22);
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 1);
        assert_eq!(max_bound, 20);
    }

    #[test]
    fn test_bounds_with_only_lte() {
        let gte = None;
        let gt = None;
        let lte = Some(30);
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 1);
        assert_eq!(max_bound, 30);
    }

    #[test]
    fn test_bounds_with_only_lt() {
        let gte = None;
        let gt = None;
        let lte = None;
        let lt = Some(25);
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 1);
        assert_eq!(max_bound, 24);
    }

    #[test]
    fn test_bounds_with_all_parameters() {
        let gte = Some(15);
        let gt = Some(12);
        let lte = Some(30);
        let lt = Some(28);
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 15);
        assert_eq!(max_bound, 27);
    }

    #[test]
    fn test_bounds_with_none() {
        let gte = None;
        let gt = None;
        let lte = None;
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) =
            calculate_inclusive_height_bounds(gte, gt, lte, lt, best_block_height).unwrap();

        assert_eq!(min_bound, 1);
        assert_eq!(max_bound, best_block_height);
    }
}
