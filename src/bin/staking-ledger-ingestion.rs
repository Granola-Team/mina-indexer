use clap::Parser;

use glob::glob;

use mina_indexer::{
    staking_ledger::{
        staking_ledger_store::StakingLedgerStore, StakingLedger, StakingLedgerAccount,
    },
    store::IndexerStore,
};

use std::{ffi::OsStr, fs::File, io::Read, path::PathBuf, time::Instant, u32::MAX};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    staking_ledgers_dir: PathBuf,
    #[arg(short, long, default_value = concat!(env!("HOME"), "/.mina-indexer/database"))]
    database_dir: PathBuf,
}

/// extract epoch from filename
fn get_epoch(file_name: &OsStr) -> Option<u32> {
    file_name
        .to_str()?
        .split('-')
        .fold(None, |acc, x| match x.parse::<u32>() {
            Err(_) => acc,
            Ok(x) => Some(x),
        })
}

fn main() {
    let args = Args::parse();
    let staking_ledgers_dir = args.staking_ledgers_dir;
    let database_dir = args.database_dir;
    let db = IndexerStore::new(&database_dir).unwrap();

    let pattern = format!("{}/*.json", staking_ledgers_dir.display());

    let mut paths: Vec<PathBuf> = glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(|x| x.ok())
        .collect();

    paths.sort_by(|x, y| {
        get_epoch(x.file_name().unwrap())
            .unwrap_or(MAX)
            .cmp(&get_epoch(y.file_name().unwrap()).unwrap_or(MAX))
    });

    let mut count = 0;
    let start_time = Instant::now();

    for path in paths {
        let epoch = get_epoch(path.file_name().unwrap()).unwrap();
        let ledger_hash = "foobar".to_string();
        let display = path.display();
        let mut file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, why),
            Ok(file) => file,
        };
        let mut bytes = Vec::new();
        let _ = file.read_to_end(&mut bytes);

        let accounts = match serde_json::from_slice::<Vec<StakingLedgerAccount>>(&bytes) {
            Err(why) => panic!("Unable to parse JSON {}: {}", display, why),
            Ok(file) => file,
        };
        let ledger = StakingLedger {
            epoch_number: epoch,
            ledger_hash: ledger_hash,
            accounts: accounts.clone(),
        };

        match db.add_epoch(epoch, &ledger) {
            Ok(_) => println!("Successfully persisted staking ledger: {}", epoch),
            Err(why) => panic!("Failed to persist staking ledger {}: {}", epoch, why),
        }
        count += 1;
    }
    let delta = Instant::now().duration_since(start_time).as_millis();
    println!(
        "Processed {} staking ledgers in {} milliseconds",
        count, delta
    );
    println!(
        "{} ledgers/sec",
        (count as f64 / (delta as f64 / 1000.0_f64))
    );
}
