use async_tungstenite::tungstenite::Message;
use futures::prelude::*;
use mina_indexer::{
    constants::GRAPHQL_URL,
    queries::mina_daemon_ws_init,
    subchain::SubchainContext,
    websocket::{graphql_websocket, TokioTlsWebSocketConnection},
    Block,
};
use std::error::Error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let mut socket_conn = graphql_websocket(GRAPHQL_URL).await?;
    mina_daemon_ws_init(&mut socket_conn).await?;

    socket_loop(&mut socket_conn).await?;
    Ok(())
}

/// Process blocks from GQL using SubchainIndexer, print the longest chain
pub async fn socket_loop(conn: &mut TokioTlsWebSocketConnection) -> Result<(), Box<dyn Error>> {
    let connection_ack_msg = Message::Text(String::from(
        r#"{"type":"connection_ack","id":null,"payload":null}"#,
    ));

    let mut subchain_ctx = SubchainContext::new();

    while let Some(message) = conn.stream.next().await {
        if let Ok(message) = message {
            log::info!("Message Recieved...");
            if connection_ack_msg == message {
                log::debug!("{:?}", message);
                continue;
            }

            if let Message::Text(message) = message {
                if let Ok(parsed) = json::parse(&message) {
                    let json = &parsed["payload"]["data"]["newBlock"];
                    match serde_json::from_str::<Block>(&json.dump()) {
                        Ok(block) => {
                            subchain_ctx.recv_block(block);
                            println!("{:?}", subchain_ctx.longest_chain());
                        }
                        Err(err) => {
                            log::error!("{:?}", err);
                            log::debug!("{:?}", json.pretty(1))
                        }
                    }
                } else {
                    log::debug!("Message:{:?}", message);
                }
            }
        }
    }

    Ok(())
}
