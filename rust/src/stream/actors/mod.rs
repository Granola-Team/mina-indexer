pub(crate) mod berkeley_block_parser_actor;
pub(crate) mod block_ancestor_actor;
pub(crate) mod mainnet_block_parser_actor;
pub(crate) mod pcb_path_actor;

use super::events::Event;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

#[async_trait]
pub trait Actor: Send + Sync {
    fn id(&self) -> String;
    fn events_processed(&self) -> &AtomicUsize;

    // Default implementation of `shutdown` to log the count
    fn shutdown(&self) {
        let count = self.events_processed().load(Ordering::SeqCst);
        println!("Actor {} processed {} events before shutdown.", self.id(), count);
    }

    async fn on_event(&self, event: Event) {
        self.handle_event(event).await;
    }

    fn incr_event_processed(&self) {
        self.events_processed().fetch_add(1, Ordering::SeqCst);
    }

    // Define handle_event for specific event processing per actor
    async fn handle_event(&self, event: Event);

    fn publish(&self, event: Event);
}
