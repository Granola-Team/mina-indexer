use env_logger::Builder;
use log::error;
use mina_indexer::event_sourcing::{
    actors_v2::spawn_actor_dag,
    events::{Event, EventType},
    sourcing::{get_block_entries, sort_entries},
};
use std::{env, path::PathBuf, time::Duration};
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    let sender = spawn_actor_dag(&shutdown_rx);

    let mut entries = get_block_entries(&blocks_dir).await.unwrap();
    sort_entries(&mut entries);
    for file in entries {
        if let Err(err) = sender
            .send(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: file.to_str().unwrap().to_string(),
            })
            .await
        {
            error!("{}", err);
        }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}