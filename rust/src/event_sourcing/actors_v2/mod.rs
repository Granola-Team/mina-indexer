use super::{
    actor_dag::{ActorDAG, ActorFactory},
    events::Event,
};
use account_summary_actor::AccountSummaryActor;
use account_summary_persistence_actor::AccountSummaryPersistenceActor;
use accounting_actor::AccountingActor;
use berkeley_block_actor::BerkeleyBlockActor;
use block_ancestor_actor::BlockAncestorActor;
use block_canonicity_actor::BlockCanonicityActor;
use block_confirmations_actor::BlockConfirmationsActor;
use canonical_berkeley_block_actor::CanonicalBerkeleyBlockActor;
use canonical_mainnet_block_actor::CanonicalMainnetBlockActor;
use identity_actor::IdentityActor;
use ledger_persistence_actor::LedgerPersistenceActor;
use mainnet_block_actor::MainnetBlockParserActor;
use new_account_actor::NewAccountActor;
use new_block_actor::NewBlockActor;
use pcb_file_path_actor::PcbFilePathActor;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) mod account_summary_actor;
pub(crate) mod account_summary_persistence_actor;
pub(crate) mod accounting_actor;
pub(crate) mod berkeley_block_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod block_canonicity_actor;
pub(crate) mod block_confirmations_actor;
pub(crate) mod canonical_berkeley_block_actor;
pub(crate) mod canonical_mainnet_block_actor;
pub(crate) mod identity_actor;
pub(crate) mod ledger_persistence_actor;
pub(crate) mod mainnet_block_actor;
pub(crate) mod new_account_actor;
pub(crate) mod new_block_actor;
pub(crate) mod pcb_file_path_actor;

/// Spawns a DAG of interlinked actors and returns the `Sender<Event>` for the root actor (`PcbFilePathActor`).
pub async fn spawn_actor_dag() -> (Arc<Mutex<ActorDAG>>, tokio::sync::mpsc::Sender<Event>) {
    // 1. Create a new DAG.
    let mut dag = ActorDAG::new();

    // 2. Create each actor node and capture IDs before adding them to the DAG.
    let pcb_node = PcbFilePathActor::create_actor().await;
    let pcb_id = pcb_node.id(); // Root node ID

    let mainnet_block_node = MainnetBlockParserActor::create_actor().await;
    let mainnet_block_id = mainnet_block_node.id();

    let berkeley_block_node = BerkeleyBlockActor::create_actor().await;
    let berkeley_block_id = berkeley_block_node.id();

    let block_ancestor_node = BlockAncestorActor::create_actor().await;
    let block_ancestor_id = block_ancestor_node.id();

    let new_block_node = NewBlockActor::create_actor().await;
    let new_block_id = new_block_node.id();

    let block_canonicity_node = BlockCanonicityActor::create_actor().await;
    let block_canonicity_id = block_canonicity_node.id();

    let canonical_mainnet_block_node = CanonicalMainnetBlockActor::create_actor().await;
    let canonical_mainnet_block_id = canonical_mainnet_block_node.id();

    let canonical_berkeley_block_node = CanonicalBerkeleyBlockActor::create_actor().await;
    let canonical_berkeley_block_id = canonical_berkeley_block_node.id();

    let accounting_node = AccountingActor::create_actor().await;
    let accounting_node_id = accounting_node.id();

    let ledger_persistence_node = LedgerPersistenceActor::create_actor(true).await;
    let ledger_persistence_node_id = ledger_persistence_node.id();

    let block_confirmations_node = BlockConfirmationsActor::create_actor().await;
    let block_confirmations_node_id = block_confirmations_node.id();

    let new_account_node = NewAccountActor::create_actor().await;
    let new_account_node_id = new_account_node.id();

    let account_summary_node = AccountSummaryActor::create_actor().await;
    let account_summary_node_id = account_summary_node.id();

    let account_summary_pers_node = AccountSummaryPersistenceActor::create_actor(true).await;
    let account_summary_pers_node_id = account_summary_pers_node.id();

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

    dag.add_node(accounting_node);
    dag.link_parent(&canonical_mainnet_block_id, &accounting_node_id);
    dag.link_parent(&canonical_berkeley_block_id, &accounting_node_id);

    dag.add_node(ledger_persistence_node);
    dag.link_parent(&accounting_node_id, &ledger_persistence_node_id);

    dag.add_node(block_confirmations_node);
    dag.link_parent(&new_block_id, &block_confirmations_node_id);

    dag.add_node(new_account_node);
    dag.link_parent(&mainnet_block_id, &new_account_node_id);
    dag.link_parent(&block_confirmations_node_id, &new_account_node_id);

    // Introduce a Cycle into the graph
    dag.link_parent(&new_account_node_id, &accounting_node_id);

    dag.add_node(account_summary_node);
    dag.link_parent(&accounting_node_id, &account_summary_node_id);

    dag.add_node(account_summary_pers_node);
    dag.link_parent(&account_summary_node_id, &account_summary_pers_node_id);

    let dag = Arc::new(Mutex::new(dag));
    tokio::spawn({
        let dag = Arc::clone(&dag);
        async move {
            dag.lock().await.spawn_all().await;
        }
    });

    // 7. Return the root actor’s Sender<Event>.
    (dag, pcb_sender)
}

pub async fn spawn_genesis_dag() -> (Arc<Mutex<ActorDAG>>, tokio::sync::mpsc::Sender<Event>) {
    // 1. Create a new DAG.
    let mut dag = ActorDAG::new();

    let identity_node = IdentityActor::create_actor().await;
    let identity_node_id = identity_node.id();

    // Identity node joins several roots into a single root
    let sender = dag.set_root(identity_node);

    // Branch 1: For persisting account balances (summary)
    let account_summary_node = AccountSummaryActor::create_actor().await;
    let account_summary_node_id = account_summary_node.id();
    dag.add_node(account_summary_node);
    dag.link_parent(&identity_node_id, &account_summary_node_id);

    // Leaf
    let account_summary_pers_node = AccountSummaryPersistenceActor::create_actor(false).await;
    let account_summary_pers_node_id = account_summary_pers_node.id();
    dag.add_node(account_summary_pers_node);
    dag.link_parent(&account_summary_node_id, &account_summary_pers_node_id);

    // Branch 2: For persisting genesis ledger
    // Leaf
    let ledger_persistence_node = LedgerPersistenceActor::create_actor(false).await;
    let ledger_persistence_node_id = ledger_persistence_node.id();
    dag.add_node(ledger_persistence_node);
    dag.link_parent(&identity_node_id, &ledger_persistence_node_id);

    // Branch 3: For processing pre-existing accounts
    // Leaf
    let new_account_node = NewAccountActor::create_actor().await;
    let new_account_node_id = new_account_node.id();
    dag.add_node(new_account_node);
    dag.link_parent(&identity_node_id, &new_account_node_id);

    // Branch 4: Persist genesis block ledger
    let accounting_node = AccountingActor::create_actor().await;
    let accounting_node_id = accounting_node.id();
    dag.add_node(accounting_node);
    dag.link_parent(&identity_node_id, &accounting_node_id);

    // Leaf (for persisting ledger information coming out of genesis block itself)
    let genesis_block_ledger_persistence_node = LedgerPersistenceActor::create_actor(true).await;
    let genesis_block_ledger_persistence_node_id = genesis_block_ledger_persistence_node.id();
    dag.add_node(genesis_block_ledger_persistence_node);
    dag.link_parent(&accounting_node_id, &genesis_block_ledger_persistence_node_id);

    let account_summary_node = AccountSummaryActor::create_actor().await;
    let account_summary_node_id = account_summary_node.id();
    dag.add_node(account_summary_node);
    dag.link_parent(&accounting_node_id, &account_summary_node_id);

    // Leaf
    let account_summary_pers_node = AccountSummaryPersistenceActor::create_actor(true).await;
    let account_summary_pers_node_id = account_summary_pers_node.id();
    dag.add_node(account_summary_pers_node);
    dag.link_parent(&account_summary_node_id, &account_summary_pers_node_id);

    let dag = Arc::new(Mutex::new(dag));
    tokio::spawn({
        let dag = Arc::clone(&dag);
        async move {
            dag.lock().await.spawn_all().await;
        }
    });

    // 7. Return the root actor’s Sender<Event>.
    (dag, sender)
}
