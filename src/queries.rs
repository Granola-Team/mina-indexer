use std::error::Error;

use async_tungstenite::tungstenite::Message;
use futures::prelude::*;

use crate::{constants::SUBSCRIPTION_QUERY, websocket::TokioTlsWebSocketConnection};

/// Send messages to an initialized GraphQL Websocket connection
/// to tell the server to start sending subscription results
pub async fn mina_daemon_ws_init(
    conn: &mut TokioTlsWebSocketConnection,
) -> Result<(), Box<dyn Error>> {
    let connection_init_msg = Message::Text(
        r#"
            {"type":"connection_init","payload":{"X-Apollo-Tracing":"0"}}
            "#
        .to_string(),
    );
    println!(
        "Sending connection_init message:\n{:?}\n",
        connection_init_msg
    );
    conn.sink.feed(connection_init_msg).await?;

    let subscription_start_msg = Message::Text(format!(
        r#"
            {{"id":"1","type":"start","payload":{{"variables":{{}},"extensions":{{}},"operationName":"NewBlockSubscription","query":"{}"}}}}
            "#,
        SUBSCRIPTION_QUERY
    ));
    println!("Sending subscription:\n{:?}\n", subscription_start_msg);
    conn.sink.feed(subscription_start_msg).await?;
    conn.sink.flush().await?;
    Ok(())
}
