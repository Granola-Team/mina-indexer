use std::sync::Arc;

use clap::{Parser, Subcommand};
use mina_indexer::{
    client,
    server::{self, handle_command_line_arguments},
    store::IndexerStore,
};

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

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        IndexerCommand::Client { args } => client::run(&args).await,
        IndexerCommand::Server(args) => {
            let config = handle_command_line_arguments(args).await?;
            let db = Arc::new(IndexerStore::new(&config.database_dir)?);
            tokio::spawn(server::run(config, db.clone()));
            mina_indexer::gql::start_gql(db).await.unwrap();
            Ok(())
        }
    }
}
