use crate::{
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
    },
    utility::get_top_level_keys_from_json_file,
};

pub struct PcbFilePathActor;

impl ActorFactory for PcbFilePathActor {
    fn create_actor() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    let keys = get_top_level_keys_from_json_file(&event.payload).expect("file to exist");
                    if keys == vec!["data".to_string(), "version".to_string()] {
                        Some(vec![Event {
                            event_type: EventType::BerkeleyBlockPath,
                            payload: event.payload,
                        }])
                    } else {
                        Some(vec![Event {
                            event_type: EventType::MainnetBlockPath,
                            payload: event.payload,
                        }])
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod pcb_file_path_actor_tests_v2 {
    use super::PcbFilePathActor; // Your actor that implements ActorFactory
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
    };
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    /// Demonstrates how to test an actor in the DAG that produces a BerkeleyBlockPath event.
    /// Instead of using `add_receiver`, we create a separate "sink" node to capture the event.
    #[tokio::test]
    async fn test_pcb_file_path_actor_in_dag() {
        // 1. Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your PcbFilePathActor node using its ActorFactory implementation (Assuming it has an ID like "PcbFilePathActor" inside `create_actor`, or
        //    you can rename the node ID if needed.)
        let pcb_actor_node = PcbFilePathActor::create_actor();
        let pcb_actor_node_id = pcb_actor_node.id();

        // 4. Add the PcbFilePathActor to the DAG as the root. This returns a Sender<Event>.
        let pcb_actor_sender = dag.set_root(pcb_actor_node);

        // 5. Create a "TestSinkNode" to listen for BerkeleyBlockPath events coming from the actor
        let sink_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    // Capture only BerkeleyBlockPath events for testing
                    if event.event_type == EventType::BerkeleyBlockPath {
                        let mut locked_state = state.lock().await;
                        locked_state.insert("captured_event", event.payload.clone());
                    }
                    None
                })
            })
            .build();
        let sink_node_id = &sink_node.id();

        // 6. Add the sink node to the DAG, and link the PcbFilePathActor node to it
        dag.add_node(sink_node);
        dag.link_parent(&pcb_actor_node_id, sink_node_id);

        // 7. Wrap the DAG in Arc<Mutex<>> to allow spawning in the background
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the DAG in the background
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
            }
        });

        // 9. Create a temp file with "data" and "version" so that PcbFilePathActor will emit a BerkeleyBlockPath event.
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), r#"{"data": {}, "version": "1.0"}"#).unwrap();

        // 10. Send a PrecomputedBlockPath event to the PcbFilePathActor's sender
        let test_event = Event {
            event_type: EventType::PrecomputedBlockPath,
            payload: temp_file.path().to_str().unwrap().to_string(),
        };
        pcb_actor_sender.send(test_event).await.expect("Failed to send event to PcbFilePathActor");

        // 11. Give some time for the actor to process the event and propagate it
        sleep(Duration::from_millis(200)).await;

        // 12. Trigger a shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");

        // 13. Check the sink node's state for the "captured_event" from the BerkeleyBlockPath
        let dag = dag.lock().await;
        let sink_node = dag.read_node(sink_node_id.clone()).expect("TestSinkNode not found");
        let sink_state = sink_node.lock().await.get_state();
        let sink_state = sink_state.lock().await;

        // 14. Verify that the sink node indeed captured the BerkeleyBlockPath event
        let captured = sink_state.get::<String>("captured_event").expect("No captured event in sink node state");
        assert_eq!(
            captured,
            temp_file.path().to_str().unwrap(),
            "TestSinkNode should have captured the BerkeleyBlockPath event with the file path"
        );
    }
}
