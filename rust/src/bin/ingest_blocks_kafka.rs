use rdkafka::{
    config::ClientConfig,
    error::KafkaResult,
    producer::{BaseProducer, BaseRecord, Producer},
};

// Function to create a transactional producer
fn create_transactional_producer(brokers: &str, transactional_id: &str) -> BaseProducer {
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("transactional.id", transactional_id)
        .set("enable.idempotence", "true") // Enable exactly-once semantics
        .create()
        .expect("Failed to create producer");

    // Initialize transactions
    producer.init_transactions(None).expect("Failed to initialize transactions");

    producer
}

// Function to publish messages within a transaction
fn publish_messages(producer: &BaseProducer, topic: &str) -> KafkaResult<()> {
    // Start a transaction

    for i in 0..10 {
        let payload = format!("Message {}", i + 1);
        let key = format!("key-{}", i);

        producer.begin_transaction()?;

        producer
            .send(BaseRecord::to(topic).key(&key).payload(&payload))
            .expect("Failed to enqueue message");

        producer.commit_transaction(None)?;

        println!("Published: {}", payload);
    }

    // Commit the transaction

    println!("All messages published successfully and transaction committed.");
    Ok(())
}

fn main() {
    let brokers = "localhost:9092"; // Change this to your Kafka broker address
    let transactional_id = "unique-transactional-id"; // Must be unique per producer
    let topic = "persistent-eos-topic"; // Kafka topic

    // Create the producer
    let producer = create_transactional_producer(brokers, transactional_id);

    println!("Transactional producer setup complete.");

    publish_messages(&producer, topic).unwrap();

    println!("All messages published successfully.");
}
