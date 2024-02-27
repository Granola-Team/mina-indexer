use mina_indexer::ledger::{
    genesis::{self, GenesisLedger},
    Ledger,
};
#[test]
fn test_mainnet_genesis_parser() -> anyhow::Result<()> {
    let genesis_root = genesis::parse_file("./tests/data/genesis_ledgers/mainnet.json")?;
    let genesis_ledger: GenesisLedger = genesis_root.clone().into();
    let ledger: Ledger = genesis_ledger.into();

    // Ledger account balances are in nanomina
    let initial_supply = ledger
        .accounts
        .values()
        .fold(0u64, |acc, account| acc + account.balance.0);

    assert_eq!("mainnet", genesis_root.ledger.name, "Network name");
    assert_eq!(
        "2021-03-17T00:00:00Z", genesis_root.genesis.genesis_state_timestamp,
        "Genesis timestamp"
    );
    assert_eq!(
        1675,
        genesis_root.ledger.accounts.len(),
        "Total number of genesis accounts"
    );
    assert_eq!(
        805385692840038233, initial_supply,
        "Mina inital distribution"
    );
    Ok(())
}
