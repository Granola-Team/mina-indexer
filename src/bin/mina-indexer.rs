use clap::{Parser, Subcommand};
use mina_indexer::{client, server};
use tracing::instrument;

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    #[command(subcommand)]
    command: IndexerCommand,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Server commands
    Server(server::ServerArgs),
    /// Client commands
    Client {
        #[command(subcommand)]
        args: client::ClientCli,
    },
}

#[instrument]
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        IndexerCommand::Client { args } => client::run(&args).await,
        IndexerCommand::Server(args) => server::run(args.clone()).await,
    }
}
