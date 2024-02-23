pub mod rest;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::middleware;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use std::net;

use crate::store::IndexerStore;

use self::rest::accounts;
use self::rest::blockchain;
use self::rest::blocks;
use self::rest::locked_balances::LockedBalances;

fn load_locked_balances() -> anyhow::Result<LockedBalances> {
    LockedBalances::from_csv("./data/locked.csv")
}
pub async fn start_web_server<A: net::ToSocketAddrs>(
    state: Arc<IndexerStore>,
    addrs: A,
) -> std::io::Result<()> {
    let locked = Arc::new(load_locked_balances().unwrap());
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .app_data(Data::new(locked.clone()))
            .service(blocks::get_blocks)
            .service(blocks::get_block)
            .service(accounts::get_account)
            .service(blockchain::get_blockchain_summary)
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(addrs)?
    .run()
    .await
}
