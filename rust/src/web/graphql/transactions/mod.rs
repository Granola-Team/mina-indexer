//! GraphQL `transaction` & `transactions` endpoint

use super::{date_time_to_scalar, db, get_block_canonicity, PK};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    command::{
        signed::{SignedCommandWithData, TxnHash},
        store::UserCommandStore,
        CommandStatusData,
    },
    constants::millis_to_global_slot,
    ledger::token::TokenAddress,
    store::{zkapp::tokens::ZkappTokenStore, IndexerStore},
    utility::store::{
        command::user::{user_commands_iterator_state_hash, user_commands_iterator_txn_hash},
        common::{state_hash_suffix, U32_LEN},
    },
    web::graphql::{gen::TransactionQueryInput, DateTime},
};
use anyhow::Context as AC;
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use serde::Serialize;
use speedb::{DBIterator, Direction, IteratorMode};
use std::sync::Arc;

#[derive(Clone, Debug, SimpleObject)]
pub struct Transaction {
    block: TransactionBlock,

    #[graphql(flatten)]
    transaction: TransactionWithoutBlock,
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

    #[graphql(name = "GLOBALSLOT_ASC")]
    GlobalSlotAsc,
    #[graphql(name = "GLOBALSLOT_DESC")]
    GlobalSlotDesc,
}

#[derive(Clone, Debug, SimpleObject, Serialize)]
pub struct TransactionWithoutBlock {
    amount: u64,
    block_height: u32,
    global_slot: u32,
    canonical: bool,
    failure_reason: Option<String>,
    is_applied: bool,
    zkapp: Option<TransactionZkapp>,
    fee: u64,
    from: String,
    hash: String,
    kind: String,
    memo: String,
    nonce: u32,
    receiver: Option<PK>,
    to: Option<String>,
    tokens: Vec<String>,

    /// Total number of user commands in the given epoch
    /// (default: current epoch)
    #[graphql(name = "epoch_num_user_commands")]
    epoch_num_user_commands: u32,

    /// Total number of user commands
    #[graphql(name = "total_num_user_commands")]
    total_num_user_commands: u32,

    /// Total number of zkapp commands in the given epoch
    /// (default: current epoch)
    #[graphql(name = "epoch_num_zkapp_commands")]
    epoch_num_zkapp_commands: u32,

    /// Total number of zkapp commands
    #[graphql(name = "total_num_zkapp_commands")]
    total_num_zkapp_commands: u32,
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
struct TransactionBlock {
    date_time: DateTime,
    state_hash: String,
}

#[derive(Clone, Debug, PartialEq, SimpleObject, Serialize)]
struct TokenAccount {
    /// Public key
    pk: String,

    /// Token address
    token: String,

    /// Token symbol
    symbol: String,

    /// Balance change
    #[graphql(name = "balance_change")]
    balance_change: i64,

    /// Increment nonce
    #[graphql(name = "increment_nonce")]
    increment_nonce: bool,
}

#[derive(Clone, Debug, PartialEq, SimpleObject, Serialize)]
struct TransactionZkapp {
    /// Accounts updated
    #[graphql(name = "accounts_updated")]
    accounts_updated: Vec<TokenAccount>,

    /// Actions
    actions: Vec<String>,

    /// Events
    events: Vec<String>,
}

#[derive(Default)]
pub struct TransactionsQueryRoot;

//////////
// impl //
//////////

#[Object]
impl TransactionsQueryRoot {
    pub async fn transaction(
        &self,
        ctx: &Context<'_>,
        query: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        let db = db(ctx);

        let num_commands = [
            db.get_user_commands_epoch_count(None)?,
            db.get_user_commands_total_count()?,
            db.get_zkapp_commands_epoch_count(None)?,
            db.get_zkapp_commands_total_count()?,
        ];

        if let Some(hash) = query.hash.as_ref() {
            let hash = match TxnHash::new(hash) {
                Ok(txn_hash) => txn_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid txn hash: {}",
                        hash
                    )))
                }
            };

            return Ok(db
                .get_user_command(&hash, 0)?
                .map(|cmd| Transaction::new(cmd, db, num_commands)));
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
        let db = db(ctx);
        let num_commands = [
            db.get_user_commands_epoch_count(None)?,
            db.get_user_commands_total_count()?,
            db.get_zkapp_commands_epoch_count(None)?,
            db.get_zkapp_commands_total_count()?,
        ];

        let sort_by = sort_by.unwrap_or(TransactionSortByInput::BlockHeightDesc);
        let mut transactions = vec![];

        ////////////////////
        // txn hash query //
        ////////////////////

        if let Some(txn_hash) = query.as_ref().and_then(|input| input.hash.as_ref()) {
            let txn_hash = match TxnHash::new(txn_hash) {
                Ok(txn_hash) => txn_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid txn hash: {}",
                        txn_hash
                    )))
                }
            };

            let query = query.expect("query input");
            if let Some(state_hashes) = db.get_user_command_state_hashes(&txn_hash)? {
                for state_hash in state_hashes.iter() {
                    if transactions.len() >= limit {
                        break;
                    }

                    if let Some(cmd) = db.get_user_command_state_hash(&txn_hash, state_hash)? {
                        let txn = Transaction::new(cmd, db, num_commands);

                        if query.matches(&txn) {
                            transactions.push(txn);
                        }
                    }
                }
            }

            return Ok(transactions);
        }

        //////////////////////
        // state hash query //
        //////////////////////

        if let Some(state_hash) = query
            .as_ref()
            .and_then(|input| input.block.as_ref())
            .and_then(|block| block.state_hash.as_ref())
        {
            let state_hash = match StateHash::new(state_hash) {
                Ok(state_hash) => state_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid state hash {}",
                        state_hash
                    )))
                }
            };

            TransactionQueryInput::state_hash_query_handler(
                &mut transactions,
                db,
                query.as_ref(),
                sort_by,
                &state_hash,
                num_commands,
                limit,
            )?;

            return Ok(transactions);
        }

        ////////////////////////
        // block height query //
        ////////////////////////

        if query
            .as_ref()
            .and_then(|input| input.block_height)
            .is_some()
        {
            TransactionQueryInput::block_height_query_handler(
                &mut transactions,
                db,
                query.as_ref(),
                sort_by,
                num_commands,
                limit,
            )?;

            return Ok(transactions);
        }

        ///////////////////////////
        // sender/receiver query //
        ///////////////////////////

        if let Some((from, to)) = query.as_ref().map(|q| (q.from.as_ref(), q.to.as_ref())) {
            if from.or(to).is_some() {
                if from.map(|pk| !PublicKey::is_valid(pk)).unwrap_or_default() {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid receiver public key: {}",
                        from.unwrap()
                    )));
                }

                if to.map(|pk| !PublicKey::is_valid(pk)).unwrap_or_default() {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid sender public key: {}",
                        to.unwrap()
                    )));
                }

                TransactionQueryInput::from_to_query_handler(
                    &mut transactions,
                    db,
                    query.as_ref(),
                    sort_by,
                    num_commands,
                    limit,
                )?;

                return Ok(transactions);
            }
        }

        //////////////////////////////
        // block height bound query //
        //////////////////////////////

        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            TransactionQueryInput::block_height_bound_query_handler(
                &mut transactions,
                db,
                query.as_ref(),
                sort_by,
                num_commands,
                limit,
            )?;

            return Ok(transactions);
        }

        ///////////////////////////////////////
        // date time/global slot bound query //
        ///////////////////////////////////////

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
            TransactionQueryInput::date_time_or_slot_bound_query_handler(
                &mut transactions,
                db,
                query.as_ref(),
                sort_by,
                num_commands,
                limit,
            )?;

            return Ok(transactions);
        }

        /////////////////
        // token query //
        /////////////////

        if let Some(token) = query.as_ref().and_then(|q| q.token.as_ref()) {
            let token = match TokenAddress::new(token) {
                Some(token) => token,
                None => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid token address: {}",
                        token
                    )))
                }
            };

            TransactionQueryInput::token_query_handler(
                &mut transactions,
                db,
                query.as_ref(),
                sort_by,
                &token,
                num_commands,
                limit,
            )?;

            return Ok(transactions);
        }

        ///////////////////
        // default query //
        ///////////////////

        TransactionQueryInput::default_query_handler(
            &mut transactions,
            db,
            query.as_ref(),
            sort_by,
            num_commands,
            limit,
        )?;

        Ok(transactions)
    }
}

impl Transaction {
    fn new(
        cmd: SignedCommandWithData,
        db: &Arc<IndexerStore>,
        num_commands: [u32; 4],
    ) -> Transaction {
        let block_state_hash = cmd.state_hash.to_owned();
        let block_date_time = date_time_to_scalar(cmd.date_time as i64);

        Self {
            transaction: TransactionWithoutBlock::new(
                db,
                cmd,
                get_block_canonicity(db, &block_state_hash),
                num_commands,
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
        db: &Arc<IndexerStore>,
        cmd: SignedCommandWithData,
        canonical: bool,
        num_commands: [u32; 4],
    ) -> Self {
        let zkapp = if cmd.is_zkapp_command() {
            Some(TransactionZkapp {
                accounts_updated: cmd
                    .accounts_updated()
                    .into_iter()
                    .map(|(pk, token, balance_change, increment_nonce)| {
                        TokenAccount::from(db, pk, token, balance_change, increment_nonce)
                    })
                    .collect(),
                actions: cmd.actions(),
                events: cmd.events(),
            })
        } else {
            None
        };

        let receiver = cmd.command.receiver_pk();
        let receiver = receiver.first().map(PublicKey::to_string);

        let failure_reason = match cmd.status {
            CommandStatusData::Applied { .. } => None,
            CommandStatusData::Failed(failed_types, _) => {
                failed_types.first().map(|f| f.to_string())
            }
        };
        let is_applied = failure_reason.is_none();

        Self {
            zkapp,
            canonical,
            is_applied,
            failure_reason,
            amount: cmd.command.amount(),
            block_height: cmd.blockchain_length,
            global_slot: cmd.global_slot_since_genesis,
            fee: cmd.command.fee(),
            from: cmd.command.source_pk().0,
            hash: cmd.txn_hash.to_string(),
            kind: cmd.command.kind().to_string(),
            memo: cmd.command.memo(),
            nonce: cmd.command.nonce().0,
            receiver: receiver.to_owned().map(|pk| PK { public_key: pk }),
            to: receiver,
            tokens: cmd.command.tokens().into_iter().map(|t| t.0).collect(),
            epoch_num_user_commands: num_commands[0],
            total_num_user_commands: num_commands[1],
            epoch_num_zkapp_commands: num_commands[2],
            total_num_zkapp_commands: num_commands[3],
        }
    }
}

impl TransactionQueryInput {
    #[allow(clippy::too_many_lines)]
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
            fee_token: _,
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
            zkapp,
            fee_payer: _,
            source: _,
            from_account: _,
            receiver: _,
            to_account: _,
            token,
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

        if let Some(zkapp) = zkapp {
            if transaction.transaction.zkapp.is_some() != *zkapp {
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

        // if zkapp command, check the accounts updated
        if let Some(to) = to {
            if let Some(zkapp) = transaction.transaction.zkapp.as_ref() {
                let pk_updated = zkapp
                    .accounts_updated
                    .iter()
                    .map(|account| account.pk == *to)
                    .any(|b| b);

                if !pk_updated {
                    return false;
                }
            } else if let Some(txn_to) = transaction.transaction.to.as_ref() {
                if txn_to != to {
                    return false;
                }
            }
        }

        if let Some(memo) = memo {
            if transaction.transaction.memo != *memo {
                return false;
            }
        }

        if let Some(token) = token {
            if !transaction.transaction.tokens.contains(token) {
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

    /// Default query handler
    fn default_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let iter = match (sort_by, query.as_ref().and_then(|q| q.zkapp)) {
            (BlockHeightAsc, None | Some(false)) => {
                db.user_commands_height_iterator(IteratorMode::Start)
            }
            (BlockHeightAsc, Some(true)) => db.zkapp_commands_height_iterator(IteratorMode::Start),
            (BlockHeightDesc, None | Some(false)) => {
                db.user_commands_height_iterator(IteratorMode::End)
            }
            (BlockHeightDesc, Some(true)) => db.zkapp_commands_height_iterator(IteratorMode::End),
            (GlobalSlotAsc | DateTimeAsc, None | Some(false)) => {
                db.user_commands_slot_iterator(IteratorMode::Start)
            }
            (GlobalSlotAsc | DateTimeAsc, Some(true)) => {
                db.zkapp_commands_slot_iterator(IteratorMode::Start)
            }
            (GlobalSlotDesc | DateTimeDesc, None | Some(false)) => {
                db.user_commands_slot_iterator(IteratorMode::End)
            }
            (GlobalSlotDesc | DateTimeDesc, Some(true)) => {
                db.zkapp_commands_slot_iterator(IteratorMode::End)
            }
        };

        for (key, value) in iter.flatten() {
            if txns.len() >= limit {
                // exit if the limit has been reached
                break;
            }

            if let Some(q) = query {
                // early exit if txn hashes don't match if we're filtering by it
                if q.hash.is_some()
                    && user_commands_iterator_txn_hash(&key)
                        .ok()
                        .map_or(false, |t| t.ref_inner() != q.hash.as_ref().unwrap())
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

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.as_ref().map_or(true, |q| q.matches(&txn)) {
                txns.push(txn);
            };
        }

        Ok(())
    }

    /// Handler for block height query
    fn block_height_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let block_height = query
            .as_ref()
            .and_then(|input| input.block_height)
            .expect("block height query input");
        let query = query.expect("query input");

        let (min, max, check_height) = match sort_by {
            BlockHeightAsc | BlockHeightDesc => (block_height, block_height + 1, true),
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
                    false,
                )
            }
        };

        let iter = Self::zkapp_or_not_height_or_slot_iterator(db, &sort_by, min, max, query.zkapp)?;
        for (key, value) in iter.flatten() {
            if check_height && key[..U32_LEN] != block_height.to_be_bytes() || txns.len() >= limit {
                // beyond the desired block height or limit
                break;
            }

            let state_hash = state_hash_suffix(&key)?;
            let canonical = get_block_canonicity(db, &state_hash);

            if let Some(query_canonicity) = query.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.matches(&txn) {
                txns.push(txn);
            }
        }

        Ok(())
    }

    /// Handler for block height bound query
    fn block_height_bound_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let query = query.expect("query input");
        let (min, max) = {
            let (min_bound, max_bound) = Self::calculate_inclusive_height_bounds(
                [
                    query.block_height_gte,
                    query.block_height_gt,
                    query.block_height_lte,
                    query.block_height_lt,
                ],
                db.get_best_block_height()?.expect("best block height"),
            )?;

            match sort_by {
                BlockHeightAsc | BlockHeightDesc => (min_bound, max_bound.saturating_add(1)),
                GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                    let min_slots = db
                        .get_block_global_slots_from_height(min_bound)?
                        .expect("global slots at min height");
                    let max_slots = db
                        .get_block_global_slots_from_height(max_bound)?
                        .expect("global slots at max height");
                    (
                        min_slots.iter().min().copied().unwrap_or_default(),
                        max_slots
                            .iter()
                            .max()
                            .copied()
                            .unwrap_or(u32::MAX)
                            .saturating_add(1),
                    )
                }
            }
        };

        let iter = Self::zkapp_or_not_height_or_slot_iterator(db, &sort_by, min, max, query.zkapp)?;
        for (key, value) in iter.flatten() {
            // keys have format: {u32 prefix}{txn hash}{state hash}
            if key[..U32_LEN] > *max.to_be_bytes().as_slice()
                || key[..U32_LEN] < *min.to_be_bytes().as_slice()
                || txns.len() >= limit
            {
                // beyond the query bounds or limit
                break;
            }

            let state_hash = state_hash_suffix(&key)?;
            let canonical = get_block_canonicity(db, &state_hash);

            if let Some(query_canonicity) = query.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.matches(&txn) {
                txns.push(txn);
            }
        }

        Ok(())
    }

    /// Handler for date time/global slot bound query
    fn date_time_or_slot_bound_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let query = query.expect("query input");
        let (min, max) = {
            let (min_bound, max_bound) = Self::calculate_inclusive_slot_bounds(
                db,
                [
                    query.global_slot_gt,
                    query.global_slot_gte,
                    query.global_slot_lt,
                    query.global_slot_lte,
                ],
                [
                    &query.date_time_gt,
                    &query.date_time_gte,
                    &query.date_time_lt,
                    &query.date_time_lte,
                ],
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
        let max = max.saturating_add(1);

        let iter = Self::zkapp_or_not_height_or_slot_iterator(db, &sort_by, min, max, query.zkapp)?;
        for (key, value) in iter.flatten() {
            if key[..U32_LEN] > *max.to_be_bytes().as_slice()
                || key[..U32_LEN] < *min.to_be_bytes().as_slice()
                || txns.len() >= limit
            {
                // beyond query bounds or limit
                break;
            }

            let state_hash = state_hash_suffix(&key)?;
            let canonical = get_block_canonicity(db, &state_hash);

            if let Some(query_canonicity) = query.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.matches(&txn) {
                txns.push(txn);
            }
        }

        Ok(())
    }

    /// Handler for state hash query
    fn state_hash_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        state_hash: &StateHash,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let query = query.as_ref().expect("query input to exists");
        let block_height = db
            .get_block_height(state_hash)?
            .with_context(|| state_hash.to_string())
            .expect("block height");

        let (min, max) = match sort_by {
            BlockHeightAsc | BlockHeightDesc => (block_height, block_height.saturating_add(1)),
            GlobalSlotAsc | GlobalSlotDesc | DateTimeAsc | DateTimeDesc => {
                let slots = db
                    .get_block_global_slots_from_height(block_height.saturating_add(1))?
                    .expect("global slots at min height");
                (
                    slots.iter().min().copied().unwrap_or_default(),
                    slots.iter().max().copied().unwrap_or(u32::MAX),
                )
            }
        };

        let iter = Self::zkapp_or_not_height_or_slot_iterator(db, &sort_by, min, max, query.zkapp)?;
        for (key, value) in iter.flatten() {
            if key[..U32_LEN] < *min.to_be_bytes().as_slice()
                || key[..U32_LEN] > *max.to_be_bytes().as_slice()
                || txns.len() >= limit
            {
                // beyond desired bound or limit
                break;
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.matches(&txn) {
                txns.push(txn);
            }
        }

        Ok(())
    }

    /// Handler for token query
    fn token_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        token: &TokenAddress,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let direction = match sort_by {
            BlockHeightAsc | DateTimeAsc | GlobalSlotAsc => Direction::Forward,
            BlockHeightDesc | DateTimeDesc | GlobalSlotDesc => Direction::Reverse,
        };

        // per token iterator
        let iter = match sort_by {
            BlockHeightAsc | BlockHeightDesc => {
                db.user_commands_per_token_height_iterator(token, direction)
            }
            DateTimeAsc | GlobalSlotAsc | DateTimeDesc | GlobalSlotDesc => {
                db.user_commands_per_token_slot_iterator(token, direction)
            }
        };

        // only iterate over specified token
        for (key, value) in iter.flatten() {
            if *token.0.as_bytes() != key[..TokenAddress::LEN] || txns.len() >= limit {
                // beyond the desired token or limit
                break;
            }

            let state_hash = user_commands_iterator_state_hash(&key[TokenAddress::LEN..])?;
            let canonical = get_block_canonicity(db, &state_hash);

            if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.as_ref().map_or(false, |q| q.matches(&txn)) {
                txns.push(txn);
            };
        }

        Ok(())
    }

    /// Handler for from/to query
    fn from_to_query_handler(
        txns: &mut Vec<Transaction>,
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        sort_by: TransactionSortByInput,
        num_commands: [u32; 4],
        limit: usize,
    ) -> anyhow::Result<()> {
        use TransactionSortByInput::*;

        let direction = match sort_by {
            BlockHeightAsc | DateTimeAsc | GlobalSlotAsc => Direction::Forward,
            BlockHeightDesc | DateTimeDesc | GlobalSlotDesc => Direction::Reverse,
        };

        let query = query.expect("query input");
        let pk = query
            .from
            .as_ref()
            .or(query.to.as_ref())
            .expect("pk to exist");

        // make the iterator
        let iter = {
            // set start key
            let mut start = [0u8; PublicKey::LEN + U32_LEN + U32_LEN + 1];
            start[..PublicKey::LEN].copy_from_slice(pk.as_bytes());

            // get upper bound if reverse
            if let Direction::Reverse = direction {
                start[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
                start[PublicKey::LEN..][U32_LEN..][..U32_LEN]
                    .copy_from_slice(&u32::MAX.to_be_bytes());
                start[PublicKey::LEN..][U32_LEN..][U32_LEN..].copy_from_slice("Z".as_bytes());
            }

            if query.from.is_some() {
                db.txn_from_height_iterator(IteratorMode::From(&start, direction))
            } else {
                db.txn_to_height_iterator(IteratorMode::From(&start, direction))
            }
        };

        // iterate
        for (key, value) in iter.flatten() {
            if key[..PublicKey::LEN] != *pk.as_bytes() || txns.len() >= limit {
                // beyond the desired public key or limit
                break;
            }

            let state_hash = state_hash_suffix(&key)?;
            let canonical = get_block_canonicity(db, &state_hash);

            if let Some(query_canonicity) = query.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let txn = Transaction::new(serde_json::from_slice(&value)?, db, num_commands);
            if query.matches(&txn) {
                txns.push(txn);
            }
        }

        Ok(())
    }

    fn zkapp_or_not_height_or_slot_iterator<'a>(
        db: &'a Arc<IndexerStore>,
        sort_by: &'a TransactionSortByInput,
        min: u32,
        max: u32,
        zkapp: Option<bool>,
    ) -> anyhow::Result<DBIterator<'a>> {
        use TransactionSortByInput::*;

        let direction = match sort_by {
            BlockHeightAsc | DateTimeAsc | GlobalSlotAsc => Direction::Forward,
            BlockHeightDesc | DateTimeDesc | GlobalSlotDesc => Direction::Reverse,
        };

        Ok(match (sort_by, zkapp) {
            (BlockHeightAsc, None | Some(false)) => {
                db.user_commands_height_iterator(IteratorMode::From(&min.to_be_bytes(), direction))
            }
            (BlockHeightAsc, Some(true)) => {
                db.zkapp_commands_height_iterator(IteratorMode::From(&min.to_be_bytes(), direction))
            }
            (BlockHeightDesc, None | Some(false)) => {
                db.user_commands_height_iterator(IteratorMode::From(&max.to_be_bytes(), direction))
            }
            (BlockHeightDesc, Some(true)) => {
                db.zkapp_commands_height_iterator(IteratorMode::From(&max.to_be_bytes(), direction))
            }
            (GlobalSlotAsc | DateTimeAsc, None | Some(false)) => {
                db.user_commands_slot_iterator(IteratorMode::From(&min.to_be_bytes(), direction))
            }
            (GlobalSlotAsc | DateTimeAsc, Some(true)) => {
                db.zkapp_commands_slot_iterator(IteratorMode::From(&min.to_be_bytes(), direction))
            }
            (GlobalSlotDesc | DateTimeDesc, None | Some(false)) => {
                db.user_commands_slot_iterator(IteratorMode::From(&max.to_be_bytes(), direction))
            }
            (GlobalSlotDesc | DateTimeDesc, Some(true)) => {
                db.zkapp_commands_slot_iterator(IteratorMode::From(&max.to_be_bytes(), direction))
            }
        })
    }

    pub fn calculate_inclusive_height_bounds(
        block_height: [Option<u32>; 4],
        best_block_height: u32,
    ) -> anyhow::Result<(u32, u32)> {
        let block_height_gte = block_height[0];
        let block_height_gt = block_height[1];
        let block_height_lte = block_height[2];
        let block_height_lt = block_height[3];

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

    pub fn calculate_inclusive_slot_bounds(
        db: &Arc<IndexerStore>,
        global_slot: [Option<u32>; 4],
        date_time: [&Option<DateTime>; 4],
    ) -> anyhow::Result<(u32, u32)> {
        let global_slot_gt = global_slot[0];
        let global_slot_gte = global_slot[1];
        let global_slot_lt = global_slot[2];
        let global_slot_lte = global_slot[3];

        let date_time_gt = date_time[0];
        let date_time_gte = date_time[1];
        let date_time_lt = date_time[2];
        let date_time_lte = date_time[3];

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
}

/////////////////
// conversions //
/////////////////

impl TokenAccount {
    fn from(
        db: &Arc<IndexerStore>,
        pk: PublicKey,
        token: TokenAddress,
        balance_change: i64,
        increment_nonce: bool,
    ) -> Self {
        let token_symbol = db.get_token_symbol(&token).unwrap().expect("token symbol");

        Self {
            balance_change,
            increment_nonce,
            pk: pk.to_string(),
            token: token.to_string(),
            symbol: token_symbol.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionQueryInput;

    #[test]
    fn test_bounds_with_both_gte_and_gt() {
        let gte = Some(10);
        let gt = Some(8);
        let lte = None;
        let lt = None;
        let best_block_height = 100;

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

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

        let (min_bound, max_bound) = TransactionQueryInput::calculate_inclusive_height_bounds(
            [gte, gt, lte, lt],
            best_block_height,
        )
        .unwrap();

        assert_eq!(min_bound, 1);
        assert_eq!(max_bound, best_block_height);
    }
}
