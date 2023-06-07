use std::path::Path;

use mina_indexer::state::ledger::genesis::GenesisRoot;
use tokio::{fs::File, io::AsyncReadExt};

const GENESIS_LEDGERS_PATH: &'static str = "./tests/data/genesis_ledgers";

pub async fn read_genesis_ledger_to_string(ledger: &str) -> Result<String, anyhow::Error> {
    let mut ledger_file = File::open(Path::new(GENESIS_LEDGERS_PATH).join(ledger)).await?;

    let mut buffer = vec![];
    ledger_file.read_to_end(&mut buffer).await?;

    Ok(String::from(std::str::from_utf8(&buffer)?))
}

#[tokio::test]
pub async fn mainnet_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("mainnet.json")
        .await
        .expect("mainnet genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json).expect("mainnet genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn berkeley_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("berkeley.json")
        .await
        .expect("berkeley genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json).expect("berkeley genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn devnet_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("devnet.json")
        .await
        .expect("devnet genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json).expect("devnet genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn devnet2_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("devnet2.json")
        .await
        .expect("devnet2 genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json).expect("devnet2 genesis ledger parses into GenesisRoot");
}