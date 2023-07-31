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
use rocksdb::DB;

use crate::delegation_totals_store::create_delegation_totals_db;
use crate::delegation_totals_store::update_delegation_totals;
use crate::gql::root::Context;
use crate::gql::root::QueryRoot;
use crate::store::IndexerStore;

use self::schema::delegations::TotalDelegated;

pub(crate) mod root;
pub(crate) mod schema;

#[get("/graphql")]
#[allow(clippy::unused_async)]
async fn graphql_playground() -> impl Responder {
    Html(graphiql_source("/gql", None))
}

// GraphQL endpoint
#[route("/gql", method = "GET", method = "POST")]
pub async fn gql(
    db: Data<Arc<IndexerStore>>,
    delegation_totals_db: Data<Arc<DB>>,
    schema: Data<root::Schema>,
    data: Json<GraphQLRequest>,
) -> impl Responder {
    let ctx = Context::new(db.as_ref().clone(), delegation_totals_db.as_ref().clone());
    let res = data.execute(&schema, &ctx).await;
    HttpResponse::Ok().json(res)
}

// need to fix path to delegation_totals_db
pub async fn start_gql(db: Arc<IndexerStore>) -> std::io::Result<()> {
    //placeholder code for path to delegation_totals_db
    let delegation_totals_db = Arc::new(
        create_delegation_totals_db("/path/to/delegation_totals_db")
            .expect("Failed to create delegation totals DB"),
    );

    // delegation totals for the default epoch (1) here
    let epoch_number = 1;
    let staking_ledger = QueryRoot::stakingLedgerByEpoch(&ctx, epoch_number);
    let mut total_delegated = TotalDelegated(0.0);
    let mut count_delegates = 0;

    if let Some(staking_ledger) = staking_ledger {
        for account in staking_ledger.accounts {
            if let Some(delegation_totals) = &account.delegationTotals {
                total_delegated.0 += delegation_totals.totalDelegated.unwrap_or(TotalDelegated(0.0)).0;
                count_delegates += delegation_totals.countDelegates.unwrap_or(0);
            }
        }
    }

    update_delegation_totals(
        &delegation_totals_db,
        "public_key_here",
        epoch_number,
        total_delegated,
        count_delegates,
    )
    .expect("Failed to update delegation totals");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(Context::new(
                db.clone(),
                delegation_totals_db.clone(),
            )))
            .app_data(Data::new(root::create_schema()))
            .service(gql)
            .service(graphql_playground)
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
