use std::{ffi::OsStr, fs::File, io::Read, path::PathBuf, time::Instant};

use clap::Parser;

use glob::glob;
use mina_indexer::block::precomputed::BlockLog;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    blocks_dir: PathBuf,
}

fn get_blockchain_length(file_name: &OsStr) -> Option<u32> {
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
    let blocks_dir = args.blocks_dir;
    let pattern = format!("{}/*.json", blocks_dir.display());

    let mut paths: Vec<PathBuf> = glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(|x| x.ok())
        .collect();
    paths.sort_by(|x, y| {
        get_blockchain_length(x.file_name().unwrap())
            .cmp(&get_blockchain_length(y.file_name().unwrap()))
    });

    let mut count = 0;
    let start_time = Instant::now();

    for path in paths {
        let mut buff = Vec::new();
        File::open(path).unwrap().read_to_end(&mut buff).unwrap();
        let str = String::from_utf8_lossy(&buff);
        serde_json::from_str::<BlockLog>(&str).unwrap();
        count += 1;
    }

    let delta = Instant::now().duration_since(start_time).as_millis();
    println!("Processed {} blocks in {} milliseconds", count, delta);
    println!(
        "{} blocks/sec",
        (count as f64 / (delta as f64 / 1000.0 as f64))
    );
}
