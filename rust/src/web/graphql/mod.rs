//! GraphQL server & helpers

mod date_time;
mod long;

pub mod accounts;
pub mod actions;
pub mod blocks;
pub mod events;
pub mod feetransfers;
pub mod gen;
pub mod snarks;
pub mod staged_ledgers;
pub mod stakes;
pub mod tokens;
pub mod top_snarkers;
pub mod top_stakers;
pub mod transactions;
pub mod version;

use super::ENDPOINT_GRAPHQL;
use crate::{
    base::state_hash::StateHash,
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    store::IndexerStore,
};
use actix_web::HttpResponse;
use anyhow::Context as aContext;
use async_graphql::{
    http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, MergedObject, Schema,
    SimpleObject,
};
use date_time::DateTime;
use long::Long;
use serde::Serialize;
use std::sync::Arc;

#[derive(MergedObject, Default)]
pub struct Root(
    blocks::BlocksQueryRoot,
    stakes::StakesQueryRoot,
    accounts::AccountQueryRoot,
    transactions::TransactionsQueryRoot,
    feetransfers::FeetransferQueryRoot,
    snarks::SnarkQueryRoot,
    staged_ledgers::StagedLedgerQueryRoot,
    tokens::TokensQueryRoot,
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

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
#[graphql(name = "PublicKey")]
pub struct PK {
    pub public_key: String,
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

/// Convert epoch milliseconds to an ISO 8601 formatted [DateTime] Scalar.
pub(crate) fn date_time_to_scalar(millis: i64) -> DateTime {
    DateTime(millis_to_iso_date_string(millis))
}

/// Convenience function for obtaining a block's canonicity
pub(crate) fn get_block_canonicity(db: &Arc<IndexerStore>, state_hash: &StateHash) -> bool {
    use crate::canonicity::{store::CanonicityStore, Canonicity};
    db.get_block_canonicity(state_hash)
        .map(|status| matches!(status, Some(Canonicity::Canonical)))
        .unwrap_or(false)
}

pub(crate) fn get_block(db: &Arc<IndexerStore>, state_hash: &StateHash) -> PrecomputedBlock {
    db.get_block(state_hash)
        .with_context(|| format!("block missing from store {state_hash}"))
        .unwrap()
        .unwrap()
        .0
}
