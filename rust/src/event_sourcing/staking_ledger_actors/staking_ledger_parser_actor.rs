use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    event_sourcing::{
        payloads::StakingLedgerEntryPayload,
        staking_ledger_models::{StakingEntry, StakingLedger},
    },
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use std::{
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct StakingLedgerParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl StakingLedgerParserActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "StakingLedgerParserActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for StakingLedgerParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::StakingLedgerFilePath = event.event_type {
            let file_path = event.payload.clone();
            let file_content = std::fs::read_to_string(event.payload).expect("Failed to read staking ledger file");
            let staking_ledger_entries: Vec<StakingEntry> = sonic_rs::from_str(&file_content).expect("Failed to parse staking ledger JSON");

            let filename = Path::new(&file_path);
            let (epoch, _) = extract_height_and_hash(filename);

            let staking_ledger = StakingLedger::new(staking_ledger_entries, epoch as u64);

            let stakes_map = staking_ledger.get_stakes(staking_ledger.get_total_staked());

            for (_, stake) in stakes_map.iter() {
                let payload = StakingLedgerEntryPayload {
                    epoch: stake.epoch,
                    delegate: stake.delegate.to_string(),
                    stake: stake.stake,
                    total_staked: stake.total_staked,
                    delegators_count: stake.delegators.len() as u64,
                };
                self.publish(Event {
                    event_type: EventType::StakingLedgerEntry,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod staking_ledger_parser_tests {
    use super::*;
    use crate::{
        constants::CHANNEL_MESSAGE_CAPACITY,
        event_sourcing::{
            events::{Event, EventType},
            shared_publisher::SharedPublisher,
        },
    };
    use std::{path::PathBuf, sync::Arc};

    #[tokio::test]
    async fn test_staking_ledger_parser_actor() {
        // Prepare test file path
        let file_path = PathBuf::from("./src/event_sourcing/test_data/staking_ledgers/mainnet-9-jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk.json");

        // Assert the file exists
        assert!(file_path.exists(), "Test file does not exist: {:?}", file_path);

        // Set up a shared publisher with a test channel
        let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY));

        // Create the actor
        let actor = StakingLedgerParserActor::new(shared_publisher.clone());
        let mut receiver = shared_publisher.subscribe();

        // Create a test event
        let event = Event {
            event_type: EventType::StakingLedgerFilePath,
            payload: file_path.to_str().unwrap().to_string(),
        };

        // Trigger the actor's handle_event method
        actor.handle_event(event).await;

        // Collect all events published by the actor
        let mut published_events = vec![];
        while let Ok(event) = receiver.try_recv() {
            published_events.push(event);
        }

        // Assert that the correct number of events were published
        assert_eq!(
            actor.actor_outputs().load(std::sync::atomic::Ordering::SeqCst),
            25_524,
            "The number of events published does not match the expected staking entries."
        );

        // Further assertions can be made about specific events if needed
        for published_event in published_events.iter() {
            assert_eq!(published_event.event_type, EventType::StakingLedgerEntry);
        }
    }
}
