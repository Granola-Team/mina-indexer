use mina_indexer::{
    block::genesis::GenesisBlock,
    chain::Network,
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{
        genesis::{self, GenesisLedger},
        public_key::PublicKey,
        Ledger,
    },
};
#[test]
fn test_mainnet_genesis_parser() -> anyhow::Result<()> {
    let genesis_root = genesis::parse_file("./data/genesis_ledgers/mainnet.json")?;
    let genesis_ledger: GenesisLedger = genesis_root.clone().into();
    let ledger: Ledger = genesis_ledger.into();

    // Ledger account balances are in nanomina
    let total_supply = ledger.accounts.values().fold(0, |acc, account| {
        acc + account.balance.0 - MAINNET_ACCOUNT_CREATION_FEE.0
    });

    assert_eq!(
        Network::Mainnet.to_string(),
        genesis_root.ledger.name,
        "Network name"
    );
    assert_eq!(
        "2021-03-17T00:00:00Z", genesis_root.genesis.genesis_state_timestamp,
        "Genesis timestamp"
    );

    // genesis block creator is in genesis ledger
    assert!(ledger.accounts.contains_key(&PublicKey::from(
        "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
    )));
    assert_eq!(
        1676,
        ledger.accounts.len(),
        "Total number of genesis accounts"
    );

    let genesis_block = GenesisBlock::new_v1().unwrap().0;
    assert_eq!(
        genesis_block.total_currency(),
        total_supply,
        "Mina inital distribution"
    );
    Ok(())
}
