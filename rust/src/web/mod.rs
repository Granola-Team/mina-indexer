pub mod graphql;
pub mod rest;

use self::{
    graphql::{build_schema, index_graphiql},
    rest::{accounts, blockchain, blocks, locked_balances::LockedBalances},
};
use crate::store::IndexerStore;
use actix_cors::Cors;
use actix_web::{guard, middleware, web, web::Data, App, HttpServer};
use async_graphql_actix_web::GraphQL;
use std::{net, path::Path, sync::Arc};
use tracing::warn;

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
    let schema = build_schema(state.clone());
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .app_data(Data::new(locked.clone()))
            .service(blocks::get_blocks)
            .service(blocks::get_block)
            .service(accounts::get_account)
            .service(blockchain::get_blockchain_summary)
            .service(
                web::resource("/graphql")
                    .guard(guard::Post())
                    .to(GraphQL::new(schema.clone())),
            )
            .service(
                web::resource("/graphql")
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
