use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    utility::store::common::{u32_from_be_bytes, U32_LEN},
};

pub fn internal_commmand_block_key(
    state_hash: &StateHash,
    index: u32,
) -> [u8; StateHash::LEN + U32_LEN] {
    let mut bytes = [0; StateHash::LEN + U32_LEN];
    bytes[..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[StateHash::LEN..].copy_from_slice(&index.to_be_bytes());
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
    state_hash: &StateHash,
    index: u32,
) -> [u8; U32_LEN + StateHash::LEN + U32_LEN] {
    let mut bytes = [0; U32_LEN + StateHash::LEN + U32_LEN];
    bytes[..U32_LEN].copy_from_slice(&prefix.to_be_bytes());
    bytes[U32_LEN..][..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[U32_LEN + StateHash::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_commmand_pk_sort_key(
    pk: &PublicKey,
    sort: u32,
    state_hash: &StateHash,
    index: u32,
    kind: u8,
) -> [u8; PublicKey::LEN + U32_LEN + StateHash::LEN + U32_LEN + 1] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN + StateHash::LEN + U32_LEN + 1];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..][..U32_LEN].copy_from_slice(&sort.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][StateHash::LEN..][..U32_LEN]
        .copy_from_slice(&index.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][StateHash::LEN..][U32_LEN] = kind;
    bytes
}

pub fn internal_command_pk_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..].copy_from_slice(&index.to_be_bytes());
    bytes
}

pub fn internal_command_pk_sort_key_state_hash(key: &[u8]) -> StateHash {
    StateHash::from_bytes(&key[PublicKey::LEN + U32_LEN..][..StateHash::LEN])
        .expect("block state hash bytes")
}

pub fn internal_command_pk_sort_key_index(key: &[u8]) -> u32 {
    u32_from_be_bytes(&key[PublicKey::LEN..][U32_LEN..][StateHash::LEN..][..U32_LEN])
        .expect("internal command pk sort key index u32 bytes")
}

/// `PublicKey::LEN + U32_LEN + StateHash::LEN + U32_LEN`-th byte
pub fn internal_command_pk_sort_key_kind(key: &[u8]) -> u8 {
    key[PublicKey::LEN + U32_LEN + StateHash::LEN + U32_LEN]
}
