use mina_indexer::{
    base::public_key::PublicKey,
    block::genesis::GenesisBlock,
    chain::Network,
    ledger::{
        genesis::{GenesisLedger, GenesisRoot},
        token::TokenAddress,
        Ledger,
    },
};
#[test]
fn test_mainnet_genesis_parser() -> anyhow::Result<()> {
    let genesis_root = GenesisRoot::parse_file("./data/genesis_ledgers/mainnet.json")?;
    let genesis_ledger: GenesisLedger = genesis_root.clone().into();
    let ledger: Ledger = genesis_ledger.into();
    let mina_accounts = &ledger
        .tokens
        .get(&TokenAddress::default())
        .unwrap()
        .accounts;

    // Ledger account balances are in nanomina
    let total_supply: u64 = mina_accounts
        .values()
        .cloned()
        .map(|account| account.display().balance.0)
        .sum();

    assert_eq!(
        Network::Mainnet.to_string(),
        genesis_root.ledger.name.unwrap(),
        "Network name"
    );
    assert_eq!(
        "2021-03-17T00:00:00Z",
        genesis_root.genesis.unwrap().genesis_state_timestamp,
        "Genesis timestamp"
    );

    // genesis block creator is in genesis ledger
    assert!(mina_accounts.contains_key(&PublicKey::from(
        "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
    )));
    assert_eq!(
        1676,
        mina_accounts.len(),
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
