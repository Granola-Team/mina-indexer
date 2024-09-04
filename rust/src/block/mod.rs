//! Indexer internal block representation used in the witness tree

pub mod blockchain_length;
pub mod genesis;
pub mod parser;
pub mod precomputed;
pub mod previous_state_hash;
pub mod store;
pub mod vrf_output;

use self::vrf_output::VrfOutput;
use crate::{
    block::precomputed::PrecomputedBlock,
    canonicity::Canonicity,
    chain::Network,
    protocol::serialization_types::{
        common::{Base58EncodableVersionedType, HashV1},
        version_bytes,
    },
    utility::functions::is_valid_file_name,
};
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::Path};

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub height: u32,
    pub blockchain_length: u32,
    pub genesis_state_hash: BlockHash,
    pub global_slot_since_genesis: u32,
    pub hash_last_vrf_output: VrfOutput,
}

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BlockWithoutHeight {
    pub canonicity: Option<Canonicity>,
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    pub global_slot_since_genesis: u32,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct BlockHash(pub String);

impl BlockHash {
    pub const LEN: usize = 52;
    pub const PREFIX: &'static str = "3N";

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let res = String::from_utf8(bytes.to_vec())?;
        if is_valid_state_hash(&res) {
            return Ok(Self(res));
        }
        bail!("Invalid state hash from bytes")
    }

    pub fn from_hashv1(hashv1: HashV1) -> anyhow::Result<Self> {
        let versioned: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            hashv1.into();
        versioned
            .to_base58_string()
            .map(Self)
            .map_err(|e| anyhow!("Error converting from hashv1: {e}"))
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock, height: u32) -> Self {
        let parent_hash = precomputed_block.previous_state_hash();
        let state_hash = precomputed_block.state_hash();
        Self {
            height,
            state_hash,
            parent_hash,
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
            parent_hash: value.parent_hash.clone(),
            state_hash: value.state_hash.clone(),
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
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    pub hash_last_vrf_output: VrfOutput,
}

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
        }
    }
}

impl std::str::FromStr for BlockHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_valid_state_hash(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid state hash: {}", s)
        }
    }
}

impl From<String> for BlockHash {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for BlockHash {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl std::cmp::PartialOrd for BlockComparison {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for BlockComparison {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Block {
    /// Follows `selectLongerChain`
    /// A < B means A is better than B
    /// https://github.com/MinaProtocol/mina/tree/develop/docs/specs/consensus#62-select-chain
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

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

impl std::default::Default for BlockHash {
    fn default() -> Self {
        Self("3NLDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into())
    }
}

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
            &self.state_hash.0[0..12],
            &self.parent_hash.0[0..12]
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

impl std::fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BlockHash {{ {:?} }}", self.0)
    }
}

impl std::fmt::Display for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Extracts a state hash from an OS file name
pub fn get_state_hash(file_name: &OsStr) -> Option<String> {
    let last_part = file_name.to_str()?.split('-').last()?.to_string();
    let state_hash = last_part.split('.').next()?;
    if state_hash.starts_with(BlockHash::PREFIX) {
        return Some(state_hash.to_string());
    }
    None
}

/// Extracts a blockchain length from an OS file name
pub fn get_blockchain_length(file_name: &OsStr) -> Option<u32> {
    file_name
        .to_str()?
        .split('-')
        .fold(None, |acc, x| match x.parse::<u32>() {
            Err(_) => acc,
            Ok(x) => Some(x),
        })
}

pub fn is_valid_state_hash(input: &str) -> bool {
    input.starts_with(BlockHash::PREFIX) && input.len() == BlockHash::LEN
}

pub fn is_valid_block_file(path: &Path) -> bool {
    is_valid_file_name(path, &is_valid_state_hash)
}

pub fn length_from_path(path: &Path) -> Option<u32> {
    if is_valid_block_file(path) {
        get_blockchain_length(path.file_name()?)
    } else {
        None
    }
}

pub fn sort_by_height_and_lexicographical_order(paths: &mut [&std::path::PathBuf]) {
    paths.sort_by(|a, b| {
        let (height_a, hash_a) = extract_height_and_hash(a);
        let (height_b, hash_b) = extract_height_and_hash(b);

        match height_a.cmp(&height_b) {
            std::cmp::Ordering::Equal => hash_a.cmp(hash_b),
            other => other,
        }
    });
}

pub fn extract_height_and_hash(path: &std::path::Path) -> (u32, &str) {
    let filename = path
        .file_name()
        .and_then(|x| x.to_str())
        .expect("Failed to extract filename from path");

    let mut parts = filename.split('-');

    match (parts.next(), parts.next(), parts.next()) {
        (Some(_), Some(height_str), Some(hash_part)) => {
            let block_height = height_str
                .parse::<u32>()
                .expect("Failed to parse block height");
            let hash = hash_part
                .split('.')
                .next()
                .expect("Failed to parse the hash");
            (block_height, hash)
        }
        _ => panic!("Filename format is invalid {}", filename),
    }
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
    let network = &name[..dash_pos];
    Network::from(network)
}

#[cfg(test)]
mod block_tests {
    use super::*;
    use crate::block::precomputed::PcbVersion;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_block_hash_is_valid_public_key() {
        assert!(is_valid_state_hash(&BlockHash::default().0))
    }

    #[test]
    fn extract_state_hash_test() {
        let filename2 =
            &Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let filename3 = &Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(
            "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH",
            extract_state_hash(filename2)
        );
        assert_eq!(
            "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R",
            extract_state_hash(filename3)
        );
    }

    #[test]
    fn extract_block_height_test() {
        let filename2 =
            &Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let filename3 = &Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(2, extract_block_height(filename2));
        assert_eq!(3, extract_block_height(filename3));
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
    fn block_hash_roundtrip() -> anyhow::Result<()> {
        let input = BlockHash("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_string());
        let bytes = input.0.as_bytes().to_vec();

        assert_eq!(input.clone().to_bytes(), bytes, "to_bytes");
        assert_eq!(input, BlockHash::from_bytes(&bytes)?, "from_bytes");
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

    #[test]
    fn test_is_valid_file_name() {
        // Valid cases
        assert!(is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234.json"),
            &is_valid_state_hash
        ));

        assert!(is_valid_file_name(
            Path::new("mainnet-3Nabcdef12345678901234567890123456789012345678901234.json"),
            &is_valid_state_hash
        ));

        // Invalid cases
        assert!(!is_valid_file_name(
            Path::new("mainnet-42-abcdef1234567890123456789012345678901234567890123456.json"), /* Invalid hash (does not start with 3N) */
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef1234.json"), // Hash too short
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42.json"), // Missing hash part
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234.txt"), /* Invalid extension */
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234-123.json"), /* Too many parts */
            &is_valid_state_hash
        ));
    }

    #[test]
    fn test_extract_height_and_hash() {
        // Valid cases
        assert_eq!(
            extract_height_and_hash(Path::new(
                "mainnet-42-3Nabcdef1234567890123456789012345678901234567890123456.json"
            )),
            (42, "3Nabcdef1234567890123456789012345678901234567890123456")
        );

        assert_eq!(
            extract_height_and_hash(Path::new(
                "mainnet-0-3Nabcdef1234567890123456789012345678901234567890123456.json"
            )),
            (0, "3Nabcdef1234567890123456789012345678901234567890123456")
        );

        // Invalid cases (should panic)
        let result = std::panic::catch_unwind(|| {
            extract_height_and_hash(Path::new("mainnet-42.json")); // missing hash part
        });
        assert!(
            result.is_err(),
            "Expected panic for invalid filename format"
        );

        let result = std::panic::catch_unwind(|| {
            extract_height_and_hash(Path::new(
                "mainnet-xyz-3Nabcdef1234567890123456789012345678901234567890123456.json",
            )); // non-numeric height
        });
        assert!(
            result.is_err(),
            "Expected panic for non-numeric block height"
        );
    }
}
