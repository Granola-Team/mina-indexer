use std::sync::Arc;

use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::FieldResult;
use juniper::RootNode;
use rocksdb::DB;

use crate::gql::schema::delegations::get_delegation_totals_from_ctx;
use crate::gql::schema::delegations::DelegationTotals;
use crate::gql::schema::stakes;
use crate::gql::schema::transaction;
use crate::gql::schema::Transaction;
use crate::gql::schema::TransactionQueryInput;
use crate::staking_ledger::StakingLedgerAccount;
use crate::store::IndexerStore;

use super::schema::StakesQueryInput;

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
        sort_by: Option<transaction::SortBy>,
    ) -> FieldResult<Vec<Transaction>> {
        Ok(transaction::get_transactions(ctx, query, limit, sort_by))
    }

    #[graphql(description = "Get staking ledger entry")]
    fn stakes(
        ctx: &Context,
        query: Option<StakesQueryInput>,
        limit: Option<i32>,
    ) -> FieldResult<Vec<StakingLedgerAccount>> {
        Ok(stakes::get_accounts(ctx, query, limit))
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
