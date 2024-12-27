use super::{
    actor_dag::{ActorDAG, ActorFactory},
    events::Event,
};
use berkeley_block_actor::BerkeleyBlockActor;
use block_ancestor_actor::BlockAncestorActor;
use block_canonicity_actor::BlockCanonicityActor;
use canonical_berkeley_block_actor::CanonicalBerkeleyBlockActor;
use canonical_mainnet_block_actor::CanonicalMainnetBlockActor;
use mainnet_block_actor::MainnetBlockParserActor;
use new_block_actor::NewBlockActor;
use pcb_file_path_actor::PcbFilePathActor;
use tokio::sync::watch;

pub(crate) mod berkeley_block_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod block_canonicity_actor;
pub(crate) mod canonical_berkeley_block_actor;
pub(crate) mod canonical_mainnet_block_actor;
pub(crate) mod mainnet_block_actor;
pub(crate) mod new_block_actor;
pub(crate) mod pcb_file_path_actor;

/// Spawns a DAG of interlinked actors and returns the `Sender<Event>` for the root actor (`PcbFilePathActor`).
pub fn spawn_actor_dag(shutdown_rx: watch::Receiver<bool>) -> tokio::sync::mpsc::Sender<Event> {
    // 1. Create a new DAG.
    let mut dag = ActorDAG::new();

    // 2. Create each actor node and capture IDs before adding them to the DAG.
    let pcb_node = PcbFilePathActor::create_actor();
    let pcb_id = pcb_node.id(); // Root node ID

    let mainnet_block_node = MainnetBlockParserActor::create_actor();
    let mainnet_block_id = mainnet_block_node.id();

    let berkeley_block_node = BerkeleyBlockActor::create_actor();
    let berkeley_block_id = berkeley_block_node.id();

    let block_ancestor_node = BlockAncestorActor::create_actor();
    let block_ancestor_id = block_ancestor_node.id();

    let new_block_node = NewBlockActor::create_actor();
    let new_block_id = new_block_node.id();

    let block_canonicity_node = BlockCanonicityActor::create_actor();
    let block_canonicity_id = block_canonicity_node.id();

    let canonical_mainnet_block_node = CanonicalMainnetBlockActor::create_actor();
    let canonical_mainnet_block_id = canonical_mainnet_block_node.id();

    let canonical_berkeley_block_node = CanonicalBerkeleyBlockActor::create_actor();
    let canonical_berkeley_block_id = canonical_berkeley_block_node.id();

    let pcb_sender = dag.set_root(pcb_node);

    dag.add_node(mainnet_block_node);
    dag.link_parent(&pcb_id, &mainnet_block_id);

    dag.add_node(berkeley_block_node);
    dag.link_parent(&pcb_id, &berkeley_block_id);

    dag.add_node(block_ancestor_node);
    dag.link_parent(&mainnet_block_id, &block_ancestor_id);
    dag.link_parent(&berkeley_block_id, &block_ancestor_id);

    dag.add_node(new_block_node);
    dag.link_parent(&block_ancestor_id, &new_block_id);

    dag.add_node(block_canonicity_node);
    dag.link_parent(&new_block_id, &block_canonicity_id);

    dag.add_node(canonical_mainnet_block_node);
    dag.link_parent(&block_canonicity_id, &canonical_mainnet_block_id);
    dag.link_parent(&mainnet_block_id, &canonical_mainnet_block_id);

    dag.add_node(canonical_berkeley_block_node);
    dag.link_parent(&block_canonicity_id, &canonical_berkeley_block_id);
    dag.link_parent(&berkeley_block_id, &canonical_berkeley_block_id);

    tokio::spawn({
        let mut dag = dag;
        async move {
            dag.spawn_all(shutdown_rx).await;
        }
    });

    // 7. Return the root actor’s Sender<Event>.
    pcb_sender
}
