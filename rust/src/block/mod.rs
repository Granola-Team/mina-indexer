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
    chain_id::Network,
    protocol::serialization_types::{
        common::{Base58EncodableVersionedType, HashV1},
        version_bytes,
    },
};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::Path};

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub height: u32,
    pub blockchain_length: u32,
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
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let block_hash = unsafe { String::from_utf8_unchecked(Vec::from(bytes)) };
        Self(block_hash)
    }

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
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
        let parent_hash = precomputed_block.previous_state_hash();
        let state_hash = precomputed_block.state_hash();
        Self {
            parent_hash,
            state_hash,
            canonicity: None,
            global_slot_since_genesis: precomputed_block
                .consensus_state()
                .global_slot_since_genesis
                .t
                .t,
            blockchain_length: precomputed_block.blockchain_length(),
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

impl From<PrecomputedBlock> for Block {
    fn from(value: PrecomputedBlock) -> Self {
        Self {
            height: value.blockchain_length().saturating_sub(1),
            parent_hash: value.previous_state_hash(),
            blockchain_length: value.blockchain_length(),
            state_hash: value.state_hash(),
            hash_last_vrf_output: value.hash_last_vrf_output(),
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
    if state_hash.starts_with("3N") {
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
    input.starts_with("3N") && input.len() == 52
}

pub fn is_valid_file_name(path: &Path, hash_validator: &dyn Fn(&str) -> bool) -> bool {
    if let Some(ext) = path.extension() {
        // check json extension
        if ext.to_str() == Some("json") {
            // check file stem
            if let Some(file_name) = path.file_stem() {
                if let Some(parts) = file_name
                    .to_str()
                    .map(|name| name.split('-').collect::<Vec<&str>>())
                {
                    let is_valid_hash = parts
                        .last()
                        .map(|hash| hash_validator(hash))
                        .unwrap_or(false);
                    if parts.len() == 2 {
                        // e.g. mainnet-3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8.json
                        // check 2nd part is a state hash
                        return is_valid_hash;
                    } else if parts.len() == 3 {
                        // e.g. mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json
                        // check 2nd part is u32 and 3rd part is a state hash
                        let is_valid_length = parts.get(1).unwrap().parse::<u32>().is_ok();
                        return is_valid_hash && is_valid_length;
                    }
                }
            }
        }
    }
    false
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

pub fn extract_block_height(path: &Path) -> Option<u32> {
    let filename = path.file_name().and_then(|x| x.to_str()).unwrap();
    let first_dash = filename.find('-');
    let second_dash =
        first_dash.and_then(|index| filename[index + 1..].find('-').map(|i| i + index + 1));
    if let (Some(first_dash_pos), Some(second_dash_pos)) = (first_dash, second_dash) {
        let potential_block_height = &filename[first_dash_pos + 1..second_dash_pos];
        return potential_block_height.parse::<u32>().ok();
    }
    None
}

pub fn extract_block_height_or_max(path: &Path) -> u32 {
    extract_block_height(path).unwrap_or(u32::MAX)
}

pub fn extract_state_hash(path: &Path) -> String {
    let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
    let dash_pos = name.rfind('-').unwrap();
    let state_hash = &name[dash_pos + 1..];
    state_hash.to_owned()
}

pub fn extract_network(path: &Path) -> Network {
    let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
    let dash_pos = name.find('-').unwrap();
    let network = &name[..dash_pos];
    Network::from(network)
}

#[cfg(test)]
mod tests {
    use super::{
        extract_block_height_or_max, extract_state_hash, is_valid_state_hash,
        precomputed::PrecomputedBlock, Block, BlockHash,
    };
    use crate::block::precomputed::PcbVersion;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_block_hash_is_valid_public_key() {
        assert!(is_valid_state_hash(&BlockHash::default().0))
    }

    #[test]
    fn extract_state_hash_test() {
        let filename1 =
            &Path::new("mainnet-3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8.json");
        let filename2 =
            &Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let filename3 = &Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(
            "3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8".to_owned(),
            extract_state_hash(filename1)
        );
        assert_eq!(
            "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH".to_owned(),
            extract_state_hash(filename2)
        );
        assert_eq!(
            "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R".to_owned(),
            extract_state_hash(filename3)
        );
    }

    #[test]
    fn extract_block_height_or_max_test() {
        let filename1 =
            &Path::new("mainnet-3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8.json");
        let filename2 =
            &Path::new("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
        let filename3 = &Path::new(
            "/tmp/blocks/mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
        );

        assert_eq!(u32::MAX, extract_block_height_or_max(filename1));
        assert_eq!(2, extract_block_height_or_max(filename2));
        assert_eq!(3, extract_block_height_or_max(filename3));
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

        let path0: PathBuf = "./tests/initial-blocks/mainnet-10-3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5.json".into();
        let path1: PathBuf = "./tests/initial-blocks/mainnet-10-3NKHYHrqKpDcon6ToV5CLDiheanjshk5gcsNqefnK78phCFTR2aL.json".into();
        let block0: Block = PrecomputedBlock::parse_file(&path0, PcbVersion::V1)?.into();
        let block1: Block = PrecomputedBlock::parse_file(&path1, PcbVersion::V1)?.into();

        assert!(block0 < block1);
        Ok(())
    }
}
