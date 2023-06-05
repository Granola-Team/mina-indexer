use std::{collections::HashMap, path::Path};

use mina_serialization_types::{signatures::PublicKeyJson, v1::PublicKeyV1};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use super::{
    account::{Account, Amount, Nonce},
    Ledger,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisTimestamp {
    genesis_state_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisData {
    genesis: GenesisTimestamp,
    ledger: GenesisLedger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pk: PublicKeyJson,
    balance: String,
    delegate: Option<PublicKeyJson>,
    timing: Option<GenesisAccountTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccountTiming {
    initial_minimum_balance: String,
    cliff_time: String,
    cliff_amount: String,
    vesting_period: String,
    vesting_increment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisLedger {
    name: String,
    accounts: Vec<GenesisAccount>,
}

impl From<GenesisLedger> for Ledger {
    fn from(genesis_ledger: GenesisLedger) -> Ledger {
        let mut accounts = HashMap::new();
        for genesis_account in genesis_ledger.accounts {
            let balance = match str::parse::<u64>(&genesis_account.balance) {
                Ok(amt) => Amount(amt * 1_000_000_000),
                Err(_) => Amount::default(),
            };

            accounts.insert(
                PublicKeyV1::from(genesis_account.pk.clone()).into(),
                Account {
                    public_key: PublicKeyV1::from(genesis_account.pk).into(),
                    delegate: genesis_account
                        .delegate
                        .clone()
                        .map(|delegate| PublicKeyV1::from(delegate).into()),
                    balance,
                    nonce: Nonce::default(),
                },
            );
        }
        // assert_eq!(genesis_ledger.num_accounts as usize, accounts.len());
        Ledger { accounts }
    }
}

#[cfg(test)]
pub mod tests {
    use super::GenesisLedger;

    const GENESIS_LEDGER_JSON: &'static str = include_str!("./genesis_ledger.json");

    #[test]
    pub fn genesis_ledger_deserializes() {
        let _genesis_ledger: GenesisLedger = serde_json::from_str(GENESIS_LEDGER_JSON).unwrap();
    }
}

pub async fn parse_file(filename: &Path) -> anyhow::Result<GenesisLedger> {
    let mut genesis_ledger_file = tokio::fs::File::open(&filename).await?;
    let mut genesis_ledger_file_contents = Vec::new();

    genesis_ledger_file
        .read_to_end(&mut genesis_ledger_file_contents)
        .await?;

    let genesis_ledger: GenesisLedger = serde_json::from_slice(&genesis_ledger_file_contents)?;

    Ok(genesis_ledger)
}
