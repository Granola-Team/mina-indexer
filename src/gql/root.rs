use std::sync::Arc;

use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::RootNode;
use mina_serialization_types::staged_ledger_diff::UserCommandWithStatusJson;
use mina_serialization_types::v1::UserCommandWithStatusV1;

use crate::gql::schema::Transaction;
use crate::gql::schema::TransactionQueryInput;
use crate::store::IndexerStore;
use crate::store::TransactionKey;

pub struct Context {
    pub db: Arc<IndexerStore>,
}

impl Context {
    pub fn new(db: Arc<IndexerStore>) -> Self {
        Self { db }
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
                    if transaction.date_time < *timestamp_gte {
                        continue;
                    }
                }

                if let Some(ref timestamp_lte) = query_input.date_time_lte {
                    if transaction.date_time > *timestamp_lte {
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
}

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}
