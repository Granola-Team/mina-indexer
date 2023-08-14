use clap::Parser;

use glob::glob;

use mina_indexer::{
    staking_ledger::{
        staking_ledger_store::StakingLedgerStore, DelegationTotals, StakingLedger,
        StakingLedgerAccount,
    },
    store::IndexerStore,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
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

fn extract_epoch_and_hash(file_name: &OsStr) -> Option<(u32, &str)> {
    let mut chunks = file_name.to_str()?.split('-');
    let epoch = chunks.next().unwrap().parse::<u32>().unwrap();
    let ledger_hash = chunks.next().unwrap();

    return Some((epoch, ledger_hash));
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

    let start_time = Instant::now();
    let mut count = 0;

    for path in paths {
        let (epoch, ledger_hash) = extract_epoch_and_hash(path.file_stem().unwrap()).unwrap();
        println!("{}:{}", epoch, ledger_hash);
        let display = path.display();
        let mut file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, why),
            Ok(file) => file,
        };
        let mut bytes = Vec::new();
        let _ = file.read_to_end(&mut bytes);
        drop(file);

        let mut accounts = match serde_json::from_slice::<Vec<StakingLedgerAccount>>(&bytes) {
            Err(why) => panic!("Unable to parse JSON {}: {}", display, why),
            Ok(file) => file,
        };

        for account in accounts.iter_mut() {
            account.ledger_hash = Some(ledger_hash.to_string());
            account.epoch_number = Some(epoch as i32);
        }

        let mut accs = accounts.clone();
        println!("{} accounts in staking ledger {}", accs.len(), epoch);
        let now = Instant::now();
        let mut processed = 0;
        for account in &mut accounts {
            let mut count_delegates = 0;
            let mut total_delegations: i64 = 0;
            let mut j = 0;
            while j < accs.len() {
                // Counting delgations for account.pk
                if account.pk == accs[j].delegate {
                    count_delegates += 1;
                    let amount = accs[j]
                        .balance
                        .parse::<Decimal>()
                        .map(|a| a * dec!(1_000_000_000))
                        .unwrap_or_else(|_| Decimal::ZERO)
                        .to_i64()
                        .unwrap();

                    total_delegations += amount;
                    accs.swap_remove(j);
                } else {
                    j += 1;
                }
            }
            account.delegation_totals = Some(DelegationTotals {
                count_delegates,
                total_delegations,
            });
            processed += 1;
            if processed % 5000 == 0 {
                let delta: u128 = Instant::now().duration_since(now).as_millis();
                println!(
                    "Processed {} ledger accounts in {} milliseconds",
                    processed, delta
                );
            }
        }
        let ledger = StakingLedger {
            epoch_number: epoch,
            ledger_hash: ledger_hash.to_string(),
            accounts: accounts.clone(),
        };

        match db.add_epoch(epoch, &ledger) {
            Ok(_) => println!("Successfully persisted staking ledger: {}", epoch),
            Err(why) => panic!("Failed to persist staking ledger {}: {}", epoch, why),
        }
        let delta: u128 = Instant::now().duration_since(now).as_millis();
        println!(
            "Processed {} staking ledger in {} milliseconds",
            epoch, delta
        );
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
