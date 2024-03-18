use crate::constants::MAINNET_GENESIS_HASH;
use clap::{Args, Parser};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};
use tracing::instrument;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Query account by public key
    Account(AccountArgs),
    #[command(subcommand)]
    Block(BlockArgs),
    #[command(subcommand)]
    Chain(ChainArgs),
    /// Create a checkpoint of the indexer store
    Checkpoint(CheckpointArgs),
    #[command(subcommand)]
    Ledger(LedgerArgs),
    #[command(subcommand)]
    StakingLedger(StakingLedgerArgs),
    #[clap(hide = true)]
    Shutdown,
    #[clap(subcommand)]
    Snark(SnarkArgs),
    /// Show a summary of the state
    Summary(SummaryArgs),
    #[command(subcommand)]
    Transactions(TransactionArgs),
    #[clap(flatten)]
    InternalCommand(InternalCommandArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    /// Retrieve public key's account info
    #[arg(short = 'k', long)]
    public_key: String,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum BlockArgs {
    /// Query block by state hash
    Block(BlockStateHashArgs),
    /// Query the best tip block
    BestTip(BestTipArgs),
    /// Query blocks by global slot number
    BlocksAtSlot(BlocksAtSlotArgs),
    /// Query blocks by blockchain length
    BlocksAtHeight(BlocksAtHeightArgs),
    /// Query blocks by public key
    BlocksAtPublicKey(BlocksAtPublicKeyArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BestTipArgs {
    /// Path to write the best tip [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BlockStateHashArgs {
    /// Retrieve the block with given state hash
    #[arg(short, long)]
    state_hash: String,
    /// Path to write the block [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BlocksAtHeightArgs {
    /// Retrieve the blocks with given blockchain length
    #[arg(short, long)]
    height: String,
    /// Path to write the block [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BlocksAtSlotArgs {
    /// Retrieve the blocks in given global slot
    #[arg(short, long)]
    slot: String,
    /// Path to write the block [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BlocksAtPublicKeyArgs {
    /// Retrieve the blocks associated with given public key
    #[arg(short = 'k', long)]
    public_key: String,
    /// Path to write the block [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Display the entire precomputed block
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum ChainArgs {
    /// Query the best chain
    BestChain(BestChainArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BestChainArgs {
    /// Number of blocks to include in this suffix
    #[arg(short, long, default_value_t = 10)]
    num: usize,
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
pub struct CheckpointArgs {
    /// Path to write the checkpoint
    #[arg(short, long)]
    path: PathBuf,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum LedgerArgs {
    /// Query the best ledger
    BestLedger(BestLedgerArgs),
    /// Query ledger by state hash
    Ledger(LedgerHashArgs),
    /// Query ledger by height
    LedgerAtHeight(LedgerAtHeightArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct BestLedgerArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerAtHeightArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Block height of the ledger
    #[arg(short, long)]
    height: u32,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct LedgerHashArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State or ledger hash corresponding to the ledger
    #[arg(short, long)]
    hash: String,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum StakingLedgerArgs {
    /// Query staking delegations by epoch
    StakingDelegations(StakingDelegationsArgs),
    /// Query staking delegations by public key and epoch
    StakingPublicKey(StakingPublicKeyArgs),
    /// Query staking ledger by hash
    StakingLedgerHash(StakingLedgerHashArgs),
    /// Query staking ledger by epoch
    StakingLedgerEpoch(StakingLedgerEpochArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct StakingLedgerEpochArgs {
    /// Path to write the staking ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Epoch number of the staking ledger
    #[arg(short, long)]
    epoch: u32,
    /// Network
    #[arg(short, long, default_value = "mainnet")]
    network: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct StakingLedgerHashArgs {
    /// Path to write the ledger [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Ledger hash corresponding to the staking ledger
    #[arg(short, long)]
    hash: String,
    /// Network
    #[arg(short, long, default_value = "mainnet")]
    network: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct StakingPublicKeyArgs {
    /// Epoch to aggregate staking delegations
    #[arg(short, long)]
    epoch: u32,
    /// Account to aggregate staking delegations
    #[arg(short = 'k', long)]
    public_key: String,
    /// Network for the staking ledger
    #[arg(short, long, default_value = "mainnet")]
    network: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct StakingDelegationsArgs {
    /// Epoch to aggregate total delegations
    #[arg(short, long)]
    epoch: u32,
    /// Network for the staking ledger
    #[arg(short, long, default_value = "mainnet")]
    network: String,
    /// Path to write the aggregate delegations
    #[arg(short, long)]
    path: Option<PathBuf>,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum SnarkArgs {
    /// Query SNARK work by state hash
    Snark(SnarkStateHashArgs),
    /// Query SNARK work by prover public key
    SnarkPublicKey(SnarkPublickKeyArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct SnarkStateHashArgs {
    /// Path to write the snark work [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of block to query
    #[arg(short, long)]
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
    /// Query transactions by their hash
    TxHash(TransactionHashArgs),
    /// Query transactions by public key
    TxPublicKey(TransactionPublicKeyArgs),
    /// Query transactions by state hash
    TxStateHash(TransactionStateHashArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionStateHashArgs {
    /// Path to write the transactions [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of the containing block
    #[arg(short, long)]
    state_hash: String,
    /// Verbose transaction output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
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
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct TransactionPublicKeyArgs {
    /// Path to write the transactions [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Query public key transaction info
    #[arg(short = 'k', long)]
    public_key: String,
    /// Bound the fetched transactions by a start block
    #[arg(short, long, default_value_t = MAINNET_GENESIS_HASH.into())]
    start_state_hash: String,
    /// Bound the fetched transactions by an end block
    #[arg(short, long)]
    end_state_hash: Option<String>,
    /// Verbose transaction output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum InternalCommandArgs {
    /// Query internal commands by public key
    InternalStateHash(InternalCommandsStateHashArgs),
    /// Query internal commands by block
    InternalPublicKey(InternalCommandsPublicKeyArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct InternalCommandsStateHashArgs {
    /// Path to write the internal commands [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// State hash of the containing block
    #[arg(short, long)]
    state_hash: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct InternalCommandsPublicKeyArgs {
    /// Path to write the internal commands [default: stdout]
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Retrieve public key's internal command info
    #[arg(short = 'k', long)]
    public_key: String,
}

#[instrument]
pub async fn run(command: &ClientCli, domain_socket_path: &Path) -> anyhow::Result<()> {
    let conn = match UnixStream::connect(domain_socket_path).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Unable to connect to the Unix domain socket server: {}", e);
            process::exit(111);
        }
    };
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024 * 1024); // 1mb

    let command: String = match command {
        ClientCli::Account(__) => {
            format!("account {:?}\0", __)
        }
        ClientCli::Chain(__) => match __ {
            ChainArgs::BestChain(__) => format!(
                "best-chain {} {} {} {} {}\0",
                __.num,
                __.verbose,
                __.start_state_hash,
                __.end_state_hash.clone().unwrap_or("x".into()),
                __.path.clone().unwrap_or_default().display()
            ),
        },
        ClientCli::Block(__) => match __ {
            BlockArgs::BestTip(__) => format!(
                "block-best-tip {} {}\0",
                __.verbose,
                __.path.clone().unwrap_or_default().display()
            ),
            BlockArgs::Blocks(__) => format!(
                "block-state-hash {} {} {}\0",
                __.state_hash,
                __.verbose,
                __.path.clone().unwrap_or_default().display()
            ),
            BlockArgs::BlocksAtHeight(__) => format!(
                "blocks-at-height {} {} {}\0",
                __.height,
                __.verbose,
                __.path.clone().unwrap_or_default().display()
            ),
            BlockArgs::BlocksAtSlot(__) => format!(
                "blocks-at-slot {} {} {}\0",
                __.slot,
                __.verbose,
                __.path.clone().unwrap_or_default().display()
            ),
            BlockArgs::BlocksAtPublicKey(__) => format!(
                "blocks-at-public-key {} {} {}\0",
                __.public_key,
                __.verbose,
                __.path.clone().unwrap_or_default().display()
            ),
        },
        ClientCli::Checkpoint(__) => {
            format!("checkpoint {}\0", __.path.display())
        }
        ClientCli::Ledger(__) => match __ {
            LedgerArgs::BestLedger(__) => {
                format!(
                    "best-ledger {}\0",
                    __.path.clone().unwrap_or_default().display()
                )
            }
            LedgerArgs::Ledger(__) => {
                format!(
                    "ledger {} {}\0",
                    __.hash,
                    __.path.clone().unwrap_or_default().display()
                )
            }
            LedgerArgs::LedgerAtHeight(__) => {
                format!(
                    "ledger-at-height {} {}\0",
                    __.height,
                    __.path.clone().unwrap_or_default().display(),
                )
            }
        },
        ClientCli::StakingLedger(__) => match __ {
            StakingLedgerArgs::StakingDelegations(__) => {
                format!(
                    "staking-delegations {} {} {}\0",
                    __.network,
                    __.epoch,
                    __.path.clone().unwrap_or_default().display()
                )
            }
            StakingLedgerArgs::StakingPublicKey(__) => {
                format!(
                    "staking-delegations-pk {} {} {}\0",
                    __.network, __.epoch, __.public_key
                )
            }
            StakingLedgerArgs::StakingLedgerHash(__) => {
                format!(
                    "staking-ledger-hash {} {}\0",
                    __.hash,
                    __.path.clone().unwrap_or_default().display()
                )
            }
            StakingLedgerArgs::StakingLedgerEpoch(__) => {
                format!(
                    "staking-ledger-epoch {} {}\0",
                    __.epoch,
                    __.path.clone().unwrap_or_default().display()
                )
            }
        },
        ClientCli::Snark(__) => match __ {
            SnarkArgs::Snark(__) => {
                format!(
                    "snark-state-hash {} {}\0",
                    __.state_hash,
                    __.path.clone().unwrap_or_default().display()
                )
            }
            SnarkArgs::SnarkPublicKey(__) => {
                format!(
                    "snark-pk {} {}\0",
                    __.public_key,
                    __.path.clone().unwrap_or_default().display()
                )
            }
        },
        ClientCli::Shutdown => "shutdown \0".to_string(),
        ClientCli::Summary(__) => {
            format!(
                "summary {} {} {}\0",
                __.verbose,
                __.json,
                __.path.clone().unwrap_or_default().display()
            )
        }
        ClientCli::Transaction(__) => match __ {
            TransactionArgs::TxHash(__) => {
                format!("tx-hash {} {}\0", __.tx_hash, __.verbose)
            }
            TransactionArgs::TxPublicKey(__) => {
                format!(
                    "tx-pk {} {} {} {} {}\0",
                    __.public_key,
                    __.verbose,
                    __.start_state_hash,
                    __.end_state_hash.clone().unwrap_or("x".into()),
                    __.path.clone().unwrap_or_default().display(),
                )
            }
            TransactionArgs::TxStateHash(__) => {
                format!("tx-state-hash {} {}\0", __.state_hash, __.verbose)
            }
        },
        ClientCli::InternalCommand(internal_cmd_args) => match internal_cmd_args {
            InternalCommandArgs::InternalPublicKey(args) => {
                format!(
                    "internal-pk {} {}\0",
                    args.public_key,
                    args.path.clone().unwrap_or_default().display(),
                )
            }
            InternalCommandArgs::InternalStateHash(args) => {
                format!(
                    "internal-state-hash {} {}\0",
                    args.state_hash,
                    args.path.clone().unwrap_or_default().display(),
                )
            }
        },
    };

    writer.write_all(command.as_bytes()).await?;
    reader.read_to_end(&mut buffer).await?;

    let msg = String::from_utf8(buffer)?;
    let msg = msg.trim_end();
    println!("{msg}");

    Ok(())
}
