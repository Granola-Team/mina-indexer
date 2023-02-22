use clap::{Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

// TODO autocomplete args

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    /// Supply a command
    #[command(subcommand)]
    command: Option<IndexerCommand>,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Run the indexer
    Run {
        /// Start indexing logs from <PATH>
        #[arg(short, long, value_name = "PATH")]
        logs_dir: PathBuf,
    },

    /// Query account data
    Account {
        /// Account commands
        #[command(subcommand)]
        command: Option<AccountCommand>,
    },

    /// Query ledger data
    Ledger {
        /// Ledger commands
        #[command(subcommand)]
        command: Option<LedgerCommand>,
    },

    /// Query voting data
    Voting {
        /// Voting commands
        #[command(subcommand)]
        command: Option<VotingCommand>,
    },

    /// Dump the current store
    Dump {
        /// Dump commands
        #[command(subcommand)]
        command: Option<DumpCommand>,
    },

    /// Configuration settings
    Config {
        /// Set logs directory
        #[arg(short, long, value_name = "PATH")]
        logs_dir: Option<PathBuf>,
    },

    /// Status of the indexer
    Status {},
}

#[derive(Subcommand, Debug)]
enum AccountCommand {
    // /// Account balance
    // Balance {
    //     /// Get account balance
    //     #[arg(short, long)]
    //     pub_key: PublicKey,
    // },

    // /// Account delegation
    // Delegation {
    //     /// Get account delegation
    //     #[arg(short, long)]
    //     pub_key: PublicKey,
    // },
}

#[derive(Subcommand, Debug)]
enum LedgerCommand {
    /// TODO Staking ledger data
    Staking {},

    /// TODO Stage ledger data
    Staged {},

    /// TODO Snarked legder data
    Snarked {},

    /// TODO Next epoch ledger data
    NextEpoch {},
}

#[derive(Subcommand, Debug)]
enum VotingCommand {
    /// List of active MIPs
    Active {},

    /// List of complete MIPs
    Complete {},

    /// List of proposed MIPs
    Propose {},

    /// Voting results of specified MIPs
    Result {
        /// Names of MIPs
        #[arg(short, long)]
        mips: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum DumpCommand {
    /// Dump everything in the store
    All {},

    /// Dump the blocks
    Blocks {},

    /// Dump the ledgers
    Ledgers {},

    /// Dump the voting data
    Voting {},
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    if let Some(arguments) = &args.command {
        match arguments {
            IndexerCommand::Run { logs_dir } => {
                println!("=== Run ===");
                // TODO check not already running
                println!("Starting mina-indexer with logs dir {logs_dir:?}");
            }
            IndexerCommand::Dump { command } => {
                println!("=== Dump ===");
                if let Some(cmd) = command {
                    println!("{cmd:?}");
                }
            }
            IndexerCommand::Account { command: _command } => {
                println!("=== Account ===");
                // if let Some(AccountCommand::Balance { pub_key }) = command {
                //     println!("Balance for {pub_key:?}");
                // } else if let Some(AccountCommand::Delegation { pub_key }) = command {
                //     println!("Delegation for {pub_key:?}");
                //     // TODO get delegation from indexer state
                // }
            }
            IndexerCommand::Ledger { command } => {
                println!("=== Ledger ===");
                if let Some(cmd) = command {
                    println!("{cmd:?}");
                }
            }
            IndexerCommand::Voting { command } => {
                println!("=== Voting ===");
                if let Some(cmd) = command {
                    match cmd {
                        VotingCommand::Result { mips } => {
                            println!("Result of mip {mips:?}");
                        }
                        cmd => {
                            println!("{cmd:?}");
                        }
                    }
                }
            }
            IndexerCommand::Config { logs_dir } => {
                println!("=== Config ===");
                if let Some(logs) = logs_dir {
                    println!("Logs dir {logs:?}");
                }
            }
            IndexerCommand::Status {} => {
                println!("=== Status ===");
                println!("Query the status of the indexer");
            }
        }
    }
    Ok(())
}
