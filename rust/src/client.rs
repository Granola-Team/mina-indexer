use crate::constants::MAINNET_GENESIS_HASH;
use bincode::{config, Decode, Encode};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, process};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

pub const BIN_CODE_CONFIG: config::Configuration = config::standard();
pub const BUFFER_SIZE: usize = 1024;

#[derive(Parser, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    /// Query best ledger accounts
    #[clap(subcommand)]
    Accounts(Accounts),

    /// Query blocks
    #[clap(subcommand)]
    Blocks(Blocks),

    /// Query the chain
    #[clap(subcommand)]
    Chain(Chain),

    /// Create a snapshot of a running mina indexer database
    #[clap(hide = true)]
    CreateSnapshot {
        /// Full path to the new snapshot file
        #[arg(long, default_value = "./snapshot")]
        output_path: PathBuf,
    },

    /// Query best & staged ledgers
    #[clap(subcommand)]
    Ledgers(Ledgers),

    /// Query staking ledgers
    #[command(subcommand)]
    StakingLedgers(StakingLedgers),

    #[clap(hide = true)]
    Shutdown,

    /// Query SNARKs
    #[clap(subcommand)]
    Snarks(Snarks),

    /// Show the summary of a runnning mina indexer state
    Summary {
        /// Path to write the summary [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Verbose output should be redirected to a file
        #[arg(long, default_value_t = false)]
        verbose: bool,

        /// Output JSON data
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Query transactions (user commands)
    #[clap(subcommand)]
    Transactions(Transactions),

    /// Query internal commands
    #[clap(subcommand)]
    InternalCommands(InternalCommands),

    /// Query a running mina indexer for database version
    DbVersion,
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Accounts {
    /// Query accounts by public key
    PublicKey {
        /// Retrieve public key's account info
        #[arg(long)]
        public_key: String,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Blocks {
    /// Query blocks by state hash
    StateHash {
        /// Retrieve the block with given state hash
        #[arg(long)]
        state_hash: String,

        /// Path to write the block [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query the best tip block
    Best {
        /// Path to write the best tip [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query blocks by global slot number
    GlobalSlot {
        /// Retrieve the blocks in given global slot
        #[arg(long)]
        slot: String,

        /// Path to write the block [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query blocks by blockchain length
    Height {
        /// Retrieve the blocks with given blockchain length
        #[arg(long)]
        height: u32,

        /// Path to write the block [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query blocks by public key
    PublicKey {
        /// Retrieve the blocks associated with given public key
        #[arg(long)]
        public_key: String,

        /// Path to write the block [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query a block's children
    Children {
        /// Retrieve the children of the block with given state hash
        #[arg(long)]
        state_hash: String,

        /// Path to write the children [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Chain {
    /// Query the canonical chain
    Best {
        /// Number of blocks to include in this suffix
        #[arg(long, default_value_t = 10)]
        num: u32,

        /// Path to write the best chain [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Constrain chain query with a start state hash
        #[arg(long, default_value_t = MAINNET_GENESIS_HASH.into())]
        start_state_hash: String,

        /// Constrain chain query with an end state hash
        #[arg(long)]
        end_state_hash: Option<String>,

        /// Display the entire precomputed block
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Ledgers {
    /// Query the best ledger
    Best {
        /// Path to write the ledger [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Memoize the best ledger
        #[arg(long, default_value_t = false)]
        memoize: bool,
    },

    /// Query staged ledgers by (state or ledger) hash
    Hash {
        /// State or ledger hash corresponding to the ledger
        #[arg(long)]
        hash: String,

        /// Path to write the ledger [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Memoize the staged ledger
        #[arg(long, default_value_t = false)]
        memoize: bool,
    },

    /// Query canonical staged ledgers by height
    Height {
        /// Block height of the ledger
        #[arg(long)]
        height: u32,

        /// Path to write the ledger [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Memoize the staged ledger
        #[arg(long, default_value_t = false)]
        memoize: bool,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum StakingLedgers {
    /// Query staking ledgers by ledger hash
    Hash {
        /// Ledger hash corresponding to the staking ledger
        #[arg(long)]
        hash: String,

        /// Path to write the staking ledger [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Query staking ledgers by epoch
    Epoch {
        /// Epoch number of the staking ledger
        #[arg(long)]
        epoch: u32,

        /// Genesis state hash
        #[arg(long, default_value = MAINNET_GENESIS_HASH)]
        genesis_state_hash: String,

        /// Path to write the staking ledger [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Query staking ledger delegations by epoch
    Delegations {
        /// Epoch to aggregate total delegations
        #[arg(long)]
        epoch: u32,

        /// Genesis state hash
        #[arg(long, default_value = MAINNET_GENESIS_HASH)]
        genesis_state_hash: String,

        /// Path to write the aggregate delegations
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Query staking ledgers by public key
    PublicKey {
        /// Epoch to aggregate staking delegations
        #[arg(long)]
        epoch: u32,

        /// Genesis state hash
        #[arg(long, default_value = MAINNET_GENESIS_HASH)]
        genesis_state_hash: String,

        /// Account to aggregate staking delegations
        #[arg(long)]
        public_key: String,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Snarks {
    /// Query SNARK work by state hash
    StateHash {
        /// Path to write the snark work [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// State hash of block to query
        #[arg(long)]
        state_hash: String,
    },

    /// Query SNARK work by prover public key
    PublicKey {
        /// Path to write the snark work [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// State hash of block to query
        #[arg(long)]
        public_key: String,
    },

    /// Query top SNARK workers by total fees
    Top {
        /// Number of SNARK rovers to include in list
        #[arg(long, default_value_t = 10)]
        num: usize,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum Transactions {
    /// Query transactions by hash
    Hash {
        /// Hash of the transaction
        #[arg(long)]
        hash: String,

        /// Verbose transaction output
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query transactions by public key
    PublicKey {
        /// Path to write the transactions [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Retrieve public key's transaction info
        #[arg(long)]
        public_key: String,

        /// Bound the fetched transactions by a start state hash
        #[arg(long, default_value_t = MAINNET_GENESIS_HASH.into())]
        start_state_hash: String,

        /// Bound the fetched transactions by an end state hash
        #[arg(long)]
        end_state_hash: Option<String>,

        /// Format output as CSV
        #[arg(long, default_value_t = false)]
        csv: bool,

        /// Verbose transaction output
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Query transactions by state hash
    StateHash {
        /// Path to write the transactions [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// State hash of the containing block
        #[arg(long)]
        state_hash: String,

        /// Verbose transaction output
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug, Encode, Decode)]
#[command(author, version, about, long_about = None)]
pub enum InternalCommands {
    /// Query internal commands by public key
    StateHash {
        /// Path to write the internal commands [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// State hash of the containing block
        #[arg(long)]
        state_hash: String,
    },

    /// Query internal commands by block
    PublicKey {
        /// Path to write the internal commands [default: stdout]
        #[arg(long)]
        path: Option<PathBuf>,

        /// Retrieve public key's internal command info
        #[arg(long)]
        public_key: String,

        /// Format output as CSV
        #[arg(long, default_value_t = false)]
        csv: bool,
    },
}

impl ClientCli {
    pub async fn run(&self, domain_socket_path: PathBuf) -> anyhow::Result<()> {
        let conn = UnixStream::connect(domain_socket_path)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Unable to connect to the Unix domain socket server: {e}");
                process::exit(111);
            });
        let (reader, mut writer) = conn.into_split();
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::with_capacity(BUFFER_SIZE);
        let encoded = bincode::encode_to_vec(self, BIN_CODE_CONFIG)?;

        writer.write_all(&encoded).await?;
        reader.read_to_end(&mut buffer).await?;

        let msg = String::from_utf8(buffer)?;
        let msg = msg.trim_end();
        println!("{msg}");
        Ok(())
    }
}
