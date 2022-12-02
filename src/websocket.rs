use std::error::Error;

use async_tungstenite::{
    stream::Stream,
    tokio::TokioAdapter,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
    WebSocketStream,
};
use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use native_tls::TlsConnector;
use tokio::net::TcpStream;
use tokio_native_tls::TlsConnector as AsyncTlsConnector;
use tokio_native_tls::TlsStream;

/// Container struct for very specific websocket streams
pub struct TokioTlsWebSocketConnection {
    pub sink: SplitSink<
        WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>,
        Message,
    >,
    pub stream: SplitStream<
        WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>,
    >,
}

/// Initialize a WebSocket connection using the GraphQL tranport layer
pub async fn graphql_websocket(url: &str) -> Result<TokioTlsWebSocketConnection, Box<dyn Error>> {
    // Create the GQL handshake request
    let mut request = url.into_client_request()?;
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws")?,
    );
    println!("Request: {:?}\n\n\n", request);

    // Connect to the GQL server with TLS
    let https: AsyncTlsConnector = TlsConnector::new()?.into();
    let (connection, _) =
        async_tungstenite::tokio::connect_async_with_tls_connector(request, Some(https))
            .await
            .expect(&format!("unable to connect to {:?}\n", url));
    println!("Connection: {:?}\n\n\n", connection);

    let (sink, stream) = connection.split();
    Ok(TokioTlsWebSocketConnection { sink, stream })
}
