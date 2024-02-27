use super::{
    account::{Account, Amount, Nonce},
    public_key::PublicKey,
    Ledger,
};

use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisTimestamp {
    pub genesis_state_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisRoot {
    pub genesis: GenesisTimestamp,
    pub ledger: GenesisAccounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub pk: String,
    pub balance: String,
    pub delegate: Option<String>,
    pub timing: Option<GenesisAccountTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccountTiming {
    pub initial_minimum_balance: String,
    pub cliff_time: String,
    pub cliff_amount: String,
    pub vesting_period: String,
    pub vesting_increment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccounts {
    pub name: String,
    pub accounts: Vec<GenesisAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisLedger {
    ledger: Ledger,
}

impl From<GenesisRoot> for GenesisLedger {
    fn from(value: GenesisRoot) -> Self {
        Self::new(value.ledger)
    }
}

impl From<GenesisLedger> for Ledger {
    fn from(value: GenesisLedger) -> Self {
        value.ledger
    }
}

impl GenesisLedger {
    /// This is the only way to construct a genesis ledger
    pub fn new(genesis: GenesisAccounts) -> GenesisLedger {
        let mut accounts = HashMap::new();
        for genesis_account in genesis.accounts {
            let balance = Amount(match str::parse::<Decimal>(&genesis_account.balance) {
                Ok(amt) => (amt * dec!(1_000_000_000))
                    .to_u64()
                    .expect("Parsed Genesis Balance has wrong format"),
                Err(_) => panic!("Unable to parse Genesis Balance"),
            });
            let public_key = PublicKey::from(genesis_account.pk);
            accounts.insert(
                public_key.clone(),
                Account {
                    public_key: public_key.clone(),
                    // If delegate is None, delegate to yourself
                    delegate: genesis_account
                        .delegate
                        .map(PublicKey)
                        .unwrap_or(public_key),
                    balance,
                    nonce: Nonce::default(),
                },
            );
        }
        GenesisLedger {
            ledger: Ledger { accounts },
        }
    }
}

pub fn parse_file<P: AsRef<Path>>(filename: P) -> anyhow::Result<GenesisRoot> {
    let data = std::fs::read(filename)?;
    Ok(serde_json::from_slice(&data)?)
}

mod tests {
    use crate::ledger::public_key::PublicKey;

    use super::{GenesisLedger, GenesisRoot};

    #[test]
    fn test_genesis_ledger_default_delegation_test() -> anyhow::Result<()> {
        let ledger_json = r#"{
            "genesis": {
                "genesis_state_timestamp": "2021-03-17T00:00:00Z"
            },
            "ledger": {
                "name": "mainnet",
                "accounts": [
                    {"pk": "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1","balance":"0"}
                ]
            }
        }"#;

        let root: GenesisRoot = serde_json::from_str(ledger_json)?;

        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            root.ledger.accounts.first().unwrap().pk
        );
        assert_eq!(None, root.ledger.accounts.first().unwrap().delegate);

        let ledger = GenesisLedger::new(root.ledger);
        let map = ledger.ledger.accounts;
        let value = map
            .get(&PublicKey::from(
                "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            ))
            .unwrap();

        // The delete should be the same as the public key
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            value.public_key.0
        );
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            value.delegate.0
        );

        Ok(())
    }
}
