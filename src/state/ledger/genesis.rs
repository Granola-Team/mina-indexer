use super::{
    account::{Account, Amount, Nonce},
    Ledger,
};
use mina_serialization_types::{
    signatures::{CompressedCurvePoint, PublicKeyJson},
    v1::PublicKeyV1,
};
use mina_signer::CompressedPubKey;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, path::Path};
use tokio::io::AsyncReadExt;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisTimestamp {
    pub genesis_state_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisRoot {
    pub genesis: GenesisTimestamp,
    pub ledger: GenesisLedger,
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
pub struct GenesisLedger {
    pub name: String,
    pub accounts: Vec<GenesisAccount>,
}

pub fn string_to_public_key_json(s: String) -> Result<PublicKeyJson, Box<dyn Error>> {
    let pk = CompressedPubKey::from_address(&s)?;
    let pk = CompressedCurvePoint::from(&pk);
    Ok(pk.into())
}

impl From<GenesisRoot> for Ledger {
    fn from(value: GenesisRoot) -> Self {
        value.ledger.into()
    }
}

impl From<GenesisLedger> for Ledger {
    fn from(genesis_ledger: GenesisLedger) -> Ledger {
        let mut accounts = HashMap::new();
        for genesis_account in genesis_ledger.accounts {
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
            if let Ok(pk) = string_to_public_key_json(genesis_account.pk) {
                accounts.insert(
                    PublicKeyV1::from(pk.clone()).into(),
                    Account {
                        public_key: PublicKeyV1::from(pk.clone()).into(),
                        delegate: genesis_account.delegate.clone().map(|delegate| {
                            PublicKeyV1::from(string_to_public_key_json(delegate).unwrap()).into()
                        }),
                        balance,
                        nonce: Nonce::default(),
                    },
                );
            } else {
                panic!("Unparsable public key");
            }
        }
        Ledger { accounts }
    }
}

pub async fn parse_file(filename: &Path) -> anyhow::Result<GenesisRoot> {
    let mut genesis_ledger_file = tokio::fs::File::open(&filename).await?;
    let mut genesis_ledger_file_contents = Vec::new();

    genesis_ledger_file
        .read_to_end(&mut genesis_ledger_file_contents)
        .await?;
    drop(genesis_ledger_file);
    Ok(serde_json::from_slice(&genesis_ledger_file_contents)?)
}
