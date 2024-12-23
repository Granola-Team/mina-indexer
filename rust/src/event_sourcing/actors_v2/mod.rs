use super::actor_dag::{ActorFactory, ActorNode, Stateless};
use berkeley_block_actor::BerkeleyBlockActor;
use block_ancestor_actor::BlockAncestorActor;
use mainnet_block_actor::MainnetBlockParserActor;
use pcb_file_path_actor::PcbFilePathActor;
use tokio::sync::watch::Receiver;

pub(crate) mod berkeley_block_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod mainnet_block_actor;
pub(crate) mod pcb_file_path_actor;

pub fn get_actor_dag(shutdown_rx: &Receiver<bool>) -> ActorNode<Stateless> {
    // Setup root
    let mut root = PcbFilePathActor::create_actor(shutdown_rx.clone());

    root.add_child(MainnetBlockParserActor::create_actor(shutdown_rx.clone()));
    root.add_child(BerkeleyBlockActor::create_actor(shutdown_rx.clone()));
    root.add_child(BlockAncestorActor::create_actor(shutdown_rx.clone()));

    root
}
