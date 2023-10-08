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
use std::{path::PathBuf, process, time::Duration};
use tokio::{
    io::{stdout, AsyncWriteExt as OtherAsyncWriteExt},
    time::sleep,
};
use tracing::instrument;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Display the account info for the given public key
    Account(AccountArgs),
    /// Display the best chain
    BestChain(ChainArgs),
    /// Dump the best ledger to a file
    BestLedger(LedgerArgs),
    /// Show summary of indexer state
    Summary(SummaryArgs),
    /// Save the current IndexerState to an indxr file
    SaveState { out_dir: PathBuf },
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
pub struct LedgerArgs {
    /// Path to write the ledger
    #[arg(short, long)]
    path: PathBuf,
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
            println!(
                "Make sure the server has been started and initial block ingestion has completed."
            );
            println!(
                "Initial block ingestion takes several minutes if ingesting all mainnet blocks."
            );
            println!("Error: {e}");
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
        ClientCli::BestLedger(ledger_args) => {
            let command = format!("best_ledger {}\0", ledger_args.path.display());
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let msg: String = serde_json::from_slice(&buffer)?;
            println!("{msg}");
        }
        ClientCli::Summary(summary_args) => {
            let command = format!("summary {}\0", summary_args.verbose);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            if summary_args.verbose {
                let summary: SummaryVerbose = serde_json::from_slice(&buffer)?;
                if output_json {
                    stdout()
                        .write_all(serde_json::to_string(&summary)?.as_bytes())
                        .await?;
                } else {
                    stdout().write_all(format!("{summary}").as_bytes()).await?;
                }
            } else {
                let summary: SummaryShort = serde_json::from_slice(&buffer)?;
                if output_json {
                    stdout()
                        .write_all(serde_json::to_string(&summary)?.as_bytes())
                        .await?;
                } else {
                    stdout().write_all(format!("{summary}").as_bytes()).await?;
                }
            }
        }
        ClientCli::SaveState { out_dir } => {
            if !out_dir.is_dir() {
                process::exit(100);
            }

            let command = format!("save_state {}\0", out_dir.display());
            writer.write_all(command.as_bytes()).await?;
            sleep(Duration::from_secs(2)).await;
            reader.read_to_end(&mut buffer).await?;
            let response: String = serde_json::from_slice(&buffer)?;
            stdout().write_all(response.as_bytes()).await?;
        }
    }

    Ok(())
}
