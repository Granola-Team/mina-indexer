use std::path::PathBuf;
use std::process;

use clap::Parser;
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use mina_indexer::{
    block::{
        parser::BlockParser, precomputed::PrecomputedBlock, receiver::BlockReceiver,
        store::BlockStoreConn, BlockHash,
    },
    state::{
        branch::Leaf,
        ledger::{self, public_key::PublicKey, Ledger},
    },
};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    genesis_ledger: PathBuf,
    #[arg(short, long)]
    root_hash: String,
    #[arg(short, long)]
    startup_dir: PathBuf,
    #[arg(short, long)]
    update_dir: PathBuf,
    #[arg(short, long)]
    store_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = ServerArgs::parse();
    let genesis_ledger = match ledger::genesis::parse_file(&args.genesis_ledger).await {
        Ok(genesis_ledger) => Some(genesis_ledger),
        Err(e) => {
            eprintln!(
                "Unable to parse genesis ledger at {}: {}! Exiting.",
                args.genesis_ledger.display(),
                e
            );
            process::exit(100)
        }
    };

    let root_hash = BlockHash(args.root_hash);
    let store_dir = args.store_dir;
    let mut indexer_state = mina_indexer::state::IndexerState::new(
        root_hash,
        genesis_ledger,
        &store_dir,
    )?;

    let mut block_parser = BlockParser::new(&args.startup_dir)?;
    while let Some(block) = block_parser.next().await? {
        indexer_state.add_block(&block)?;
    }

    let mut block_receiver = BlockReceiver::new().await?;
    block_receiver.load_directory(&args.update_dir).await?;

    let listener = LocalSocketListener::bind(mina_indexer::SOCKET_NAME)?;

    dbg!(&indexer_state);

    loop {
        tokio::select! {
            block_fut = block_receiver.recv() => {
                if let Some(block_result) = block_fut {
                    let precomputed_block = block_result?;
                    indexer_state.add_block(&precomputed_block)?;
                    dbg!(&indexer_state);
                } else {
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                let conn = conn_fut?;
                let best_chain = indexer_state.best_chain.clone();

                let primary_path = store_dir.clone();
                let mut secondary_path = primary_path.clone();
                secondary_path.push(Uuid::new_v4().to_string());

                let block_store_readonly = BlockStoreConn::new_read_only(&primary_path, &secondary_path)?;
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(conn, block_store_readonly, best_chain).await {
                        eprintln!("{}", format_args!("Error while handling connection: {e}"));
                    }
                    tokio::fs::remove_dir_all(&secondary_path).await.ok();
                });
            }
        }
    }
}

async fn handle_conn(
    conn: LocalSocketStream,
    db: BlockStoreConn,
    best_chain: Vec<Leaf<Ledger>>,
) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(128);
    let _read = reader.read_until(0, &mut buffer).await?;

    let mut buffers = buffer.split(|byte| *byte == 32);
    let command = buffers.next().unwrap();

    let command_string = String::from_utf8(command.to_vec())?;
    dbg!(&command_string);
    match command_string.as_str() {
        "best_chain\0" => {
            println!("received best_chain command");
            dbg!(best_chain.clone());
            let best_chain: Vec<PrecomputedBlock> = best_chain[..best_chain.len() - 1]
                .iter()
                .cloned()
                .map(|leaf| leaf.block.state_hash)
                .map(|state_hash| db.get_block(&state_hash.0).unwrap().unwrap())
                .collect();
            let bytes = bcs::to_bytes(&best_chain)?;
            writer.write_all(&bytes).await?;
        }
        "account_balance" => {
            let data_buffer = buffers.next().unwrap();
            let public_key = PublicKey::from_address(&String::from_utf8(
                data_buffer[..data_buffer.len() - 1].to_vec(),
            )?)?;
            if let Some(block) = best_chain.first() {
                let account = block.get_ledger().accounts.get(&public_key);
                dbg!(block.get_ledger());
                if let Some(account) = account {
                    let bytes = bcs::to_bytes(account)?;
                    writer.write_all(&bytes).await?;
                }
            }
        }
        _ => return Err(anyhow::Error::msg("Malformed Request")),
    }

    Ok(())
}
