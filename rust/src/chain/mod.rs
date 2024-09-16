pub mod store;

use crate::constants::*;
use bincode::{Decode, Encode};
use clap::builder::OsStr;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChainId(pub String);

impl ChainId {
    pub fn new(chain_id: &str) -> Self {
        Self(chain_id.to_string())
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Chain id used by mina node p2p network
pub fn chain_id(
    genesis_state_hash: &str,
    genesis_constants: &[u32],
    constraint_system_digests: &[&str],
) -> ChainId {
    use blake2::{digest::VariableOutput, Blake2bVar};
    use std::io::Write;

    let genesis_constants_hash: String = {
        let mut gcs = genesis_constants
            .iter()
            .map(u32::to_string)
            .collect::<Vec<String>>();
        gcs.push(
            from_timestamp_millis(MAINNET_GENESIS_TIMESTAMP as i64)
                .format("%Y-%m-%d %H:%M:%S%.6fZ")
                .to_string(),
        );

        let mut hasher = Blake2bVar::new(32).unwrap();
        hasher.write_all(gcs.concat().as_bytes()).unwrap();
        hasher.finalize_boxed().encode_hex()
    };
    let all_snark_keys = constraint_system_digests.concat();
    let digest_str = [genesis_state_hash, &all_snark_keys, &genesis_constants_hash].concat();

    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.write_all(digest_str.as_bytes()).unwrap();
    ChainId(hasher.finalize_boxed().to_vec().encode_hex())
}

#[derive(Default, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Mainnet,
    Devnet,
    Testworld,
    Berkeley,
}

impl Network {
    const MAINNET: &'static str = "mainnet";
    const DEVNET: &'static str = "devnet";
    const TESTWORLD: &'static str = "testworld";
    const BERKELEY: &'static str = "berkeley";

    fn format(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                Network::Mainnet => Network::MAINNET,
                Network::Devnet => Network::DEVNET,
                Network::Testworld => Network::TESTWORLD,
                Network::Berkeley => Network::BERKELEY,
            }
        )
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.format(f)
    }
}

impl Debug for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.format(f)
    }
}

impl From<Vec<u8>> for Network {
    fn from(value: Vec<u8>) -> Self {
        Network::from(String::from_utf8(value).unwrap().as_str())
    }
}

impl From<&str> for Network {
    fn from(value: &str) -> Self {
        match value {
            Network::MAINNET => Network::Mainnet,
            Network::DEVNET => Network::Devnet,
            Network::TESTWORLD => Network::Testworld,
            Network::BERKELEY => Network::Berkeley,
            _ => panic!("{value} is not a valid network"),
        }
    }
}

impl From<Network> for OsStr {
    fn from(value: Network) -> Self {
        value.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_test() {
        assert_eq!(
            "5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1",
            chain_id(
                MAINNET_GENESIS_HASH,
                MAINNET_PROTOCOL_CONSTANTS,
                MAINNET_CONSTRAINT_SYSTEM_DIGESTS
            )
            .0
        )
    }
}
