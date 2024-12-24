use super::{
    actor_dag::{ActorFactory, ActorNode},
    events::{Event, EventType},
};
use berkeley_block_actor::BerkeleyBlockActor;
use block_ancestor_actor::BlockAncestorActor;
use mainnet_block_actor::MainnetBlockParserActor;
use new_block_actor::NewBlockActor;
use pcb_file_path_actor::PcbFilePathActor;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, watch::Receiver, Mutex};

pub(crate) mod berkeley_block_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod mainnet_block_actor;
pub(crate) mod new_block_actor;
pub(crate) mod pcb_file_path_actor;

pub fn spawn_actor_dag(shutdown_rx: &Receiver<bool>) -> Sender<Event> {
    // Setup root
    let mut root = PcbFilePathActor::create_actor(shutdown_rx.clone());
    let root_sender = root.get_sender().unwrap();

    let mut mainnet_block = MainnetBlockParserActor::create_actor(shutdown_rx.clone());
    let mut berkeley_block = BerkeleyBlockActor::create_actor(shutdown_rx.clone());

    let mut block_ancestor = BlockAncestorActor::create_actor(shutdown_rx.clone());
    block_ancestor.add_parent(EventType::BerkeleyBlock, &mut berkeley_block);
    block_ancestor.add_parent(EventType::MainnetBlock, &mut mainnet_block);

    root.add_child(mainnet_block);
    root.add_child(berkeley_block);

    let new_block = NewBlockActor::create_actor(shutdown_rx.clone());
    block_ancestor.add_child(new_block);

    // actors with multiple parents require individual spawning of DAG
    tokio::spawn(async move {
        let block_ancestor = Arc::new(Mutex::new(block_ancestor));
        ActorNode::spawn_all(block_ancestor).await;
    });

    tokio::spawn(async move {
        let root = Arc::new(Mutex::new(root));
        ActorNode::spawn_all(root).await;
    });

    root_sender
}
