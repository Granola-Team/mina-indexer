pub mod accounts;
pub mod blocks;
pub mod gen;
pub mod feetransfers;
pub mod stakes;
pub mod transactions;

use super::{millis_to_iso_date_string, ENDPOINT_GRAPHQL};
use crate::store::IndexerStore;
use actix_web::HttpResponse;
use async_graphql::{
    http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, InputValueError,
    InputValueResult, MergedObject, Scalar, ScalarType, Schema, Value,
};
use std::{cmp::Ordering, sync::Arc};

#[derive(MergedObject, Default)]
pub struct Root(
    blocks::BlocksQueryRoot,
    stakes::StakeQueryRoot,
    accounts::AccountQueryRoot,
    transactions::TransactionsQueryRoot,
    feetransfers::FeetransferQueryRoot,
);

/// Build the schema for the block endpoints
pub fn build_schema(store: Arc<IndexerStore>) -> Schema<Root, EmptyMutation, EmptySubscription> {
    Schema::build(Root::default(), EmptyMutation, EmptySubscription)
        .data(store)
        .finish()
}

pub async fn index_graphiql() -> actix_web::Result<HttpResponse> {
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

trait F64Ord {
    /// Returns an [Ordering] comparing this [f64] value and `other`.
    ///
    /// Defaults to [Ordering::Greater]
    fn cmp(&self, other: &f64) -> Ordering;
}

impl F64Ord for f64 {
    fn cmp(&self, other: &f64) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => Ordering::Greater,
        }
    }
}
