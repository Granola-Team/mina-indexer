use crate::{
    command::{signed::SignedCommand, Command},
    ledger::account::Account,
    state::summary::{SummaryShort, SummaryVerbose},
    MAINNET_GENESIS_HASH, SOCKET_NAME,
};
use clap::{Args, Parser};
use futures::{
    io::{AsyncWriteExt, BufReader},
    AsyncReadExt,
};
use interprocess::local_socket::tokio::LocalSocketStream;
use serde_derive::{Deserialize, Serialize};
use std::{path::PathBuf, process};
use tokio::{
    fs::write,
    io::{stdout, AsyncWriteExt as OtherAsyncWriteExt},
};
use tracing::instrument;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Display account info for the given public key
    Account(AccountArgs),
    /// Display the best chain
    BestChain(ChainArgs),
    /// Dump the best ledger to a file
    BestLedger(BestLedgerArgs),
    /// Dump the ledger at a specified state hash
    Ledger(LedgerArgs),
    /// Dump the ledger at a specified height (blockchain length)
    LedgerAtHeight(LedgerAtHeightArgs),
    /// Show summary of the state
    Summary(SummaryArgs),
    /// Shutdown the server
    Shutdown,
    /// Get transactions for an account
    Transactions(TransactionArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    /// Retrieve public key's account info
    #[arg(short, long)]
    public_key: String,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct ChainArgs {
    /// Number of blocks to include in this suffix
    #[arg(short, long)]
    num: Option<usize>,
    /// Path to write the best chain [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Constrain chain query with a start state hash
    #[arg(short, long, default_value_t = MAINNET_GENESIS_HASH.into())]
    start_state_hash: String,
    /// Constrain chain query with an end state hash
    #[arg(short, long)]
    end_state_hash: Option<String>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BestLedgerArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerAtHeightArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Block height of the ledger
    #[arg(long)]
    height: u32,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State or ledger hash corresponding to the ledger
    #[arg(long)]
    hash: String,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct SummaryArgs {
    /// Path to write the summary [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Verbose output should be redirected to a file
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionArgs {
    /// Path to write the transactions [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Retrieve public key's transaction info
    #[arg(long)]
    public_key: String,
    /// Verbose transaction output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[instrument]
pub async fn run(command: &ClientCli) -> Result<(), anyhow::Error> {
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

    async fn write_output<T>(
        input: &T,
        output_json: bool,
        path: Option<PathBuf>,
    ) -> anyhow::Result<()>
    where
        T: ?Sized + serde::Serialize + std::fmt::Display,
    {
        async fn _write(path: Option<&PathBuf>, buffer: &[u8]) -> anyhow::Result<()> {
            if let Some(path) = path {
                write(path, buffer.to_vec()).await?;
            } else {
                stdout().write_all(buffer).await?;
            }
            Ok(())
        }

        if output_json {
            _write(path.as_ref(), serde_json::to_string(&input)?.as_bytes()).await
        } else {
            _write(path.as_ref(), format!("{input}").as_bytes()).await
        }
    }

    match command {
        ClientCli::Account(account_args) => {
            let command = format!("account {}\0", account_args.public_key);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let account: Account = serde_json::from_slice(&buffer)?;
            write_output(&account, account_args.json, None).await?;
        }
        ClientCli::BestChain(chain_args) => {
            let command = format!(
                "best_chain {} {} {} {}\0",
                chain_args.num.unwrap_or(0),
                chain_args.verbose,
                chain_args.start_state_hash.clone(),
                chain_args.end_state_hash.clone().unwrap_or("x".into())
            );
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            if let Some(path) = chain_args.path.as_ref() {
                write(path, buffer.to_vec()).await?;
            } else {
                stdout().write_all(&buffer).await?;
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

            let not_available = "No summary available yet";
            if summary_args.verbose {
                if let Ok(summary) = serde_json::from_slice::<SummaryVerbose>(&buffer) {
                    write_output(&summary, summary_args.json, summary_args.path.clone()).await?;
                } else {
                    write_output(not_available, summary_args.json, summary_args.path.clone())
                        .await?;
                }
            } else if let Ok(summary) = serde_json::from_slice::<SummaryShort>(&buffer) {
                write_output(&summary, summary_args.json, summary_args.path.clone()).await?;
            } else {
                write_output(not_available, summary_args.json, summary_args.path.clone()).await?;
            }
        }
        ClientCli::Shutdown => {
            let command = "shutdown \0".to_string();
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg: String = serde_json::from_slice(&buffer)?;
            println!("{msg}");
        }
        ClientCli::Transactions(transaction_args) => {
            let command = match &transaction_args.path {
                None => format!(
                    "transactions {} {}\0",
                    transaction_args.public_key, transaction_args.verbose
                ),
                Some(path) => format!(
                    "transactions {} {} {}\0",
                    transaction_args.public_key,
                    transaction_args.verbose,
                    path.display()
                ),
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            if transaction_args.verbose {
                let msg: Vec<SignedCommand> = serde_json::from_slice(&buffer)?;
                println!("{:?}", msg)
            } else {
                let msg: Vec<Command> = serde_json::from_slice(&buffer)?;
                println!("{:?}", msg)
            }
        }
    }

    Ok(())
}
