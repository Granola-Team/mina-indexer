use env_logger::Builder;
use log::error;
use mina_indexer::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode},
    actors_v2::pcb_file_path_actor::PcbFilePathActor,
    events::{Event, EventType},
    sourcing::{get_block_entries, sort_entries},
};
use std::{env, path::PathBuf};
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    let safe_root = PcbFilePathActor::create_actor(shutdown_rx);
    let sender = safe_root.lock().await.consume_sender().unwrap();

    tokio::spawn(async move {
        ActorNode::spawn_all(safe_root).await;
    });

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
}
