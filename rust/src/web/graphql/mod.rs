pub mod accounts;
pub mod blocks;
pub mod feetransfers;
pub mod gen;
pub mod snarks;
pub mod staged_ledgers;
pub mod stakes;
pub mod transactions;
pub mod version;

use super::ENDPOINT_GRAPHQL;
use crate::{constants::*, store::IndexerStore};
use actix_web::HttpResponse;
use async_graphql::{
    http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, InputValueError,
    InputValueResult, MergedObject, Scalar, ScalarType, Schema, SimpleObject, Value,
};
use std::sync::Arc;

#[derive(MergedObject, Default)]
pub struct Root(
    blocks::BlocksQueryRoot,
    stakes::StakeQueryRoot,
    accounts::AccountQueryRoot,
    transactions::TransactionsQueryRoot,
    feetransfers::FeetransferQueryRoot,
    snarks::SnarkQueryRoot,
    staged_ledgers::StagedLedgerQueryRoot,
    version::VersionQueryRoot,
);

#[derive(SimpleObject)]
pub struct Timing {
    #[graphql(name = "cliff_amount")]
    pub cliff_amount: Option<u64>,

    #[graphql(name = "cliff_time")]
    pub cliff_time: Option<u32>,

    #[graphql(name = "initial_minimum_balance")]
    pub initial_minimum_balance: Option<u64>,

    #[graphql(name = "vesting_period")]
    pub vesting_period: Option<u32>,

    #[graphql(name = "vesting_increment")]
    pub vesting_increment: Option<u64>,
}

/// Build schema for all endpoints
pub fn build_schema(store: Arc<IndexerStore>) -> Schema<Root, EmptyMutation, EmptySubscription> {
    Schema::build(Root::default(), EmptyMutation, EmptySubscription)
        .data(store)
        .finish()
}

pub async fn indexer_graphiql() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint(ENDPOINT_GRAPHQL).finish()))
}

pub(crate) fn db<'a>(ctx: &'a Context) -> &'a Arc<IndexerStore> {
    ctx.data::<Arc<IndexerStore>>()
        .expect("Database should be in the context")
}

#[derive(Debug, Clone)]
pub struct Long(pub String);

#[Scalar]
impl ScalarType for Long {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(Long(s)),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DateTime(pub String);

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(DateTime(s)),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

/// Convert epoch milliseconds to an ISO 8601 formatted [DateTime] Scalar.
pub(crate) fn date_time_to_scalar(millis: i64) -> DateTime {
    DateTime(millis_to_iso_date_string(millis))
}

pub(crate) fn get_block_canonicity(db: &Arc<IndexerStore>, state_hash: &str) -> bool {
    use crate::canonicity::{store::CanonicityStore, Canonicity};

    db.get_block_canonicity(&state_hash.to_owned().into())
        .map(|status| matches!(status, Some(Canonicity::Canonical)))
        .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq, SimpleObject)]
#[graphql(name = "PublicKey")]
pub(crate) struct PK {
    pub public_key: String,
}
