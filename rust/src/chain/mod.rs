pub mod store;

use crate::{
    block::{precomputed::PcbVersion, BlockHash},
    constants::*,
};
use bincode::{Decode, Encode};
use clap::builder::OsStr;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter, Result},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChainId(pub String);

#[derive(Debug)]
pub struct ChainData(pub HashMap<BlockHash, (PcbVersion, ChainId)>);

impl ChainId {
    pub const LEN: u32 = 64;

    pub fn new(chain_id: &str) -> Self {
        Self(chain_id.to_string())
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ChainData {
    fn default() -> Self {
        let v1_genesis_state_hash: BlockHash = MAINNET_GENESIS_HASH.into();
        let v1_chain_id = chain_id(
            &v1_genesis_state_hash.0,
            MAINNET_PROTOCOL_CONSTANTS,
            MAINNET_CONSTRAINT_SYSTEM_DIGESTS,
            MAINNET_GENESIS_TIMESTAMP as i64,
            None,
            None,
        );
        let v2_genesis_state_hash: BlockHash = HARDFORK_GENSIS_HASH.into();
        let v2_chain_id = chain_id(
            &v2_genesis_state_hash.0,
            MAINNET_PROTOCOL_CONSTANTS,
            HARDFORK_CONSTRAINT_SYSTEM_DIGESTS,
            HARDFORK_GENESIS_TIMESTAMP as i64,
            Some(HARDFORK_PROTOCOL_TXN_VERSION_DIGEST),
            Some(HARDFORK_PROTOCOL_NETWORK_VERSION_DIGEST),
        );
        Self(HashMap::from([
            (v1_genesis_state_hash, (PcbVersion::V1, v1_chain_id)),
            (v2_genesis_state_hash, (PcbVersion::V2, v2_chain_id)),
        ]))
    }
}

/// Chain id used by mina node p2p network
///
/// See https://github.com/MinaProtocol/mina/blob/compatible/src/app/cli/src/cli_entrypoint/mina_cli_entrypoint.ml#L20
pub fn chain_id(
    genesis_state_hash: &str,
    genesis_constants: &[u32],
    constraint_system_digests: &[&str],
    genesis_timestamp: i64,
    protocol_txn_version_disgest: Option<&str>,
    protocol_network_version_disgest: Option<&str>,
) -> ChainId {
    use blake2::{digest::VariableOutput, Blake2bVar};
    use std::io::Write;

    let genesis_constants_hash: String = {
        let mut gcs = genesis_constants
            .iter()
            .map(u32::to_string)
            .collect::<Vec<String>>();
        gcs.push(
            from_timestamp_millis(genesis_timestamp)
                .format("%Y-%m-%d %H:%M:%S%.6fZ")
                .to_string(),
        );

        let mut hasher = Blake2bVar::new(32).unwrap();
        hasher.write_all(gcs.concat().as_bytes()).unwrap();
        hasher.finalize_boxed().encode_hex()
    };
    let all_snark_keys = constraint_system_digests.concat();
    let mut digest = vec![genesis_state_hash, &all_snark_keys, &genesis_constants_hash];

    // post-hardfork
    if let Some(protocol_txn_version_disgest) = protocol_txn_version_disgest {
        digest.push(protocol_txn_version_disgest);
    }
    if let Some(protocol_network_version_disgest) = protocol_network_version_disgest {
        digest.push(protocol_network_version_disgest);
    }

    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.write_all(digest.concat().as_bytes()).unwrap();
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
    fn chain_id_v1() {
        assert_eq!(
            MAINNET_CHAIN_ID,
            chain_id(
                MAINNET_GENESIS_HASH,
                MAINNET_PROTOCOL_CONSTANTS,
                MAINNET_CONSTRAINT_SYSTEM_DIGESTS,
                MAINNET_GENESIS_TIMESTAMP as i64,
                None,
                None,
            )
            .0
        )
    }

    #[test]
    fn chain_id_v2() {
        assert_eq!(
            HARDFORK_CHAIN_ID,
            chain_id(
                HARDFORK_GENSIS_HASH,
                MAINNET_PROTOCOL_CONSTANTS,
                HARDFORK_CONSTRAINT_SYSTEM_DIGESTS,
                HARDFORK_GENESIS_TIMESTAMP as i64,
                Some(HARDFORK_PROTOCOL_TXN_VERSION_DIGEST),
                Some(HARDFORK_PROTOCOL_NETWORK_VERSION_DIGEST),
            )
            .0
        )
    }
}
