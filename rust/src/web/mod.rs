pub mod graphql;
pub mod rest;

pub const ENDPOINT_GRAPHQL: &str = "/graphql";

use self::{
    graphql::{build_schema, indexer_graphiql},
    rest::{accounts, blockchain, blocks, locked_balances::LockedBalances},
};
use crate::store::IndexerStore;
use actix_cors::Cors;
use actix_web::{guard, middleware, web, web::Data, App, HttpServer};
use async_graphql_actix_web::GraphQL;
use log::warn;
use std::{net, sync::Arc};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

fn load_locked_balances() -> LockedBalances {
    match LockedBalances::new() {
        Ok(locked_balances) => locked_balances,
        Err(e) => {
            warn!("locked supply csv ingestion failed. {}", e);
            LockedBalances::default()
        }
    }
}

pub async fn start_web_server<A: net::ToSocketAddrs>(
    subsys: SubsystemHandle,
    state: Arc<IndexerStore>,
    addrs: A,
) -> anyhow::Result<()> {
    let locked = Arc::new(load_locked_balances());

    let _ = HttpServer::new(move || {
        App::new()
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
                    .to(indexer_graphiql),
            )
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(addrs)
    .unwrap()
    .run()
    .cancel_on_shutdown(&subsys)
    .await;

    Ok(())
}
