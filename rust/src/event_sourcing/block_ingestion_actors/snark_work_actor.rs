use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::payloads::{BerkeleyBlockPayload, MainnetBlockPayload, SnarkWorkSummaryPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct SnarkWorkSummaryActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

#[allow(dead_code)]
impl SnarkWorkSummaryActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "SnarkWorkSummaryActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for SnarkWorkSummaryActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::MainnetBlock => {
                let block_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                for snark_job in block_payload.snark_work.iter() {
                    let payload = SnarkWorkSummaryPayload {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.to_string(),
                        timestamp: block_payload.timestamp,
                        prover: snark_job.prover.to_string(),
                        fee_nanomina: snark_job.fee_nanomina,
                    };
                    self.publish(Event {
                        event_type: EventType::SnarkWorkSummary,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
            }
            EventType::BerkeleyBlock => {
                let block_payload: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                for snark_job in block_payload.snark_work.iter() {
                    let payload = SnarkWorkSummaryPayload {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.to_string(),
                        timestamp: block_payload.timestamp,
                        prover: snark_job.prover.to_string(),
                        fee_nanomina: snark_job.fee_nanomina,
                    };
                    self.publish(Event {
                        event_type: EventType::SnarkWorkSummary,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod snark_work_actor_tests {
    use super::SnarkWorkSummaryActor;
    use crate::{
        event_sourcing::{
            berkeley_block_models::BerkeleyBlock,
            block::BlockTrait,
            block_ingestion_actors::Actor,
            events::{Event, EventType},
            mainnet_block_models::MainnetBlock,
            payloads::{BerkeleyBlockPayload, MainnetBlockPayload, SnarkWorkSummaryPayload},
            shared_publisher::SharedPublisher,
        },
        utility::get_cleaned_pcb,
    };
    use std::{fs, path::PathBuf, sync::Arc};

    #[tokio::test]
    async fn test_snark_work_summary_actor_with_multiple_snarks() -> anyhow::Result<()> {
        async fn verify_snark_work_events(receiver: &mut tokio::sync::broadcast::Receiver<Event>, block_payload: &MainnetBlockPayload, counter: &mut usize) {
            while let Ok(event) = receiver.try_recv() {
                if event.event_type == EventType::SnarkWorkSummary {
                    let published_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();

                    // Validate that each event matches the expected block height, state hash, and timestamp
                    assert_eq!(published_payload.height, block_payload.height);
                    assert_eq!(published_payload.state_hash, block_payload.state_hash);
                    assert_eq!(published_payload.timestamp, block_payload.timestamp);

                    // Ensure that the published prover and fee are from the expected snark job
                    let expected_snark = &block_payload.snark_work[*counter];
                    assert_eq!(published_payload.prover, expected_snark.prover);
                    assert_eq!(published_payload.fee_nanomina, expected_snark.fee_nanomina);

                    *counter += 1;
                }
            }
        }
        // Path to the sample file with 64 snark works
        let path = PathBuf::from("./src/event_sourcing/test_data/misc_blocks/mainnet-185-3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu.json");

        // Load and parse the JSON file to simulate the event payload
        let file_content = fs::read_to_string(&path).expect("Could not read test data file");
        let block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Invalid JSON format in test data");
        let block_payload = MainnetBlockPayload {
            height: 185,
            global_slot: 300,
            state_hash: "3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu".to_string(),
            previous_state_hash: block.get_previous_state_hash(),
            last_vrf_output: block.get_last_vrf_output(),
            user_command_count: block.get_user_commands_count(),
            snark_work_count: block.get_aggregated_snark_work().len(),
            snark_work: block.get_aggregated_snark_work(),
            timestamp: block.get_timestamp(),
            coinbase_receiver: block.get_coinbase_receiver(),
            coinbase_reward_nanomina: block.get_coinbase_reward_nanomina(),
            global_slot_since_genesis: block.get_global_slot_since_genesis(),
            user_commands: block.get_user_commands(),
            fee_transfer_via_coinbase: block.get_fee_transfers_via_coinbase(),
            fee_transfers: block.get_fee_transfers(),
            internal_command_count: block.get_internal_command_count(),
        };

        // Create shared publisher and SnarkWorkSummaryActor
        let shared_publisher = Arc::new(SharedPublisher::new(1000));
        let actor = SnarkWorkSummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Serialize MainnetBlockPayload to JSON for the event payload
        let payload_json = sonic_rs::to_string(&block_payload).unwrap();
        let event = Event {
            event_type: EventType::MainnetBlock,
            payload: payload_json,
        };

        // Call handle_event to process the MainnetBlock event with multiple snark works
        actor.handle_event(event).await;

        // Verify that 64 SnarkWorkSummary events are published
        let mut snark_work_events_received = 0;
        verify_snark_work_events(&mut receiver, &block_payload, &mut snark_work_events_received).await;

        // Ensure that 64 SnarkWorkSummaryPayload events were published
        assert_eq!(snark_work_events_received, 1, "Expected 1 SnarkWorkSummary events");

        Ok(())
    }

    #[tokio::test]
    async fn test_snark_work_summary_actor_with_berkeley_block() -> anyhow::Result<()> {
        async fn verify_snark_work_events(receiver: &mut tokio::sync::broadcast::Receiver<Event>, block_payload: &BerkeleyBlockPayload, counter: &mut usize) {
            while let Ok(event) = receiver.try_recv() {
                if event.event_type == EventType::SnarkWorkSummary {
                    let published_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();

                    // Validate that each event matches the expected block height, state hash, and timestamp
                    assert_eq!(published_payload.height, block_payload.height);
                    assert_eq!(published_payload.state_hash, block_payload.state_hash);
                    assert_eq!(published_payload.timestamp, block_payload.timestamp);

                    // Ensure that the published prover and fee are from the expected snark job
                    let expected_snark = &block_payload.snark_work[*counter];
                    assert_eq!(published_payload.prover, expected_snark.prover);
                    assert_eq!(published_payload.fee_nanomina, expected_snark.fee_nanomina);

                    *counter += 1;
                }
            }
        }
        // Load and parse the JSON file to simulate the event payload
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-409021-3NLWau54pjGtX98RyvEffWyK5NQbqkYfzuzMv1Y2TTUbbKqP7MDk.json").unwrap();
        let block: BerkeleyBlock = sonic_rs::from_str(&file_content).expect("Invalid JSON format in test data");
        let block_payload = BerkeleyBlockPayload {
            height: 185,
            state_hash: "3NLWau54pjGtX98RyvEffWyK5NQbqkYfzuzMv1Y2TTUbbKqP7MDk".to_string(),
            previous_state_hash: block.get_previous_state_hash(),
            last_vrf_output: block.get_last_vrf_output(),
            user_command_count: block.get_user_commands_count(),
            user_commands: block.get_user_commands(),
            snark_work_count: block.get_aggregated_snark_work().len(),
            zk_app_command_count: block.get_zk_app_commands_count(),
            snark_work: block.get_aggregated_snark_work(),
            fee_transfers: block.get_fee_transfers(),
            timestamp: block.get_timestamp(),
            coinbase_receiver: block.get_coinbase_receiver(),
            coinbase_reward_nanomina: block.get_coinbase_reward_nanomina(),
            global_slot_since_genesis: block.get_global_slot_since_genesis(),
        };

        // Create shared publisher and SnarkWorkSummaryActor
        let shared_publisher = Arc::new(SharedPublisher::new(1000));
        let actor = SnarkWorkSummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Serialize BerkeleyBlockPayload to JSON for the event payload
        let payload_json = sonic_rs::to_string(&block_payload).unwrap();
        let event = Event {
            event_type: EventType::BerkeleyBlock,
            payload: payload_json,
        };

        // Call handle_event to process the BerkeleyBlock event
        actor.handle_event(event).await;

        // Verify that all SnarkWorkSummary events are published
        let mut snark_work_events_received = 0;
        verify_snark_work_events(&mut receiver, &block_payload, &mut snark_work_events_received).await;

        // Ensure that the expected number of SnarkWorkSummaryPayload events were published
        assert_eq!(
            snark_work_events_received,
            block_payload.snark_work.len(),
            "Expected {} SnarkWorkSummary events, got {}",
            block_payload.snark_work.len(),
            snark_work_events_received
        );

        Ok(())
    }
}
