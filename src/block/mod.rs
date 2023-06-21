use std::{ffi::OsStr, path::Path};

use mina_serialization_types::{common::Base58EncodableVersionedType, v1::HashV1, version_bytes};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use std::{
    fs::File,
    io::Read,
};
use regex::Regex;

use self::precomputed::{BlockLogContents, PrecomputedBlock};

pub mod parser;
pub mod precomputed;
pub mod receiver;
pub mod store;

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub height: u32,
    pub blockchain_length: Option<u32>,
    pub global_slot_number: Option<u32>,
}

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BlockWithoutHeight {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub blockchain_length: Option<u32>,
}

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

    pub fn previous_state_hash(block: &PrecomputedBlock) -> Self {
        Self::from_hashv1(block.protocol_state.previous_state_hash.clone())
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock, height: u32) -> Self {
        let parent_hash =
            BlockHash::from_hashv1(precomputed_block.protocol_state.previous_state_hash.clone());
        let state_hash = BlockHash(precomputed_block.state_hash.clone());
        Self {
            parent_hash,
            state_hash,
            height,
            blockchain_length: precomputed_block.blockchain_length,
            global_slot_number: precomputed_block.global_slot_number,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{{ len: {}, state: {} }}",
            self.blockchain_length
                .map_or("unknown".to_string(), |len| len.to_string()),
            self.state_hash.0
        )
    }
}

impl BlockWithoutHeight {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let parent_hash =
            BlockHash::from_hashv1(precomputed_block.protocol_state.previous_state_hash.clone());
        let state_hash = BlockHash(precomputed_block.state_hash.clone());
        Self {
            parent_hash,
            state_hash,
            blockchain_length: precomputed_block.blockchain_length,
        }
    }
}

impl std::cmp::PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.state_hash == other.state_hash {
            Some(std::cmp::Ordering::Equal)
        } else if self.height > other.height
            || self.height == other.height && self.state_hash.0 > other.state_hash.0
        {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Less)
        }
    }
}

impl std::cmp::Ord for Block {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Block {{ height: {}, len: {}, state: {}, parent: {}, slot: {} }}",
            self.height,
            self.blockchain_length.unwrap_or(0),
            &self.state_hash.0[0..12],
            &self.parent_hash.0[0..12],
            self.global_slot_number.unwrap_or(0),
        )
    }
}

impl std::fmt::Debug for BlockWithoutHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Block {{ len: {}, state: {}, parent: {} }}",
            self.blockchain_length.unwrap_or(0),
            &self.state_hash.0[0..12],
            &self.parent_hash.0[0..12]
        )
    }
}

impl std::fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BlockHash {{ {:?} }}", self.0)
    }
}

pub async fn parse_file(filename: &Path) -> anyhow::Result<PrecomputedBlock> {
    if is_valid_block_file(filename) {
        let blockchain_length =
            get_blockchain_length(filename.file_name().expect("filename already checked"));
        let state_hash = get_state_hash(filename.file_name().expect("filename already checked"))
            .expect("state hash already checked");

        let mut log_file = tokio::fs::File::open(&filename).await?;
        let mut log_file_contents = Vec::new();

        log_file.read_to_end(&mut log_file_contents).await?;

        let global_slot_number = extract_global_slot_number(&String::from_utf8_lossy(&log_file_contents));

        let precomputed_block = PrecomputedBlock::from_log_contents(BlockLogContents {
            state_hash,
            blockchain_length,
            contents: log_file_contents,
            global_slot_number,
        })?;

        Ok(precomputed_block)
    } else {
        Err(anyhow::Error::msg(format!(
            "
[PrecomputedBlock::parse_file]
Could not find valid block!
{:} is not a valid Precomputed Block",
            filename.display()
        )))
    }
}

/// extract a state hash from an OS file name
fn get_state_hash(file_name: &OsStr) -> Option<String> {
    let last_part = file_name.to_str()?.split('-').last()?.to_string();
    if last_part.starts_with('.') {
        return None;
    }
    if !last_part.starts_with("3N") {
        return None;
    }
    let state_hash = last_part.split('.').next()?;
    if state_hash.contains('.') {
        return None;
    }
    Some(state_hash.to_string())
}

/// extract a blockchain length from an OS file name
fn get_blockchain_length(file_name: &OsStr) -> Option<u32> {
    file_name
        .to_str()?
        .split('-')
        .fold(None, |acc, x| match x.parse::<u32>() {
            Err(_) => acc,
            Ok(x) => Some(x),
        })
}

fn is_valid_block_file(path: &Path) -> bool {
    let file_name = path.file_name();
    if let Some(file_name) = file_name {
        get_state_hash(file_name).is_some()
            && file_name
                .to_str()
                .map(|file_name| file_name.ends_with(".json"))
                .unwrap_or(false)
    } else {
        false
    }
}

fn extract_global_slot_number(file_path: &str) -> Option<u32> {
    let mut file = File::open(file_path).ok()?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).ok()?;

    let regex = Regex::new(r"global_slot_since_genesis=(\d+)").unwrap();
    let contents_str = String::from_utf8_lossy(&contents);

    if let Some(captures) = regex.captures(&contents_str) {
        if let Some(slot_str) = captures.get(1) {
            if let Ok(slot) = slot_str.as_str().parse::<u32>() {
                return Some(slot);
            }
        }
    }

    None
}
