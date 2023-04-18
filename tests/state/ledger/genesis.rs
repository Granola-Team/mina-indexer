use std::path::Path;

use mina_indexer::state::ledger::genesis::GenesisLedger;
use tokio::{fs::File, io::AsyncReadExt};

const GENESIS_LEDGERS_PATH: &'static str = "./tests/data/genesis_ledgers";

#[tokio::test]
pub async fn default_genesis_ledger_parses() {
    let mut ledger_file = File::open(Path::new(GENESIS_LEDGERS_PATH).join("default.json")).await.unwrap();

    let mut buffer = vec![];
    ledger_file.read_to_end(&mut buffer).await.unwrap();

    let ledger_json = std::str::from_utf8(&buffer).unwrap();

    let _genesis_ledger: GenesisLedger = serde_json::from_str(ledger_json).unwrap();
}