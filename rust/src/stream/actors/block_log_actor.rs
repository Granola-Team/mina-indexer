use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{BlockLogPayload, GenesisBlockPayload, MainnetBlockPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BlockLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl BlockLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for BlockLogActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::GenesisBlock => {
                let block_payload: GenesisBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let payload = BlockLogPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    user_command_count: 0,
                    snark_work_count: 0,
                    timestamp: block_payload.unix_timestamp,
                    coinbase_receiver: String::new(),
                    coinbase_reward_nanomina: 0,
                    global_slot_since_genesis: 1,
                    last_vrf_output: block_payload.last_vrf_output,
                    is_berkeley_block: false,
                };
                self.publish(Event {
                    event_type: EventType::BlockLog,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
            EventType::MainnetBlock => {
                let block_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let payload = BlockLogPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash,
                    previous_state_hash: block_payload.previous_state_hash,
                    user_command_count: block_payload.user_command_count,
                    snark_work_count: block_payload.snark_work_count,
                    timestamp: block_payload.timestamp,
                    coinbase_receiver: block_payload.coinbase_receiver,
                    coinbase_reward_nanomina: block_payload.coinbase_reward_nanomina,
                    global_slot_since_genesis: block_payload.global_slot_since_genesis,
                    last_vrf_output: block_payload.last_vrf_output,
                    is_berkeley_block: false,
                };
                self.publish(Event {
                    event_type: EventType::BlockLog,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
            EventType::BerkeleyBlock => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_block_summary_actor_handle_event() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{BlockLogPayload, MainnetBlockPayload},
    };
    // Create a shared publisher to test if events are published
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockLogActor::new(Arc::clone(&shared_publisher));

    // Create a mock MainnetBlockPayload
    let block_payload = MainnetBlockPayload {
        height: 100,
        last_vrf_output: "some_vrf_output".to_string(),
        state_hash: "some_state_hash".to_string(),
        previous_state_hash: "previous_state_hash".to_string(),
        user_command_count: 5,
        snark_work_count: 3,
        snark_work: vec![],
        timestamp: 1623423000,
        coinbase_receiver: "receiver_public_key".to_string(),
        coinbase_reward_nanomina: 720_000_000_000,
        global_slot_since_genesis: 12345,
        ..Default::default()
    };

    // Serialize the MainnetBlockPayload to JSON for the event payload
    let payload_json = sonic_rs::to_string(&block_payload).unwrap();
    let event = Event {
        event_type: EventType::MainnetBlock,
        payload: payload_json,
    };

    // Subscribe to the shared publisher to capture published events
    let mut receiver = shared_publisher.subscribe();

    // Call handle_event to process the MainnetBlock event
    actor.handle_event(event).await;

    // Check if the BlockSummary event was published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockLog);

        // Deserialize the payload of the BlockSummary event
        let summary_payload: BlockLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Verify that the BlockSummaryPayload matches expected values
        assert_eq!(summary_payload.height, 100);
        assert_eq!(summary_payload.state_hash, "some_state_hash");
        assert_eq!(summary_payload.previous_state_hash, "previous_state_hash");
        assert_eq!(summary_payload.user_command_count, 5);
        assert_eq!(summary_payload.snark_work_count, 3);
        assert_eq!(summary_payload.timestamp, 1623423000);
        assert_eq!(summary_payload.coinbase_receiver, "receiver_public_key");
        assert_eq!(summary_payload.coinbase_reward_nanomina, 720_000_000_000);
        assert_eq!(summary_payload.global_slot_since_genesis, 12345);
        assert!(!summary_payload.is_berkeley_block);

        // Verify that the event was marked as processed
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected BlockSummary event from BlockSummaryActor.");
    }
}
