pub mod graphql;
pub mod rest;

pub const ENDPOINT_GRAPHQL: &str = "/graphql";

use self::{
    graphql::{build_schema, index_graphiql},
    rest::{accounts, blockchain, blocks, locked_balances::LockedBalances},
};
use crate::store::IndexerStore;
use actix_cors::Cors;
use actix_web::{guard, middleware, web, web::Data, App, HttpServer};
use async_graphql_actix_web::GraphQL;
use chrono::{DateTime, SecondsFormat, Utc};
use log::warn;
use std::{net, path::Path, sync::Arc};

fn load_locked_balances<P: AsRef<Path>>(path: Option<P>) -> LockedBalances {
    match LockedBalances::from_csv(path) {
        Ok(locked_balances) => locked_balances,
        Err(e) => {
            warn!("locked supply csv ingestion failed. {}", e);
            LockedBalances::default()
        }
    }
}

pub async fn start_web_server<A: net::ToSocketAddrs, P: AsRef<Path>>(
    state: Arc<IndexerStore>,
    addrs: A,
    locked_supply: Option<P>,
) -> std::io::Result<()> {
    let locked = Arc::new(load_locked_balances(locked_supply));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(Data::new(state.clone()))
            .app_data(Data::new(locked.clone()))
            .service(blocks::get_blocks)
            .service(blocks::get_block)
            .service(accounts::get_account)
            .service(blockchain::get_blockchain_summary)
            .service(
                web::resource(ENDPOINT_GRAPHQL)
                    .guard(guard::Post())
                    .to(GraphQL::new(build_schema(state.clone()))),
            )
            .service(
                web::resource(ENDPOINT_GRAPHQL)
                    .guard(guard::Get())
                    .to(index_graphiql),
            )
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(addrs)?
    .run()
    .await
}

/// convert epoch milliseconds to an ISO 8601 formatted date
pub(crate) fn millis_to_iso_date_string(millis: i64) -> String {
    from_timestamp_millis(millis).to_rfc3339_opts(SecondsFormat::Millis, true)
}

// convert epoch milliseconds to DateTime<Utc>
fn from_timestamp_millis(millis: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(millis).unwrap()
}
