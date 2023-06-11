use crate::{
    block::{precomputed::PrecomputedBlock, Block},
    state::{
        ledger::{account::Account, Ledger},
        summary::Summary,
    },
    SOCKET_NAME,
};
use clap::Parser;
use futures::{
    io::{AsyncWriteExt, BufReader},
    AsyncReadExt,
};
use interprocess::local_socket::tokio::LocalSocketStream;
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Display the account info for the given public key
    Account(AccountArgs),
    /// Display the best chain
    BestChain,
    /// Dump the best ledger to a file
    BestLedger(LedgerPath),
    /// Show summary of indexer state
    Summary, // TODO
}

#[derive(clap::Args, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    #[arg(short, long)]
    public_key: String,
}

#[derive(clap::Args, Debug)]
#[command(author, version, about, long_about = None)]
pub struct LedgerPath {
    #[arg(short, long)]
    path: std::path::PathBuf,
}

pub async fn run(command: &ClientCli) -> Result<(), anyhow::Error> {
    let conn = match LocalSocketStream::connect(SOCKET_NAME).await {
        Ok(conn) => conn,
        Err(e) => {
            println!("Make sure the server has started!");
            println!("Error: {e}");
            process::exit(111);
        }
    };
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);

    let mut buffer = Vec::with_capacity(1280000);

    match command {
        ClientCli::Account(account_args) => {
            let command = format!("account {}\0", account_args.public_key);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let account: Account = bcs::from_bytes(&buffer)?;
            println!("{account:?}");
        }
        ClientCli::BestChain => {
            writer.write_all(b"best_chain\0").await?;
            reader.read_to_end(&mut buffer).await?;
            let blocks: Vec<PrecomputedBlock> = bcs::from_bytes(&buffer)?;
            blocks.iter().for_each(|block| {
                // TODO only show height and state hash
                println!(
                    "{:?}",
                    Block::from_precomputed(block, block.blockchain_length.unwrap())
                )
            });
        }
        ClientCli::BestLedger(path) => {
            let command = format!("best_ledger {}\0", path.path.display());
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let ledger: Ledger = bcs::from_bytes(&buffer)?;
            println!("{ledger:?}");
        }
        ClientCli::Summary => {
            writer.write_all(b"summary\0").await?;
            reader.read_to_end(&mut buffer).await?;
            let summary: Summary = bcs::from_bytes(&buffer)?;
            println!("{summary}");
        }
    }

    Ok(())
}
