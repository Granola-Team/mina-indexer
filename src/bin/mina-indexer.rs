use clap::{Parser, Subcommand};
use mina_indexer::{client, server};
use std::error::Error;
use tracing::instrument;

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    #[command(subcommand)]
    command: Option<IndexerCommand>,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Server commands
    Server(server::ServerArgs),
    /// Client commands
    Client {
        #[command(subcommand)]
        command: client::ClientCli,
    },
}

#[instrument]
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let indexer_folder = std::path::Path::new(concat!(env!("HOME"), "/mina-indexer"));
    if !indexer_folder.exists() {
        std::fs::create_dir(indexer_folder)?;
    }

    let args = Cli::parse();

    if let Some(arguments) = &args.command {
        match arguments {
            IndexerCommand::Client { command } => {
                client::run(command).await?;
            }
            IndexerCommand::Server(args) => {
                server::run(args.clone()).await?;
            }
        }
    }
    Ok(())
}
