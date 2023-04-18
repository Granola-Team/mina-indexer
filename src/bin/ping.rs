use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use interprocess::local_socket::tokio::LocalSocketStream;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let conn = LocalSocketStream::connect(mina_indexer::SOCKET_NAME).await?;
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);

    let mut buffer = Vec::with_capacity(128);
    writer.write_all(b"ping\0").await?;
    let _read = reader.read_until(0, &mut buffer).await?;

    let string = String::from_utf8(buffer)?;
    println!("{string}");

    Ok(())
}
