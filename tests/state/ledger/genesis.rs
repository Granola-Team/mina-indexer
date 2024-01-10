use mina_indexer::ledger::{
    genesis::{self, GenesisRoot},
    Ledger,
};
#[test]
fn test_mainnet_genesis_parser() {
    let genesis_ledger = genesis::parse_file("./tests/data/genesis_ledgers/mainnet.json").unwrap();
    let ledger: Ledger = genesis_ledger.clone().into();
    // Ledger account balances are in nanomina
    let initial_supply = ledger
        .accounts
        .values()
        .fold(0u64, |acc, account| acc + account.balance.0);
    assert_eq!(
        "mainnet", genesis_ledger.ledger.name,
        "The network name should match"
    );
    assert_eq!(
        "2021-03-17T00:00:00Z", genesis_ledger.genesis.genesis_state_timestamp,
        "Genesis timestamp should match"
    );
    // 1675 total genesis ledger accounts
    assert_eq!(
        1675,
        genesis_ledger.ledger.accounts.len(),
        "Total number of genesis accounts should match"
    );
    // 805253692.840038233 was manually calculated ignoring the 2 bad accounts balances
    assert_eq!(
        805253692840038233, initial_supply,
        "Mina inital distribution should match"
    );
}

#[test]
fn test_ignore_known_invalid_pks_on_mainnet() {
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
