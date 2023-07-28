use std::sync::Arc;

use crate::gql::schema::transaction;
use crate::gql::schema::Transaction;
use crate::gql::schema::TransactionQueryInput;
use crate::store::IndexerStore;
use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::FieldResult;
use juniper::RootNode;

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
        sort_by: Option<transaction::SortBy>,
    ) -> FieldResult<Vec<Transaction>> {
        Ok(transaction::get_transactions(ctx, query, limit, sort_by))
    }
}

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}
