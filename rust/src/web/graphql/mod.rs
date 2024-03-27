pub mod blocks;

use super::ENDPOINT_GRAPHQL;
use actix_web::HttpResponse;
use async_graphql::http::GraphiQLSource;

pub async fn index_graphiql() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint(ENDPOINT_GRAPHQL).finish()))
}
