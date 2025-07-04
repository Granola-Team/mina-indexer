//! Indexer internal block representation used in the witness tree

pub mod epoch_data;
pub mod genesis;
pub mod genesis_state_hash;
pub mod parser;
pub mod post_hardfork;
pub mod precomputed;
pub mod previous_state_hash;
pub mod store;
pub mod vrf_output;

use self::{precomputed::PrecomputedBlock, vrf_output::VrfOutput};
use crate::{
    base::{blockchain_length::BlockchainLength, state_hash::StateHash},
    canonicity::Canonicity,
    chain::Network,
    constants::*,
    utility::functions::{extract_height_and_hash, is_valid_file_name},
};
use precomputed::PcbVersion;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

// re-export types
pub type AccountCreated = post_hardfork::account_created::AccountCreated;

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Block {
    pub parent_hash: StateHash,
    pub state_hash: StateHash,
    pub height: u32,
    pub blockchain_length: u32,
    pub genesis_state_hash: StateHash,
    pub global_slot_since_genesis: u32,
    pub hash_last_vrf_output: VrfOutput,
}

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BlockWithoutHeight {
    pub canonicity: Option<Canonicity>,
    pub parent_hash: StateHash,
    pub state_hash: StateHash,
    pub blockchain_length: u32,
    pub global_slot_since_genesis: u32,
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock, height: u32) -> Self {
        Self {
            height,
            state_hash: precomputed_block.state_hash(),
            parent_hash: precomputed_block.previous_state_hash(),
            blockchain_length: precomputed_block.blockchain_length(),
            hash_last_vrf_output: precomputed_block.hash_last_vrf_output(),
            global_slot_since_genesis: precomputed_block.global_slot_since_genesis(),
            genesis_state_hash: precomputed_block.genesis_state_hash(),
        }
    }

    pub fn summary(&self) -> String {
        format!("(length {}): {}", self.blockchain_length, self.state_hash)
    }
}

impl From<Block> for BlockWithoutHeight {
    fn from(value: Block) -> Self {
        Self {
            canonicity: None,
            parent_hash: value.parent_hash,
            state_hash: value.state_hash,
            global_slot_since_genesis: value.global_slot_since_genesis,
            blockchain_length: value.blockchain_length,
        }
    }
}

impl BlockWithoutHeight {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        Self {
            canonicity: None,
            state_hash: precomputed_block.state_hash(),
            parent_hash: precomputed_block.previous_state_hash(),
            blockchain_length: precomputed_block.blockchain_length(),
            global_slot_since_genesis: precomputed_block.global_slot_since_genesis(),
        }
    }

    pub fn with_canonicity(block: &PrecomputedBlock, canonicity: Canonicity) -> Self {
        let block: Block = block.into();
        Self {
            canonicity: Some(canonicity),
            state_hash: block.state_hash,
            parent_hash: block.parent_hash,
            blockchain_length: block.blockchain_length,
            global_slot_since_genesis: block.global_slot_since_genesis,
        }
    }

    pub fn summary(&self) -> String {
        format!("(length {}): {}", self.blockchain_length, self.state_hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockComparison {
    pub state_hash: StateHash,
    pub blockchain_length: u32,
    pub hash_last_vrf_output: VrfOutput,
    pub version: PcbVersion,
}

/////////////////
// conversions //
/////////////////

impl From<PrecomputedBlock> for Block {
    fn from(value: PrecomputedBlock) -> Self {
        Self {
            height: value.blockchain_length().saturating_sub(1),
            parent_hash: value.previous_state_hash(),
            blockchain_length: value.blockchain_length(),
            state_hash: value.state_hash(),
            hash_last_vrf_output: value.hash_last_vrf_output(),
            genesis_state_hash: value.genesis_state_hash(),
            global_slot_since_genesis: value.global_slot_since_genesis(),
        }
    }
}

impl From<&PrecomputedBlock> for Block {
    fn from(value: &PrecomputedBlock) -> Self {
        Self {
            height: value.blockchain_length().saturating_sub(1),
            parent_hash: value.previous_state_hash(),
            blockchain_length: value.blockchain_length(),
            state_hash: value.state_hash(),
            hash_last_vrf_output: value.hash_last_vrf_output(),
            genesis_state_hash: value.genesis_state_hash(),
            global_slot_since_genesis: value.global_slot_since_genesis(),
        }
    }
}

impl From<&PrecomputedBlock> for BlockWithoutHeight {
    fn from(value: &PrecomputedBlock) -> Self {
        Self {
            canonicity: None,
            parent_hash: value.previous_state_hash(),
            state_hash: value.state_hash(),
            blockchain_length: value.blockchain_length(),
            global_slot_since_genesis: value.global_slot_since_genesis(),
        }
    }
}

impl From<&PrecomputedBlock> for BlockComparison {
    fn from(value: &PrecomputedBlock) -> Self {
        Self {
            state_hash: value.state_hash(),
            blockchain_length: value.blockchain_length(),
            hash_last_vrf_output: value.hash_last_vrf_output(),
            version: value.version(),
        }
    }
}

/////////////////
// comparisons //
/////////////////

impl std::cmp::PartialOrd for BlockComparison {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for BlockComparison {
    fn cmp(&self, other: &Self) -> Ordering {
        use PcbVersion::*;

        // hardfork blocks are better than pre-hardfork blocks
        match (&self.version, &other.version) {
            (V1, V2) => return Ordering::Greater,
            (V2, V1) => return Ordering::Less,
            _ => (),
        }

        let length_cmp = self.blockchain_length.cmp(&other.blockchain_length);
        let vrf_cmp = self.hash_last_vrf_output.cmp(&other.hash_last_vrf_output);
        let hash_cmp = self.state_hash.cmp(&other.state_hash);

        match (length_cmp, vrf_cmp, hash_cmp) {
            (Ordering::Greater, _, _)
            | (Ordering::Equal, Ordering::Greater, _)
            | (Ordering::Equal, Ordering::Equal, Ordering::Greater) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl std::cmp::PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Block {
    /// Follows `selectLongerChain`
    /// A < B means A is better than B
    /// https://github.com/MinaProtocol/mina/tree/develop/docs/specs/consensus#62-select-chain
    fn cmp(&self, other: &Self) -> Ordering {
        // hardfork blocks are better than pre-hardfork blocks
        match (
            &self.genesis_state_hash.0 as &str,
            &other.genesis_state_hash.0 as &str,
        ) {
            (MAINNET_GENESIS_HASH, HARDFORK_GENESIS_HASH) => return Ordering::Greater,
            (HARDFORK_GENESIS_HASH, MAINNET_GENESIS_HASH) => return Ordering::Less,
            _ => (),
        }

        let length_cmp = self.blockchain_length.cmp(&other.blockchain_length);
        let vrf_cmp = self.hash_last_vrf_output.cmp(&other.hash_last_vrf_output);
        let hash_cmp = self.state_hash.cmp(&other.state_hash);

        match (length_cmp, vrf_cmp, hash_cmp) {
            (Ordering::Greater, _, _)
            | (Ordering::Equal, Ordering::Greater, _)
            | (Ordering::Equal, Ordering::Equal, Ordering::Greater) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl std::default::Default for StateHash {
    fn default() -> Self {
        Self("3NLDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into())
    }
}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Block {{ height: {}, len: {}, slot: {}, state: {}, parent: {} }}",
            self.height,
            self.blockchain_length,
            self.global_slot_since_genesis,
            self.state_hash,
            self.parent_hash
        )
    }
}

impl std::fmt::Debug for BlockWithoutHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for BlockWithoutHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl std::fmt::Debug for StateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "StateHash {{ {:?} }}", self.0)
    }
}

impl std::fmt::Display for StateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/////////////
// helpers //
/////////////

pub fn is_valid_block_file<P>(path: P) -> bool
where
    P: AsRef<Path>,
    P: Into<PathBuf>,
{
    is_valid_file_name(path, &StateHash::is_valid)
}

pub fn sort_by_height_and_lexicographical_order(paths: &mut [&std::path::PathBuf]) {
    paths.sort_by(|a, b| {
        let (height_a, hash_a) = extract_height_and_hash(a);
        let (height_b, hash_b) = extract_height_and_hash(b);

        match height_a.cmp(&height_b) {
            Ordering::Equal => hash_a.cmp(hash_b),
            other => other,
        }
    });
}

pub fn extract_block_height(path: &Path) -> u32 {
    extract_height_and_hash(path).0
}

pub fn extract_state_hash(path: &Path) -> &str {
    extract_height_and_hash(path).1
}

pub fn extract_network(path: &Path) -> Network {
    let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
    let dash_pos = name.find('-').unwrap();
    Network::from(&name[..dash_pos])
}

/// Extracts all three values from file name
///
/// Valid block file names have the form: {network}-{block height}-{state hash}
pub fn extract_network_height_hash(path: &Path) -> (Network, BlockchainLength, StateHash) {
    let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
    let network_end = name.find('-').expect("valid block file has network");
    let height_end = name[network_end + 1..]
        .find('-')
        .expect("valid block file has height");
    let block_height = name[network_end + 1..][..height_end]
        .parse::<u32>()
        .expect("block height is u32");
    let state_hash = &name[network_end + 1..][height_end + 1..];
    (
        Network::from(&name[..network_end]),
        block_height.into(),
        state_hash.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use precomputed::PcbVersion;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_block_hash_is_valid_public_key() {
        assert!(StateHash::is_valid(&StateHash::default().0))
    }

    #[test]
    fn extract_state_hash_test() {
        let path0 =
            Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let path1 = Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(
            "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH",
            extract_state_hash(path0)
        );
        assert_eq!(
            "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R",
            extract_state_hash(path1)
        );
    }

    #[test]
    fn extract_block_height_test() {
        let path0 =
            Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let path1 = Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(2, extract_block_height(path0));
        assert_eq!(3, extract_block_height(path1));
    }

    #[test]
    fn extract_network_height_hash_test() {
        let path0 =
            Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let path1 = Path::new(
            "/tmp/blocks/devnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(
            (
                Network::Mainnet,
                2.into(),
                StateHash::from("3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH")
            ),
            extract_network_height_hash(path0)
        );
        assert_eq!(
            (
                Network::Devnet,
                3.into(),
                StateHash::from("3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R")
            ),
            extract_network_height_hash(path1)
        );
    }

    #[test]
    fn compare_blocks() -> anyhow::Result<()> {
        let path0: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json".into();
        let path1: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let path2: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK.json".into();
        let block0: Block = PrecomputedBlock::parse_file(&path0, PcbVersion::V1)?.into();
        let block1: Block = PrecomputedBlock::parse_file(&path1, PcbVersion::V1)?.into();
        let block2: Block = PrecomputedBlock::parse_file(&path2, PcbVersion::V1)?.into();

        assert!(block0 < block1);
        assert!(block0 < block2);
        assert!(block1 < block2);

        let path0: PathBuf = "./tests/data/canonical_chain_discovery/contiguous/mainnet-10-3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5.json".into();
        let path1: PathBuf = "./tests/data/canonical_chain_discovery/gaps/mainnet-10-3NKHYHrqKpDcon6ToV5CLDiheanjshk5gcsNqefnK78phCFTR2aL.json".into();
        let block0: Block = PrecomputedBlock::parse_file(&path0, PcbVersion::V1)?.into();
        let block1: Block = PrecomputedBlock::parse_file(&path1, PcbVersion::V1)?.into();

        assert!(block0 < block1);
        Ok(())
    }

    #[test]
    fn test_sort_by_height_and_lexicographical_order() {
        let filename1 = PathBuf::from("mainnet-1-abc123.json");
        let filename2 = PathBuf::from("mainnet-2-def456.json");
        let filename3 = PathBuf::from("mainnet-2-ghi789.json");
        let filename4 = PathBuf::from("mainnet-3-jkl012.json");

        let mut paths = [&filename3, &filename1, &filename4, &filename2];

        sort_by_height_and_lexicographical_order(&mut paths);

        assert_eq!(paths[0].file_name().unwrap(), "mainnet-1-abc123.json");
        assert_eq!(paths[1].file_name().unwrap(), "mainnet-2-def456.json");
        assert_eq!(paths[2].file_name().unwrap(), "mainnet-2-ghi789.json");
        assert_eq!(paths[3].file_name().unwrap(), "mainnet-3-jkl012.json");
    }
}
