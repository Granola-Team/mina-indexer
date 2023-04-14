use std::path::PathBuf;

use clap::Parser;
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use mina_indexer::{block::{receiver::BlockReceiver, precomputed::PrecomputedBlock}, state::IndexerState};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    root_block: PathBuf,
    #[arg(short, long, default_value = None)]
    logs_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = ServerArgs::parse();
    let root_block = mina_indexer::block::parse_file(&args.root_block).await?;
    let mut indexer_state =
        mina_indexer::state::IndexerState::new(&root_block, args.logs_dir.as_deref())?;
    let mut block_receiver = BlockReceiver::new().await?;

    let listener = LocalSocketListener::bind(mina_indexer::SOCKET_NAME)?;

    loop {
        tokio::select! {
            block_fut = block_receiver.recv() => {
                if let Some(block_result) = block_fut {
                    let precomputed_block = block_result?;
                    indexer_state.add_block(&precomputed_block)?;
                } else {
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                let conn = conn_fut?;
                let state = indexer_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(conn, state).await {
                        eprintln!("Error while handling connection: {}", e);
                    }
                });
            }
        }
    }
}

async fn handle_conn(conn: LocalSocketStream, state: IndexerState) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(128);
    let _read = reader.read_until(0, &mut buffer).await?;

    let string = String::from_utf8(buffer)?;
    match string.as_str() {
        "best_chain\0" => {
            println!("received best_chain command");
            let best_chain: Vec<PrecomputedBlock> = state
                .best_chain
                .into_iter()
                .map(|leaf| leaf.block.state_hash)
                .map(|state_hash| {
                    state
                        .block_store_pool
                        .as_ref()
                        .unwrap()
                        .get()
                        .unwrap()
                        .get_block(&state_hash.0)
                        .unwrap()
                        .unwrap()
                }).collect();
            let bytes = bcs::to_bytes(&best_chain)?;
            writer.write_all(&bytes).await?;
        }
        _ => return Err(anyhow::Error::msg("Malformed Request")),
    }

    Ok(())
}
