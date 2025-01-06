use env_logger::Builder;
use log::{error, info};
use mina_indexer::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actors_v2::spawn_genesis_dag,
        events::{Event, EventType},
        sourcing::get_ledger_at_fork,
    },
};
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    // 1) Initialize logger
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
        .await
        .expect("Failed to connect to database");

    // Spawn the connection handle
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Postgres connection error: {}", e);
        }
    });

    if let Err(e) = client.execute("DROP SCHEMA public CASCADE;", &[]).await {
        eprintln!("unable to drop public schema: {}", e);
    }

    if let Err(e) = client.execute("CREATE SCHEMA public;", &[]).await {
        eprintln!("unable to public schema: {}", e);
    }

    let (dag, sender) = spawn_genesis_dag().await;

    for de in get_ledger_at_fork().get_accounting_double_entries() {
        if let Err(err) = sender
            .send(Event {
                event_type: EventType::DoubleEntryTransaction,
                payload: sonic_rs::to_string(&de).unwrap(),
            })
            .await
        {
            error!("Failed to process a double entry from the genesis ledger: {err}");
        }
    }

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    dag.lock().await.wait_until_quiesced().await;

    info!("Shutting down gracefully. Goodbye!");
}
