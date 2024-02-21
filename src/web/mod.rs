pub mod rest;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::middleware;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;

use crate::store::IndexerStore;

use self::rest::accounts;
use self::rest::blocks;

pub async fn start_web_server(state: Arc<IndexerStore>) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(blocks::get_blocks)
            .service(blocks::get_block)
            .service(accounts::get_account)
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
