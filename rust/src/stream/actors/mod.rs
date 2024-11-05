pub(crate) mod pcb_path_actor;

use super::events::Event;
use async_trait::async_trait;

#[async_trait]
pub trait Actor {
    async fn on_event(&self, event: Event);
    fn publish(&self, event: Event);
    fn id(&self) -> String;
}
