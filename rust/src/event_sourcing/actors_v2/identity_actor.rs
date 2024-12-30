use crate::event_sourcing::actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore};
use async_trait::async_trait;

pub struct IdentityActor;

#[async_trait]
impl ActorFactory for IdentityActor {
    async fn create_actor() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| Box::pin(async move { Some(vec![event]) }))
            .build()
    }
}
