use std::error::Error;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use mina_indexer::state::ledger::PublicKey;

// TODO autocomplete args
// TODO default args
// TODO config file

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = None)]
struct Cli {
    /// Supply a command
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

    /// Query voting data
    Voting {
        /// Voting-related commands
        #[command(subcommand)]
        command: Option<VotingCommand>,
    },

    /// Configuration settings
    Config {
        /// Set logs directory
        #[arg(short, long, value_name = "PATH")]
        log_dir: Option<PathBuf>,

        /// Set config file path
        #[arg(short, long, value_name = "PATH")]
        config_file: Option<PathBuf>,
    },

    /// Start the indexer
    Start {},

    /// Run tests
    Test {
        /// Run provided tests
        #[arg(short, long)]
        paths: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum AccountCommand {
    /// Account balance
    Balance {
        /// Get account balance
        #[arg(short, long)]
        pub_key: Option<PublicKey>,
    },

    /// Account delegation
    Delegation {
        /// Get account delegation
        #[arg(short, long)]
        pub_key: Option<PublicKey>,
    },

    /// Account transactions back to boundary
    Transactions {
        /// Get account transactions
        #[arg(short, long)]
        pub_key: Option<PublicKey>,
    },
}

#[derive(Subcommand, Debug)]
enum BlockCommand {
    /// Check canonical status
    Canonical {
        /// Get the slot number
        #[arg(short, long)]
        block_hash: Option<String>,
    },

    /// Highest block we know about
    Highest {},

    /// Get the boundary before which we don't keep constructed blocks
    Boundary {},

    // TODO
}

#[derive(Subcommand, Debug)]
enum ChainCommand {
    /// Chain network
    Network {
        /// Choose the network: mainnet, berkeley, etc.
        #[arg(short, long)]
        name: String,
    },

    /// Best known tip
    BestTip {},

    /// Global slot since genesis
    GlobalSlotSinceGenesis {},

    // TODO
}

#[derive(Subcommand, Debug)]
enum ConsensusCommand {
    /// Minimum window density
    MinWindowDensity {},
    
    // TODO
}

#[derive(Subcommand, Debug)]
enum LedgerCommand {
    /// Staking ledger data
    Staking {},
    
    /// Stage ledger data
    Staged {},
    
    /// Snarked legder data
    Snarked {},
    
    /// Next epoch ledger data
    NextEpoch {},
    
    // TODO
}

#[derive(Subcommand, Debug)]
enum ZkappCommand {
    /// Network commands
    Network {        
        /// Choose the network: mainnet, berkeley, devnet, etc.
        #[arg(short, long)]
        name: String,
    },

    /// State commands
    State {
        /// Get on-chain state
        #[arg(short, long)]
        pub_key: PublicKey,
    },

    // TODO
}

#[derive(Subcommand, Debug)]
enum VotingCommand {
    /// List active MIPs
    Active {},

    /// List complete MIPs
    Complete {},

    /// List proposed MIPs
    Propose {},

    /// Voting results of specified MIPs
    Result {
        /// Name of MIP
        #[arg(long)]
        mip: String,
    },
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    if let Some(arguments) = &args.command {
        match arguments {
            IndexerCommand::Start {} => {
                println!("=== Start ===");
            }
            IndexerCommand::Account { command } => {
                println!("=== Account ===");
                if let Some(AccountCommand::Balance { pub_key }) = command {
                    println!("Balance for {:?}", pub_key);
                } else if let Some(AccountCommand::Delegation { pub_key }) = command {
                    println!("Delegation for {:?}", pub_key);
                }
            }
            IndexerCommand::Block { command } => {
                println!("=== Block ===");
                if let Some(cmd) = command {
                    match cmd {
                        BlockCommand::Boundary {} => {
                            println!("Boundary");
                        }
                        BlockCommand::Canonical { block_hash } => {
                            println!("Canonical.block_hash = {:?}", block_hash);
                        }
                        BlockCommand::Highest {} => {
                            println!("Highest");
                        }
                    }
                }
            }
            IndexerCommand::Chain { command } => {
                println!("=== Chain ===");
                if let Some(cmd) = command {
                    match cmd {
                        ChainCommand::BestTip {} => {
                            println!("BestTip");
                        }
                        ChainCommand::GlobalSlotSinceGenesis {} => {
                            println!("GlobalSlotSinceGenesis");
                        }
                        ChainCommand::Network { name } => {
                            println!("Network.name = {:?}", name);
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
                        LedgerCommand::Staking {} => {
                            println!("{:?}", cmd)
                        },
                        LedgerCommand::Staged {} => {
                            println!("{:?}", cmd)
                        },
                        LedgerCommand::Snarked {} => {
                            println!("{:?}", cmd)  
                        },
                        LedgerCommand::NextEpoch {} => {
                            println!("{:?}", cmd)
                        },
                    }
                }

            }
            IndexerCommand::Zkapp { command } => {
                println!("=== Zkapp ===");
                if let Some(cmd) = command {
                    match cmd {
                        ZkappCommand::Network { name } => {
                            println!("network = {:?}", name);
                        }
                        ZkappCommand::State { pub_key } => {
                            println!("address = {:?}", pub_key);
                        }
                    }
                }
            }
            IndexerCommand::Voting { command } => {
                println!("=== Voting ===");
                if let Some(cmd) = command {
                    match cmd {
                        VotingCommand::Active {} => {
                            println!("Active");
                        }
                        VotingCommand::Complete {} => {
                            println!("Complete");
                        }
                        VotingCommand::Propose {} => {
                            println!("Propose");
                        }
                        VotingCommand::Result { mip } => {
                            println!("Result of mip {}", mip);
                        }
                    }
                }
            }
            IndexerCommand::Config { log_dir, config_file } => {
                println!("=== Config ===");
                let pair = (log_dir, config_file);
                if let (Some(logs), Some(config)) = pair {
                    println!("Logs {:?}", logs);
                    println!("Config file {:?}", config);
                } else if let (Some(logs), _) = pair {
                    println!("Logs {:?}", logs);
                } else if let (_, Some(config)) = pair {
                    println!("Config file {:?}", config);
                }
            }
            IndexerCommand::Test { paths } => {
                println!("=== Test ===");
                println!("{:?}", paths);
            }
        }
    }
    Ok(())
}
