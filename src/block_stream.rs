use std::error::Error;

use async_tungstenite::tungstenite::Message;
use tokio_stream::StreamExt;

use crate::{
    queries::mina_daemon_ws_init,
    websocket::{graphql_websocket, TokioTlsWebSocketConnection},
    Block,
};

pub struct BlockStream {
    conn: TokioTlsWebSocketConnection,
}

impl BlockStream {
    pub async fn new(url: &str) -> Result<Self, Box<dyn Error>> {
        let mut conn = graphql_websocket(url).await?;
        mina_daemon_ws_init(&mut conn).await?;

        while let Some(message) = conn.stream.next().await {
            if let Ok(message) = message {
                let connection_ack_msg = Message::Text(String::from(
                    r#"{"type":"connection_ack","id":null,"payload":null}"#,
                ));

                if connection_ack_msg == message {
                    log::debug!("{:?}", message);
                    log::info!("Ready to begin streaming blocks!");
                    break;
                }
            }
        }

        Ok(Self { conn })
    }

    pub async fn next(&mut self) -> Option<Option<Block>> {
        let message = self.conn.stream.next().await?;
        log::info!("Message Recieved...");
        log::debug!("{:?}", message);
        if let Ok(Message::Text(message)) = message {
            if let Ok(Ok(block)) = json::parse(&message).map(|json| {
                serde_json::from_str::<Block>(&json["payload"]["data"]["newBlock"].dump())
            }) {
                return Some(Some(block));
            }
            log::debug!("Message:{:?}", message);
            return Some(None);
        }
        None
    }
}
