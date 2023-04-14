use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let listener = LocalSocketListener::bind(mina_indexer::SOCKET_NAME)?;

    loop {
        let conn = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_conn(conn).await {
                eprintln!("Error while handling connection: {}", e);
            }
        });
    }
}

async fn handle_conn(conn: LocalSocketStream) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(128);
    let _read = reader.read_until(0, &mut buffer).await?;

    let string = String::from_utf8(buffer)?;
    match string.as_str() {
        "ping\0" => {
            println!("received ping");
            writer.write_all(b"pong\0").await?;
        }
        _ => return Err(anyhow::Error::msg("Malformed Request")),
    }

    Ok(())
}
