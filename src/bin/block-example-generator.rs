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

/// Output one block from the MINA GQL to
pub async fn socket_loop(conn: &mut TokioTlsWebSocketConnection) -> Result<(), Box<dyn Error>> {
    conn.stream.next().await.unwrap()?;
    conn.stream.next().await.unwrap()?;
    if let Some(message) = conn.stream.next().await {
        if let Ok(Message::Text(message)) = message {
            if let Ok(json) = json::parse(&message) {
                if let Ok(_block) = serde_json::from_str::<Block>(&json.pretty(1)) {
                    println!("{}", json.pretty(1));
                }
            }
        }
    }

    Ok(())
}
