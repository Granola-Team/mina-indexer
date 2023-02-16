use std::error::Error;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use mina_indexer::state::ledger::PublicKey;

// TODO autocomplete args
// TODO default args

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = None)]
struct Cli {
    /// Optionally supply a command
    #[command(subcommand)]
    command: Option<IndexerCommand>,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Query account data
    Account {
        /// Account-related commands
        #[command(subcommand)]
        command: Option<AccountCommand>,
    },

    /// Query block data
    Block {
        /// Block-related commands
        #[command(subcommand)]
        command: Option<BlockCommand>,
    },

    /// Query chain data
    Chain {
        /// Chain-related commands
        #[command(subcommand)]
        command: Option<ChainCommand>,
    },

    /// Query consensus data
    Consensus {
        /// Consensus-related commands
        #[command(subcommand)]
        command: Option<ConsensusCommand>,
    },

    /// Query ledger data
    Ledger {
        /// Ledger-related commands
        #[command(subcommand)]
        command: Option<LedgerCommand>,
    },

    /// Query zkapp data
    Zkapp {
        /// Zkapp-related commands
        #[command(subcommand)]
        command: Option<ZkappCommand>,
    },

    /// Configuration settings
    Config {
        /// Set logs directory
        #[arg(short, long, value_name = "PATH")]
        logs: Option<PathBuf>,

        /// Set config file path
        #[arg(short, long, value_name = "PATH")]
        config: Option<PathBuf>,
    },

    /// Run tests
    Test {
        /// Run provided tests
        #[arg(short, long, value_name = "PATH")]
        tests: Vec<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum AccountCommand {
    /// Account balance
    Balance {
        /// Get account balance
        #[arg(short, long, value_name = "PUBLIC_KEY")]
        balance: Option<PublicKey>,
    },

    /// Account delegation
    Delegation {
        /// Get account delegation
        #[arg(short, long, value_name = "PUBLIC_KEY")]
        delegation: Option<PublicKey>,
    },

    /// Account transactions back to boundary
    Transaction {
        /// Get account transactions
        #[arg(short, long, value_name = "PUBLIC_KEY")]
        transaction: Option<PublicKey>,
    },
}

#[derive(Subcommand, Debug)]
enum BlockCommand {
    Canonical {
        /// Get the slot number
        #[arg(short, long, value_name = "BLOCK_HASH")]
        canonical: Option<String>,
    },

    Highest {
        /// Choose the network: mainnet, berkeley, etc.
        #[arg(short, long)]
        highest: (),
    },

    Boundary {
        /// Choose the network: mainnet, berkeley, etc.
        #[arg(short, long)]
        boundary: (),
    },

    // TODO
}

#[derive(Subcommand, Debug)]
enum ChainCommand {
    /// Chain network
    Network {
        /// Choose the network: mainnet, berkeley, etc.
        #[arg(short, long, value_name = "NAME")]
        network: String,
    },

    /// Best known tip
    BestTip {
        /// Get the best known tip
        #[arg(short, long)]
        best_known_tip: (),
    },

    /// Global slot since genesis
    GlobalSlot {
        /// Get global
        #[arg(short, long)]
        global_slot_since_genesis: (),
    },
}

#[derive(Subcommand, Debug)]
enum ConsensusCommand {
    MinWindowDensity {},
    // TODO
}

#[derive(Subcommand, Debug)]
enum LedgerCommand {
    Staged {},
    Staking {},
    // TODO
}

#[derive(Subcommand, Debug)]
enum ZkappCommand {
    /// Network
    Network {        
        /// Choose the network: mainnet, berkeley, devnet, etc.
        #[arg(short, long)]
        network: String,
    },

    /// State
    State {
        /// Get on-chain state
        #[arg(short, long, value_name = "PUBLIC_KEY")]
        state: PublicKey,
    },
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    if let Some(arguments) = &args.command {
        match arguments {
            IndexerCommand::Test { tests } => {
                println!("=== Tests ===");
                println!("{:?}", tests);
            }
            IndexerCommand::Account { command } => {
                println!("=== Account ===");
                if let Some(AccountCommand::Balance { balance }) = command {
                    println!("balance for {:?}", balance);
                } else if let Some(AccountCommand::Delegation { delegation }) = command {
                    println!("delegation for {:?}", delegation);
                } else if let Some(AccountCommand::Transaction { transaction }) = command {
                    println!("transactions for {:?}", transaction);
                }
            }
            IndexerCommand::Block { command } => {
                println!("=== Block ===");
                if let Some(cmd) = command {
                    match cmd {
                        BlockCommand::Boundary { boundary } => {
                            println!("boundary = {:?}", boundary);
                        }
                        BlockCommand::Canonical { canonical } => {
                            println!("canonical = {:?}", canonical);
                        }
                        BlockCommand::Highest { highest } => {
                            println!("highest = {:?}", highest);
                        }
                    }
                }
            }
            IndexerCommand::Chain { command } => {
                println!("=== Chain ===");
                if let Some(cmd) = command {
                    match cmd {
                        ChainCommand::BestTip { best_known_tip } => {
                            println!("boundary = {:?}", best_known_tip);
                        }
                        ChainCommand::GlobalSlot { global_slot_since_genesis } => {
                            println!("canonical = {:?}", global_slot_since_genesis);
                        }
                        ChainCommand::Network { network } => {
                            println!("highest = {:?}", network);
                        }
                    }
                }
            }
            IndexerCommand::Consensus { command } => {
                println!("=== Consensus ===");
                println!("{:?}", command);
                if let Some(cmd) = command {
                    match cmd {
                        ConsensusCommand::MinWindowDensity {  } => {
                            println!("{:?}", cmd)
                        },
                    }
                }
            }
            IndexerCommand::Ledger { command } => {
                println!("=== Ledger ===");
                println!("{:?}", command);
                if let Some(cmd) = command {
                    match cmd {
                        LedgerCommand::Staged {  } => {
                            println!("{:?}", cmd)
                        },
                        LedgerCommand::Staking {  } => {
                            println!("{:?}", cmd)
                        },
                    }
                }

            }
            IndexerCommand::Zkapp { command } => {
                println!("=== Zkapp ===");
                if let Some(cmd) = command {
                    match cmd {
                        ZkappCommand::Network { network } => {
                            println!("network = {:?}", network);
                        }
                        ZkappCommand::State { state } => {
                            println!("address = {:?}", state);
                        }
                    }
                }
            }
            IndexerCommand::Config { logs, config } => {
                println!("=== Config ===");
                let pair = (logs, config);
                if let (Some(l), Some(c)) = pair {
                    println!("Config (1/2): logs {:?}", l);
                    println!("Config (2/2): config {:?}", c);
                } else if let (Some(l), _) = pair {
                    println!("Config (1/1): logs {:?}", l);
                } else if let (_, Some(c)) = pair {
                    println!("Config (1/1): config {:?}", c);
                } else {
                    println!("Config: empty");
                }
            }
        }
    }
    Ok(())
}
