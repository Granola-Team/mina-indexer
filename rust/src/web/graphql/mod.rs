pub mod blocks;
pub mod stakes;

use super::ENDPOINT_GRAPHQL;
use crate::store::IndexerStore;
use actix_web::HttpResponse;
use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, MergedObject, Schema};
use std::sync::Arc;

#[derive(MergedObject, Default)]
pub struct Root(blocks::BlocksQueryRoot, stakes::StakesQueryRoot);

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
