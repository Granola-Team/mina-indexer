use crate::{
    block::{precomputed::PrecomputedBlock, Block},
    state::{
        ledger::account::Account,
        summary::{SummaryShort, SummaryVerbose},
    },
    SOCKET_NAME,
};
use clap::Parser;
use futures::{
    io::{AsyncWriteExt, BufReader},
    AsyncReadExt,
};
use interprocess::local_socket::tokio::LocalSocketStream;
use serde_derive::{Deserialize, Serialize};
use std::{path::PathBuf, process};
use tokio::io::{stdout, AsyncWriteExt as OtherAsyncWriteExt};
use tracing::instrument;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Display the account info for the given public key
    Account(AccountArgs),
    /// Display the best chain
    BestChain(ChainArgs),
    /// Dump the best ledger to a file
    BestLedger(BestLedgerArgs),
    /// Dump the ledger at a specified state_hash
    Ledger(LedgerArgs),
    /// Dump the ledger at a specified blockchain_length
    LedgerAtHeight(LedgerAtHeightArgs),
    /// Show summary of indexer state
    Summary(SummaryArgs),
    /// Signal Mina Indexer server to shutdown
    Shutdown,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    /// Retrieve this public key's account info
    #[arg(short, long)]
    public_key: String,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct ChainArgs {
    /// Number of blocks to include
    #[arg(short, long, default_value_t = 10)]
    num: usize,
    /// Path to write the best chain (default: stdout)
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Verbose displays the entire precomputed block (default: false)
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BestLedgerArgs {
    /// Path to write the ledger (default: stdout)
    #[arg(short, long)]
    path: Option<PathBuf>,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerAtHeightArgs {
    /// Path to write the ledger (default: stdout)
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Block height of the ledger
    #[arg(long)]
    height: u32,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerArgs {
    /// Path to write the ledger (default: stdout)
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Block hash corresponding to the ledger
    #[arg(long)]
    hash: String,
}

#[derive(clap::Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct SummaryArgs {
    /// Verbose output should be redirected to a file
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[instrument]
pub async fn run(command: &ClientCli, output_json: bool) -> Result<(), anyhow::Error> {
    let conn = match LocalSocketStream::connect(SOCKET_NAME).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Unable to connect to the domain socket server: {e}");
            process::exit(111);
        }
    };
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024 * 1024); // 1mb

    match command {
        ClientCli::Account(account_args) => {
            let command = format!("account {}\0", account_args.public_key);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let account: Account = serde_json::from_slice(&buffer)?;
            if output_json {
                stdout()
                    .write_all(serde_json::to_string(&account)?.as_bytes())
                    .await?;
            } else {
                stdout()
                    .write_all(format!("{account:?}").as_bytes())
                    .await?;
            }
        }
        ClientCli::BestChain(chain_args) => {
            let command = format!("best_chain {}\0", chain_args.num);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let blocks: Vec<PrecomputedBlock> = serde_json::from_slice(&buffer)?;
            for block in blocks.iter() {
                if output_json {
                    stdout()
                        .write_all(serde_json::to_string(block)?.as_bytes())
                        .await?;
                } else {
                    let block = Block::from_precomputed(block, block.blockchain_length);
                    stdout().write_all(block.summary().as_bytes()).await?;
                }
            }
        }
        ClientCli::BestLedger(best_ledger_args) => {
            let command = match &best_ledger_args.path {
                None => "best_ledger \0".to_string(),
                Some(path) => format!("best_ledger {}\0", path.display()),
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let msg = String::from_utf8(buffer)?;
            println!("{msg}");
        }
        ClientCli::Ledger(ledger_args) => {
            let command = match &ledger_args.path {
                None => format!("ledger {}\0", ledger_args.hash),
                Some(path) => format!("ledger {} {}\0", ledger_args.hash, path.display()),
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let msg = String::from_utf8(buffer)?;
            println!("{msg}");
        }
        ClientCli::LedgerAtHeight(ledger_at_height_args) => {
            let command = match &ledger_at_height_args.path {
                None => format!("ledger_at_height {}\0", ledger_at_height_args.height),
                Some(path) => format!(
                    "ledger_at_height {} {}\0",
                    ledger_at_height_args.height,
                    path.display()
                ),
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let msg = String::from_utf8(buffer)?;
            println!("{msg}");
        }
        ClientCli::Summary(summary_args) => {
            let command = format!("summary {}\0", summary_args.verbose);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            if summary_args.verbose {
                if let Ok(summary) = serde_json::from_slice::<SummaryVerbose>(&buffer) {
                    if output_json {
                        stdout()
                            .write_all(serde_json::to_string(&summary)?.as_bytes())
                            .await?;
                    } else {
                        stdout().write_all(format!("{summary}").as_bytes()).await?;
                    }
                } else {
                    stdout()
                        .write_all(serde_json::to_string("No summary available yet")?.as_bytes())
                        .await?;
                }
            } else if let Ok(summary) = serde_json::from_slice::<SummaryShort>(&buffer) {
                if output_json {
                    stdout()
                        .write_all(serde_json::to_string(&summary)?.as_bytes())
                        .await?;
                } else {
                    stdout().write_all(format!("{summary}").as_bytes()).await?;
                }
            } else {
                stdout()
                    .write_all("No summary not available yet".as_bytes())
                    .await?;
            }
        }
        ClientCli::Shutdown => {
            let command = "shutdown \0".to_string();
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let msg: String = bcs::from_bytes(&buffer)?;
            println!("{msg}");
        }
    }

    Ok(())
}
