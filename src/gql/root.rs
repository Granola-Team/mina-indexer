use std::sync::Arc;

use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::RootNode;
use mina_serialization_types::staged_ledger_diff::UserCommandWithStatusJson;
use mina_serialization_types::v1::UserCommandWithStatusV1;
use rocksdb::DB;

use crate::gql::schema::delegations::get_delegation_totals_from_ctx;
use crate::gql::schema::delegations::DelegationTotals;
use crate::gql::schema::Stakes;
use crate::gql::schema::Transaction;
use crate::gql::schema::TransactionQueryInput;
use crate::staking_ledger::staking_ledger_store::StakingLedgerStore;
use crate::store::IndexerStore;
use crate::store::TransactionKey;

pub struct Context {
    pub db: Arc<IndexerStore>,
    pub delegation_totals_db: Arc<DB>,
}

impl Context {
    pub fn new(db: Arc<IndexerStore>, delegation_totals_db: Arc<DB>) -> Self {
        Self {
            db,
            delegation_totals_db,
        }
    }
}

impl juniper::Context for Context {}

pub struct QueryRoot;

#[juniper::graphql_object(Context = Context)]
impl QueryRoot {
    #[graphql(description = "Indexer version")]
    fn version() -> &str {
        "0.1.1"
    }

    #[graphql(description = "List of all transactions")]
    fn transactions(
        ctx: &Context,
        query: Option<TransactionQueryInput>,
        limit: Option<i32>,
    ) -> Vec<Transaction> {
        let limit = limit.unwrap_or(100);
        let limit_idx = usize::try_from(limit).unwrap();

        let mut transactions: Vec<Transaction> = Vec::new();

        for entry in ctx.db.iter_prefix_cf("tx", b"T") {
            let (key, value) = entry.unwrap();

            let key = TransactionKey::from_slice(&key).unwrap();
            let cmd = bcs::from_bytes::<UserCommandWithStatusV1>(&value)
                .unwrap()
                .inner();

            let transaction = Transaction::from_cmd(
                UserCommandWithStatusJson::from(cmd),
                key.height() as i32,
                key.timestamp(),
            );

            // Only apply filters if a query is provided
            if let Some(ref query_input) = query {
                if let Some(ref from) = query_input.from {
                    if transaction.from != *from {
                        continue;
                    }
                }

                if let Some(ref to) = query_input.to {
                    if transaction.to != *to {
                        continue;
                    }
                }

                if let Some(ref memos) = query_input.memos {
                    if !memos.contains(&transaction.memo) {
                        continue;
                    }
                }

                if let Some(ref timestamp_gte) = query_input.date_time_gte {
                    if transaction.dateTime < *timestamp_gte {
                        continue;
                    }
                }

                if let Some(ref timestamp_lte) = query_input.date_time_lte {
                    if transaction.dateTime > *timestamp_lte {
                        continue;
                    }
                }
            }

            transactions.push(transaction);

            // stop collecting when reaching limit
            if transactions.len() >= limit_idx {
                break;
            }
        }

        transactions
    }

    #[graphql(description = "Get staking ledger by epoch number")]
    fn stakingLedgerByEpoch(ctx: &Context, epoch_number: i32) -> Option<Stakes> {
        ctx.db
            .get_by_epoch(epoch_number as u32)
            .unwrap_or(None)
            .map(|ledger| Stakes::from_staking_ledger(&ledger))
    }

    #[graphql(description = "Get staking ledger by ledger hash")]
    fn stakingLedgerByHash(ctx: &Context, ledger_hash: String) -> Option<Stakes> {
        ctx.db
            .get_by_ledger_hash(&ledger_hash)
            .unwrap_or(None)
            .map(|ledger| Stakes::from_staking_ledger(&ledger))
    }

    #[graphql(
        description = "Get delegation totals from delegation_totals_db based on public key and epoch"
    )]
    async fn delegationTotals(
        ctx: &Context,
        public_key: String,
        epoch: i32,
    ) -> Option<DelegationTotals> {
        get_delegation_totals_from_ctx(ctx, &public_key, epoch).await
    }
}

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}
