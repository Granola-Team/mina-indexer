pub(crate) mod accounting_actor;
pub(crate) mod accounts_log_actor;
pub(crate) mod berkeley_block_parser_actor;
pub(crate) mod best_block_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod block_canonicity_actor;
pub(crate) mod block_confirmations_actor;
pub(crate) mod block_log_actor;
pub(crate) mod blockchain_tree_builder_actor;
pub(crate) mod canonical_block_log_actor;
pub(crate) mod coinbase_transfer_actor;
pub(crate) mod fee_transfer_actor;
pub(crate) mod fee_transfer_via_coinbase_actor;
pub(crate) mod internal_command_canonicity_actor;
pub(crate) mod internal_command_persistence_actor;
pub(crate) mod mainnet_block_parser_actor;
pub(crate) mod new_account_actor;
pub(crate) mod pcb_path_actor;
pub(crate) mod snark_canonicity_summary_actor;
pub(crate) mod snark_summary_persistence_actor;
pub(crate) mod snark_work_actor;
pub(crate) mod transition_frontier_actor;
pub(crate) mod user_command_actor;
pub(crate) mod user_command_canonicity_actor;
pub(crate) mod user_command_persistence_actor;

use super::events::Event;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

#[async_trait]
pub trait Actor: Send + Sync {
    fn id(&self) -> String;
    fn actor_outputs(&self) -> &AtomicUsize;

    // Default implementation of `shutdown` to log the count
    fn shutdown(&self) {
        let count = self.actor_outputs().load(Ordering::SeqCst);
        println!("Actor {} output {} events/inserts before shutdown.", self.id(), count);
    }

    fn print_report(&self, description: &'static str, size: usize) {
        println!("{}: {} has size {}", self.id(), description, size);
    }

    async fn report(&self) {}

    async fn on_event(&self, event: Event) {
        self.handle_event(event).await;
    }

    fn incr_event_published(&self) {
        self.actor_outputs().fetch_add(1, Ordering::SeqCst);
    }

    // Define handle_event for specific event processing per actor
    async fn handle_event(&self, event: Event);

    fn publish(&self, event: Event);
}
