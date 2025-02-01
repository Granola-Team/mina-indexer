//! Chain id

use crate::constants::*;
use anyhow::bail;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ChainId(pub String);

impl ChainId {
    pub const LEN: u32 = 64;

    /// Chain id used by mina node p2p network
    ///
    /// See https://github.com/MinaProtocol/mina/blob/compatible/src/app/cli/src/cli_entrypoint/mina_cli_entrypoint.ml#L20
    pub fn new(
        genesis_state_hash: &str,
        genesis_constants: &[u32],
        constraint_system_digests: &[&str],
        genesis_timestamp: i64,
        protocol_txn_version_disgest: Option<&str>,
        protocol_network_version_disgest: Option<&str>,
    ) -> Self {
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

        Self(hasher.finalize_boxed().to_vec().encode_hex())
    }

    /// Pre-hardfork chain id
    pub fn v1() -> Self {
        Self(MAINNET_CHAIN_ID.to_string())
    }

    /// Post-hardfork chain id
    pub fn v2() -> Self {
        Self(HARDFORK_CHAIN_ID.to_string())
    }

    /// Checks length & hex
    pub fn is_valid<T>(chain_id: T) -> bool
    where
        T: ToString,
    {
        let chain_id = chain_id.to_string();
        chain_id.len() as u32 == Self::LEN && hex::decode(chain_id).is_ok()
    }
}

/////////////////
// conversions //
/////////////////

impl FromStr for ChainId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid(s) {
            return Ok(Self(s.to_string()));
        }

        bail!("Invalid chain id: {}", s)
    }
}

impl From<Vec<u8>> for ChainId {
    fn from(value: Vec<u8>) -> Self {
        let chain_id = String::from_utf8(value).expect("chain id");
        Self::from_str(&chain_id).expect("valid chain id")
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid() {
        assert!(ChainId::is_valid(MAINNET_CHAIN_ID), "mainnet");
        assert!(ChainId::is_valid(HARDFORK_CHAIN_ID), "hardfork");
    }

    #[test]
    fn chain_id_v1() {
        assert_eq!(
            MAINNET_CHAIN_ID,
            ChainId::new(
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
            ChainId::new(
                HARDFORK_GENESIS_HASH,
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
