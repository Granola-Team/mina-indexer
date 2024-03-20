use crate::constants::MAINNET_GENESIS_HASH;
use bincode::{config, Decode, Encode};
use clap::{Parser, Subcommand};
use std::{
    path::{Path, PathBuf},
    process,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};
use tracing::instrument;

pub const BIN_CODE_CONFIG: config::Configuration = config::standard();

#[derive(Parser, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    #[clap(subcommand)]
    Accounts(Accounts),
    #[clap(subcommand)]
    Blocks(Blocks),
    #[clap(subcommand)]
    Chain(Chain),
    #[clap(subcommand)]
    Checkpoints(Checkpoints),
    #[clap(subcommand)]
    Ledger(Ledger),
    #[command(subcommand)]
    StakingLedger(StakingLedger),
    #[clap(hide = true)]
    Shutdown,
    #[clap(subcommand)]
    Snark(Snark),
    /// Show a summary of the state
    Summary {
        /// Path to write the summary [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Verbose output should be redirected to a file
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
        /// Output JSON data
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    #[clap(subcommand)]
    Transactions(Transactions),
    #[clap(subcommand)]
    InternalCommands(InternalCommands),
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
/// Query accounts
pub enum Accounts {
    PublicKey {
        /// Retrieve public key's account info
        #[arg(short = 'k', long)]
        public_key: String,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
/// Query blocks
pub enum Blocks {
    /// Query block by state hash
    StateHash {
        /// Retrieve the block with given state hash
        #[arg(short, long)]
        state_hash: String,
        /// Path to write the block [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Display the entire precomputed block
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query the best tip block
    BestTip {
        /// Path to write the best tip [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Display the entire precomputed block
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query blocks by global slot number
    Slot {
        /// Retrieve the blocks in given global slot
        #[arg(short, long)]
        slot: String,
        /// Path to write the block [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Display the entire precomputed block
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query blocks by blockchain length
    Height {
        /// Retrieve the blocks with given blockchain length
        #[arg(short, long)]
        height: String,
        /// Path to write the block [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Display the entire precomputed block
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query blocks by public key
    PublicKey {
        /// Retrieve the blocks associated with given public key
        #[arg(short = 'k', long)]
        public_key: String,
        /// Path to write the block [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Display the entire precomputed block
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum Chain {
    /// Query the best chain
    Best {
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
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum Checkpoints {
    /// Create a checkpoint of the indexer store
    Create {
        /// Path to write the checkpoint
        #[arg(short, long)]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum Ledger {
    /// Query the best ledger
    Best {
        /// Path to write the ledger [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Query ledger by hash (state or ledger)
    Hash {
        /// Path to write the ledger [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// State or ledger hash corresponding to the ledger
        #[arg(short, long)]
        hash: String,
    },
    /// Query ledger by height
    Height {
        /// Path to write the ledger [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Block height of the ledger
        #[arg(short, long)]
        height: u32,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum StakingLedger {
    /// Query staking ledger by hash
    Hash {
        /// Path to write the staking ledger [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Ledger hash corresponding to the staking ledger
        #[arg(short, long)]
        hash: String,
    },
    /// Query staking ledger at epoch
    Epoch {
        /// Path to write the staking ledger [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Epoch number of the staking ledger
        #[arg(short, long)]
        epoch: u32,
    },
    Delegations {
        /// Epoch to aggregate total delegations
        #[arg(short, long)]
        epoch: u32,
        /// Network for the staking ledger
        #[arg(short, long, default_value = "mainnet")]
        network: String,
        /// Path to write the aggregate delegations
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    PublicKey {
        /// Epoch to aggregate staking delegations
        #[arg(short, long)]
        epoch: u32,
        /// Account to aggregate staking delegations
        #[arg(short = 'k', long)]
        public_key: String,
        /// Network for the staking ledger
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum Snark {
    /// Query SNARK work by state hash
    StateHash {
        /// Path to write the snark work [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// State hash of block to query
        #[arg(short, long)]
        state_hash: String,
    },
    /// Query SNARK work by prover public key
    PublicKey {
        /// Path to write the snark work [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// State hash of block to query
        #[arg(short = 'k', long)]
        public_key: String,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum Transactions {
    /// Query transactions by their hash
    Hash {
        /// Hash of the transaction
        #[arg(short, long)]
        hash: String,
        /// Verbose transaction output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query transactions by public key
    PublicKey {
        /// Path to write the transactions [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Retrieve public key's transaction info
        #[arg(short = 'k', long)]
        public_key: String,
        /// Bound the fetched transactions by a start state hash
        #[arg(short, long, default_value_t = MAINNET_GENESIS_HASH.into())]
        start_state_hash: String,
        /// Bound the fetched transactions by an end state hash
        #[arg(short, long)]
        end_state_hash: Option<String>,
        /// Verbose transaction output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Query transactions by state hash
    StateHash {
        /// Path to write the transactions [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// State hash of the containing block
        #[arg(short, long)]
        state_hash: String,
        /// Verbose transaction output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub enum InternalCommands {
    /// Query internal commands by public key
    StateHash {
        /// Path to write the internal commands [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// State hash of the containing block
        #[arg(short, long)]
        state_hash: String,
    },
    /// Query internal commands by block
    PublicKey {
        /// Path to write the internal commands [default: stdout]
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Retrieve public key's internal command info
        #[arg(short = 'k', long)]
        public_key: String,
    },
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

    let encoded = bincode::encode_to_vec(command, BIN_CODE_CONFIG).unwrap();

    writer.write_all(&encoded).await?;
    reader.read_to_end(&mut buffer).await?;

    let msg = String::from_utf8(buffer)?;
    let msg = msg.trim_end();
    println!("{msg}");

    Ok(())
}
