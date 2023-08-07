use clap::{Parser, Subcommand};
use mina_indexer::{
    client,
    server::{self, create_dir_if_non_existent, handle_command_line_arguments},
    store::IndexerStore,
};
use std::{fs, path::PathBuf, sync::Arc};
use tracing::debug;
use tracing_subscriber::prelude::*;

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
            let option_snapshot_path = args.snapshot_path.clone();
            let database_dir = args.database_dir.clone();
            let log_dir = args.log_dir.clone();
            let log_level = args.log_level;
            let log_level_stdout = args.log_level_stdout;
            let config = handle_command_line_arguments(args).await?;

            let mut log_number = 0;
            let mut log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
            create_dir_if_non_existent(log_dir.to_str().unwrap()).await;
            while tokio::fs::metadata(&log_file).await.is_ok() {
                log_number += 1;
                log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
            }
            let log_file = PathBuf::from(log_file);

            // setup tracing
            if let Some(parent) = log_file.parent() {
                create_dir_if_non_existent(parent.to_str().unwrap()).await;
            }

            let log_file = std::fs::File::create(log_file.clone())?;
            let file_layer = tracing_subscriber::fmt::layer().with_writer(log_file);

            let stdout_layer = tracing_subscriber::fmt::layer();
            tracing_subscriber::registry()
                .with(stdout_layer.with_filter(log_level_stdout))
                .with(file_layer.with_filter(log_level))
                .init();

            let db = if let Some(snapshot_path) = option_snapshot_path {
                let indexer_store = IndexerStore::from_backup(&snapshot_path, &database_dir)?;
                Arc::new(indexer_store)
            } else {
                if database_dir.exists() {
                    debug!("Deleting the existing db and creating a fresh one just for you!");
                    fs::remove_dir_all(database_dir.clone())?;
                }
                Arc::new(IndexerStore::new(&database_dir)?)
            };
            tokio::spawn(server::run(config, db.clone()));
            mina_indexer::gql::start_gql(db).await.unwrap();
            Ok(())
        }
    }
}
