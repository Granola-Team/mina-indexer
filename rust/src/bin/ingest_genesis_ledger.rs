use env_logger::Builder;
use log::{error, info};
use mina_indexer::event_sourcing::{
    actors_v2::spawn_genesis_ledger_dag,
    events::{Event, EventType},
    sourcing::get_genesis_ledger,
};

#[tokio::main]
async fn main() {
    // 1) Initialize logger
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    // 2) Spawn your actor DAG, which returns a Sender<Event>
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

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    dag.lock().await.wait_until_quiesced().await;

    info!("Shutting down gracefully. Goodbye!");
}
