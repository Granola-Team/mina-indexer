pub mod accounts;
pub mod blocks;
pub mod feetransfers;
pub mod gen;
pub mod snarks;
pub mod staged_ledgers;
pub mod stakes;
pub mod top_snarkers;
pub mod top_stakers;
pub mod transactions;
pub mod version;

use super::ENDPOINT_GRAPHQL;
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    constants::*,
    store::IndexerStore,
};
use actix_web::HttpResponse;
use anyhow::Context as aContext;
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
    top_stakers::TopStakersQueryRoot,
    top_snarkers::TopSnarkersQueryRoot,
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DateTime(pub String);

impl DateTime {
    pub fn timestamp_millis(&self) -> i64 {
        let date_time = chrono::DateTime::parse_from_rfc3339(&self.0).expect("RFC3339 date time");
        date_time.timestamp_millis()
    }
}

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

/// Convenience function for obtaining a block's canonicity
pub(crate) fn get_block_canonicity(db: &Arc<IndexerStore>, state_hash: &BlockHash) -> bool {
    use crate::canonicity::{store::CanonicityStore, Canonicity};
    db.get_block_canonicity(state_hash)
        .map(|status| matches!(status, Some(Canonicity::Canonical)))
        .unwrap_or(false)
}

pub(crate) fn get_block(db: &Arc<IndexerStore>, state_hash: &BlockHash) -> PrecomputedBlock {
    db.get_block(state_hash)
        .with_context(|| format!("block missing from store {state_hash}"))
        .unwrap()
        .unwrap()
        .0
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject)]
#[graphql(name = "PublicKey")]
pub(crate) struct PK {
    pub public_key: String,
}

#[cfg(test)]
mod tests {
    use super::DateTime;
    use crate::constants::*;

    #[test]
    fn date_time_millis() {
        assert_eq!(
            DateTime("1970-01-01T00:00:00.000Z".into()).timestamp_millis(),
            0
        );
        assert_eq!(
            DateTime("2021-03-17T00:00:00.000Z".into()).timestamp_millis(),
            1615939200000
        );
        assert_eq!(
            DateTime("2024-06-02T00:00:00.000Z".into()).timestamp_millis(),
            1717286400000
        );
        assert_eq!(
            DateTime("2024-06-03T00:00:00.000Z".into()).timestamp_millis(),
            1717372800000
        );
        assert_eq!(
            DateTime("2024-06-05T00:00:00.000Z".into()).timestamp_millis(),
            1717545600000
        );
    }

    #[test]
    fn date_time_to_global_slot() {
        assert_eq!(millis_to_global_slot(MAINNET_GENESIS_TIMESTAMP as i64), 0);
        assert_eq!(
            millis_to_global_slot(HARDFORK_GENESIS_TIMESTAMP as i64),
            564480
        );

        let dt_millis = DateTime("2024-06-02T00:00:00.000Z".into()).timestamp_millis();
        assert_eq!(millis_to_global_slot(dt_millis), 563040);

        let dt_millis = DateTime("2024-06-03T00:00:00.000Z".into()).timestamp_millis();
        assert_eq!(millis_to_global_slot(dt_millis), 563520);
    }
}
