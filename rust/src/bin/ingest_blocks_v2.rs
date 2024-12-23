use env_logger::Builder;
use log::error;
use mina_indexer::event_sourcing::{
    actor_dag::ActorNode,
    actors_v2::get_actor_dag,
    events::{Event, EventType},
    sourcing::{get_block_entries, sort_entries},
};
use std::{env, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{watch, Mutex};

#[tokio::main]
async fn main() {
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    let mut root = get_actor_dag(&shutdown_rx);
    let sender = root.consume_sender().unwrap();

    tokio::spawn(async move {
        let root = Arc::new(Mutex::new(root));
        ActorNode::spawn_all(root).await;
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

    tokio::time::sleep(Duration::from_secs(1)).await;
}
