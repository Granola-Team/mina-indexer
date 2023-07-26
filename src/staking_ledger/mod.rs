pub(crate) mod staking_ledger_store;

use anyhow::Context;
use rust_decimal::{prelude::ToPrimitive, Decimal};

use rust_decimal_macros::dec;

use serde::{
    de::Visitor,
    de::{self, Deserializer},
    Deserialize,
};

use serde::Serialize;

use std::fmt;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StakingLedger {
    pub epoch_number: i32,
    pub ledger_hash: String,
    pub accounts: Vec<StakingLedgerAccount>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NanoMina(u64);

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce(u32);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StakingLedgerAccount {
    pub pk: String,
    pub balance: NanoMina,
    pub delegate: String,
    pub epoch_number: i32,
    pub ledger_hash: String,
    pub nonce: Option<Nonce>, // u32
    pub receipt_chain_hash: String,
    pub token: String, // u32
    pub voting_for: String,
}

impl<'de> Deserialize<'de> for NanoMina {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = NanoMina;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string formatted number")
            }

            fn visit_str<E>(self, balance: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Decimal::from_str(balance)
                    .context("balance should be Decimal parsable")
                    .map(|bal| bal * dec!(1_000_000_000))
                    .map(|bal| bal.to_u64().unwrap_or(0))
                    .context("Should be convertible to nanomina")
                    .map(NanoMina)
                    .map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}

impl<'de> Deserialize<'de> for Nonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = Nonce;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string formated u32")
            }

            fn visit_str<E>(self, nonce: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                nonce.parse().map(Nonce).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}
