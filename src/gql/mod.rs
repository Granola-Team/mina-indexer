use std::sync::Arc;

use actix_cors::Cors;
use actix_web::get;
use actix_web::middleware;
use actix_web::route;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Responder;
use actix_web_lab::respond::Html;
use juniper::http::graphiql::graphiql_source;
use juniper::http::GraphQLRequest;

use crate::gql::root::Context;
use crate::store::IndexerStore;

mod root;
mod schema;

#[get("/graphql")]
#[allow(clippy::unused_async)]
async fn graphql_playground() -> impl Responder {
    Html(graphiql_source("/gql", None))
}

/// GraphQL endpoint
#[route("/gql", method = "GET", method = "POST")]
pub async fn gql(
    db: Data<Arc<IndexerStore>>,
    schema: Data<root::Schema>,
    data: Json<GraphQLRequest>,
) -> impl Responder {
    let ctx = Context::new(db.as_ref().clone());
    let res = data.execute(&schema, &ctx).await;
    HttpResponse::Ok().json(res)
}

pub async fn start_gql(db: Arc<IndexerStore>) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db.clone()))
            .app_data(Data::new(root::create_schema()))
            .service(gql)
            .service(graphql_playground)
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
