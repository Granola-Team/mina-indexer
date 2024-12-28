use env_logger::Builder;
use log::{error, info};
use mina_indexer::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actors_v2::{spawn_genesis_ledger_dag, spawn_preexisting_account_dag},
        events::{Event, EventType},
        sourcing::get_genesis_ledger,
    },
};
use std::sync::Arc;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    // 1) Initialize logger
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
        .await
        .expect("Failed to connect to the database");

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    if let Err(e) = client.execute("DROP SCHEMA public CASCADE", &[]).await {
        error!("Unable to drop/create schema {e}");
    }
    if let Err(e) = client.execute("CREATE SCHEMA public", &[]).await {
        error!("Unable to drop/create schema {e}");
    }

    let dag1 = {
        let (dag, sender) = spawn_genesis_ledger_dag().await;

        for de in get_genesis_ledger().get_accounting_double_entries() {
            if let Err(_err) = sender
                .send(Event {
                    event_type: EventType::DoubleEntryTransaction,
                    payload: sonic_rs::to_string(&de).unwrap(),
                })
                .await
            {
                error!("Failed to process a double entry from the genesis ledger");
            }
        }

        Arc::clone(&dag)
    };

    let dag2 = {
        let (dag, sender) = spawn_preexisting_account_dag().await;

        for account in get_genesis_ledger().get_accounts() {
            if let Err(_err) = sender
                .send(Event {
                    event_type: EventType::PreExistingAccount,
                    payload: account,
                })
                .await
            {
                error!("Failed to process a pre-existing account");
            };
        }

        Arc::clone(&dag)
    };

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    dag1.lock().await.wait_until_quiesced().await;
    dag2.lock().await.wait_until_quiesced().await;

    info!("Shutting down gracefully. Goodbye!");
}
