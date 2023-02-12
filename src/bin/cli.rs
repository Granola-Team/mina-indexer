use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Calculate example
    example: bool,

    /// Optional subchain flag
    subchain: Option<String>,

    /// Optional graph visualization tool
    visualization: bool,

    /// Sets custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn on debugging info
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Does testing things
    Test {
        /// List test values
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // check the value provided by positional arguments or option arguments
    let example = cli.example;
    println!("Value for example: {}", example);

    let visualization = cli.visualization;
    println!("Value for visualization: {}", visualization);

    if let Some(subchain) = cli.subchain.as_deref() {
        println!("Value for subchain: {}", subchain);
    }

    if let Some(config_path) = cli.config.as_deref() {
        println!("Value for config: {}", config_path.display());
    }

    // see how many times a particular flag or argument is occurred
    // note: only flags can have multiple occurrences
    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // check existence of subcommands
    // if found use matches just as a top level cmd
    match &cli.command {
        Some(Commands::Test { list }) => {
            if *list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        }
        None => {}
    }

    // TODO
}
