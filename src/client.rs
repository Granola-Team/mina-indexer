use crate::constants::*;
use clap::{Args, Parser};
use futures::{
    io::{AsyncWriteExt, BufReader},
    AsyncReadExt,
};
use interprocess::local_socket::tokio::LocalSocketStream;
use serde_derive::{Deserialize, Serialize};
use std::{path::PathBuf, process};
use tracing::instrument;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Display account info for the given public key
    Account(AccountArgs),
    /// Display the best chain
    BestChain(ChainArgs),
    /// Create a checkpoint of the indexer store Speedb
    Checkpoint(CheckpointArgs),
    #[clap(flatten)]
    Ledger(LedgerArgs),
    #[clap(flatten)]
    Snark(SnarkArgs),
    /// Show summary of the state
    Summary(SummaryArgs),
    /// Shutdown the server
    Shutdown,
    #[clap(flatten)]
    Transactions(TransactionArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    /// Retrieve public key's account info
    #[arg(short = 'k', long)]
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
pub struct CheckpointArgs {
    /// Path to write the checkpoint
    #[arg(short, long)]
    path: PathBuf,
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

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum LedgerArgs {
    /// Query the best ledger
    BestLedger(BestLedgerArgs),
    /// Query ledger by state hash
    Ledger(LedgerStateHashArgs),
    /// Query ledger at height
    LedgerAtHeight(LedgerAtHeightArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerStateHashArgs {
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

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum SnarkArgs {
    /// Query a block for SNARK work
    Snark(SnarkStateHashArgs),
    /// Query a public key for all SNARK work
    SnarkPublicKey(SnarkPublickKeyArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct SnarkStateHashArgs {
    /// Path to write the snark work [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of block to query
    #[arg(long)]
    state_hash: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct SnarkPublickKeyArgs {
    /// Path to write the snark work [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of block to query
    #[arg(short = 'k', long)]
    public_key: String,
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

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum TransactionArgs {
    /// Query transactions by state hash
    TxStateHash(TransactionStateHashArgs),
    /// Query transactions by their hash
    TxHash(TransactionHashArgs),
    /// Query transactions by public key
    TxPublicKey(TransactionPublicKeyArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionStateHashArgs {
    /// Path to write the transactions [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of the contining block
    #[arg(short, long)]
    state_hash: String,
    /// Verbose transaction output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionHashArgs {
    /// Hash of the transaction
    #[arg(short, long)]
    tx_hash: String,
    /// Verbose transaction output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Output JSON data
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionPublicKeyArgs {
    /// Path to write the transactions [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Retrieve public key's transaction info
    #[arg(short = 'k', long)]
    public_key: String,
    /// Bound the fetched transactions by a number
    #[arg(short, long)]
    num: Option<usize>,
    /// Bound the fetched transactions by a start state hash
    #[arg(short, long, default_value_t = MAINNET_GENESIS_HASH.into())]
    start_state_hash: String,
    /// Bound the fetched transactions by an end state hash
    #[arg(short, long)]
    end_state_hash: Option<String>,
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

    match command {
        ClientCli::Account(account_args) => {
            let command = format!("account {}\0", account_args.public_key);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::BestChain(args) => {
            let command = format!(
                "best-chain {} {} {} {} {}\0",
                args.num.unwrap_or(0),
                args.verbose,
                args.start_state_hash,
                args.end_state_hash.clone().unwrap_or("x".into()),
                args.path.clone().unwrap_or("".into()).display()
            );
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::Checkpoint(checkpoint_args) => {
            let command = format!("checkpoint {}\0", checkpoint_args.path.display());
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::Ledger(ledger_args) => {
            let command = match ledger_args {
                LedgerArgs::BestLedger(args) => {
                    format!(
                        "best-ledger {}\0",
                        args.path.clone().unwrap_or("".into()).display()
                    )
                }
                LedgerArgs::Ledger(args) => {
                    format!(
                        "ledger {} {}\0",
                        args.hash,
                        args.path.clone().unwrap_or("".into()).display()
                    )
                }
                LedgerArgs::LedgerAtHeight(args) => {
                    format!(
                        "ledger-at-height {} {}\0",
                        args.height,
                        args.path.clone().unwrap_or("".into()).display(),
                    )
                }
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::Snark(snark_args) => {
            let command = match snark_args {
                SnarkArgs::Snark(snark_args) => {
                    format!(
                        "snark-state-hash {} {}\0",
                        snark_args.state_hash,
                        snark_args.path.clone().unwrap_or("".into()).display()
                    )
                }
                SnarkArgs::SnarkPublicKey(pk_args) => {
                    format!(
                        "snark-pk {} {}\0",
                        pk_args.public_key,
                        pk_args.path.clone().unwrap_or("".into()).display()
                    )
                }
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::Shutdown => {
            let command = "shutdown \0".to_string();
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
        }
        ClientCli::Summary(summary_args) => {
            let command = format!(
                "summary {} {} {}\0",
                summary_args.verbose,
                summary_args.json,
                summary_args.path.clone().unwrap_or("".into()).display()
            );
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
        ClientCli::Transactions(transaction_args) => {
            let command = match transaction_args {
                TransactionArgs::TxHash(args) => {
                    format!("tx-hash {} {} {}\0", args.tx_hash, args.verbose, args.json)
                }
                TransactionArgs::TxPublicKey(pk_args) => {
                    format!(
                        "tx-pk {} {} {} {} {}\0",
                        pk_args.public_key,
                        pk_args.verbose,
                        pk_args.start_state_hash,
                        pk_args.end_state_hash.clone().unwrap_or("x".into()),
                        pk_args.path.clone().unwrap_or("".into()).display(),
                    )
                }
                TransactionArgs::TxStateHash(args) => {
                    format!(
                        "tx-state-hash {} {} {}\0",
                        args.state_hash, args.verbose, args.json
                    )
                }
            };
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;

            let msg = String::from_utf8(buffer)?;
            let msg = msg.trim_end();
            println!("{msg}");
        }
    }

    Ok(())
}
