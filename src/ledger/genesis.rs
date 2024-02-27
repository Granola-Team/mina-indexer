use super::{
    account::{Account, Amount, Nonce},
    public_key::PublicKey,
    Ledger,
};

use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::debug;

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
            // Temporary hack to ignore bad PKs in mainnet genesis ledger
            if genesis_account.pk == "B62qpyhbvLobnd4Mb52vP7LPFAasb2S6Qphq8h5VV8Sq1m7VNK1VZcW"
                || genesis_account.pk == "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1"
            {
                debug!(
                    "Known broken public keys... Ignoring {}",
                    genesis_account.pk
                );
                continue;
            }
            let public_key = PublicKey::from(genesis_account.pk);
            accounts.insert(
                public_key.clone(),
                Account {
                    public_key,
                    delegate: genesis_account.delegate.map(PublicKey),
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
