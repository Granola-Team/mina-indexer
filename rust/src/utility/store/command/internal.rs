use crate::{
    block::BlockHash,
    ledger::public_key::PublicKey,
    utility::store::{u32_from_be_bytes, U32_LEN},
};

pub fn internal_commmand_block_key(
    state_hash: &BlockHash,
    index: u32,
) -> [u8; BlockHash::LEN + U32_LEN] {
    let mut bytes = [0; BlockHash::LEN + U32_LEN];
    bytes[..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[BlockHash::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_commmand_pk_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_commmand_sort_key(
    prefix: u32,
    state_hash: &BlockHash,
    index: u32,
) -> [u8; U32_LEN + BlockHash::LEN + U32_LEN] {
    let mut bytes = [0; U32_LEN + BlockHash::LEN + U32_LEN];
    bytes[..U32_LEN].copy_from_slice(&prefix.to_be_bytes());
    bytes[U32_LEN..][..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[U32_LEN + BlockHash::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_commmand_pk_sort_key(
    pk: &PublicKey,
    sort: u32,
    state_hash: &BlockHash,
    index: u32,
    kind: u8,
) -> [u8; PublicKey::LEN + U32_LEN + BlockHash::LEN + U32_LEN + 1] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN + BlockHash::LEN + U32_LEN + 1];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..][..U32_LEN].copy_from_slice(&sort.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][BlockHash::LEN..][..U32_LEN]
        .copy_from_slice(&index.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][BlockHash::LEN..][U32_LEN] = kind;
    bytes
}

pub fn internal_command_pk_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_command_pk_sort_key_state_hash(key: &[u8]) -> BlockHash {
    BlockHash::from_bytes(&key[PublicKey::LEN + U32_LEN..][..BlockHash::LEN])
        .expect("block state hash bytes")
}

pub fn internal_command_pk_sort_key_index(key: &[u8]) -> u32 {
    u32_from_be_bytes(&key[PublicKey::LEN..][U32_LEN..][BlockHash::LEN..][..U32_LEN])
        .expect("internal command pk sort key index u32 bytes")
}

/// `PublicKey::LEN + U32_LEN + BlockHash::LEN + U32_LEN`-th byte
pub fn internal_command_pk_sort_key_kind(key: &[u8]) -> u8 {
    key[PublicKey::LEN + U32_LEN + BlockHash::LEN + U32_LEN]
}
