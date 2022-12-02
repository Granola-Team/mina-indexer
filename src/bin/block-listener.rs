use async_tungstenite::tungstenite::Message;
use futures::prelude::*;
use mina_indexer::{
    constants::GRAPHQL_URL,
    queries::mina_daemon_ws_init,
    websocket::{graphql_websocket, TokioTlsWebSocketConnection}, Block
};
use std::error::Error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let mut socket_conn = graphql_websocket(GRAPHQL_URL).await?;
    mina_daemon_ws_init(&mut socket_conn).await?;

    socket_loop(&mut socket_conn).await?;
    Ok(())
}

/// Receive blocks from GQL and print them out!
pub async fn socket_loop(conn: &mut TokioTlsWebSocketConnection) -> Result<(), Box<dyn Error>> {
    let connection_ack_msg = Message::Text(String::from(
        r#"{"type":"connection_ack","id":null,"payload":null}"#,
    ));
    while let Some(message) = conn.stream.next().await {
        if let Ok(message) = message {
            println!("Message Recieved!...");
            if connection_ack_msg == message {
                println!("{:?}", message);
                continue; // ignore
            }

            // spicy messages -- new blocks
            if let Message::Text(message) = message {
                if let Ok(parsed) = json::parse(&message) {
                    // println!("\n{:?}\n\n", parsed["payload"]["data"]["newBlock"].pretty(1));
                    let pretty = parsed["payload"]["data"]["newBlock"].pretty(1);
                    match serde_json::from_str::<Block>(&pretty) {
                        Ok(block) => println!("{:?}", block),
                        Err(err) => {println!("{:?}", err); println!("{:?}", pretty)},
                    }
                } else {
                    println!("Got Message:\n{:?}\n", message);
                }
            }
        }
    }

    Ok(())
}
