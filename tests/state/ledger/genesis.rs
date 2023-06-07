use std::path::Path;

use mina_indexer::state::ledger::{genesis::GenesisRoot, Ledger};
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
    serde_json::from_str::<GenesisRoot>(&ledger_json)
        .expect("mainnet genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn berkeley_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("berkeley.json")
        .await
        .expect("berkeley genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json)
        .expect("berkeley genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn devnet_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("devnet.json")
        .await
        .expect("devnet genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json)
        .expect("devnet genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn devnet2_genesis_ledger_parses() {
    let ledger_json = read_genesis_ledger_to_string("devnet2.json")
        .await
        .expect("devnet2 genesis ledger file exists");
    serde_json::from_str::<GenesisRoot>(&ledger_json)
        .expect("devnet2 genesis ledger parses into GenesisRoot");
}

#[tokio::test]
pub async fn test_ignore_known_invalid_pks_on_mainnet() {
    let ledger_json = r#"{
        "genesis": {
            "genesis_state_timestamp": "2021-03-17T00:00:00Z"
        },
        "ledger": {
            "name": "mainnet",
            "accounts": [
                {"pk": "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1","balance":"0"},
                {"pk": "B62qpyhbvLobnd4Mb52vP7LPFAasb2S6Qphq8h5VV8Sq1m7VNK1VZcW","balance":"0"},
                {"pk": "B62qmVHmj3mNhouDf1hyQFCSt3ATuttrxozMunxYMLctMvnk5y7nas1","balance":"0"}
            ]
        }
    }"#;
    let root: GenesisRoot =
        serde_json::from_str(ledger_json).expect("Genesis ledger parses into GenesisRoot");
    assert_eq!(3, root.ledger.accounts.len(), "Should contain 3 accounts");
    let ledger: Ledger = root.ledger.into();
    assert_eq!(1, ledger.accounts.len(), "Should only be 1 account")
}
