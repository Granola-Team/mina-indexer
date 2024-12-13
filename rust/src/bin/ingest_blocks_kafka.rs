use rdkafka::{
    config::ClientConfig,
    producer::{BaseProducer, Producer},
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

fn main() {
    let brokers = "localhost:9092"; // Change this to your Kafka broker address
    let transactional_id = "unique-transactional-id"; // Must be unique per producer

    // Create the producer
    let producer = create_transactional_producer(brokers, transactional_id);

    println!("Transactional producer setup complete.");
}
