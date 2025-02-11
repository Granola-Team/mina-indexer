use crate::{
    base::{nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    command::signed::TxnHash,
    utility::store::common::{state_hash_suffix, u32_from_be_bytes, U32_LEN},
};

/// Key format for sorting txns by block height/global slot & txn hash
/// `{prefix}{txn_hash}{state_hash}`
/// ```
/// - prefix:     [u32] BE bytes
/// - txn_hash:   [TxnHash::V1_LEN] bytes
/// - state_hash: [StateHash::LEN] bytes
pub fn txn_sort_key(
    prefix: u32,
    txn_hash: &TxnHash,
    state_hash: &StateHash,
) -> [u8; U32_LEN + TxnHash::V1_LEN + StateHash::LEN] {
    let mut bytes = [0; U32_LEN + TxnHash::V1_LEN + StateHash::LEN];
    bytes[..U32_LEN].copy_from_slice(&prefix.to_be_bytes());
    bytes[U32_LEN..][..TxnHash::V1_LEN].copy_from_slice(&txn_hash.right_pad_v2());
    bytes[U32_LEN..][TxnHash::V1_LEN..].copy_from_slice(state_hash.0.as_bytes());
    bytes
}

/// Key format for sorting txns by sender/receiver:
/// `{pk}{u32_sort}{nonce}{txn_hash}{state_hash}`
/// ```
/// - pk:         [PublicKey::LEN] bytes
/// - u32_sort:   [u32] BE bytes
/// - nonce:      [u32] BE bytes
/// - txn_hash:   [TxnHash::V1_LEN] bytes
/// - state_hash: [StateHash::LEN] bytes
pub fn pk_txn_sort_key(
    pk: &PublicKey,
    sort: u32,
    nonce: u32,
    txn_hash: &TxnHash,
    state_hash: &StateHash,
) -> [u8; PublicKey::LEN + U32_LEN + U32_LEN + TxnHash::V1_LEN + StateHash::LEN] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN + U32_LEN + TxnHash::V1_LEN + StateHash::LEN];
    bytes[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    bytes[PublicKey::LEN..][..U32_LEN].copy_from_slice(&sort.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&nonce.to_be_bytes());
    bytes[PublicKey::LEN..][U32_LEN..][U32_LEN..][..TxnHash::V1_LEN]
        .copy_from_slice(&txn_hash.right_pad_v2());
    bytes[PublicKey::LEN..][U32_LEN..][U32_LEN..][TxnHash::V1_LEN..]
        .copy_from_slice(state_hash.0.as_bytes());
    bytes
}

/// Prefix `{pk}{u32_sort}`
pub fn pk_txn_sort_key_prefix(public_key: &PublicKey, sort: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut bytes = [0; PublicKey::LEN + U32_LEN];
    bytes[..PublicKey::LEN].copy_from_slice(public_key.0.as_bytes());
    bytes[PublicKey::LEN..].copy_from_slice(&sort.to_be_bytes());
    bytes
}

/// Drop [PublicKey::LEN] + [U32_LEN] bytes & parse the next [U32_LEN] bytes
pub fn pk_txn_sort_key_nonce(key: &[u8]) -> Nonce {
    u32_from_be_bytes(&key[PublicKey::LEN..][U32_LEN..][..U32_LEN])
        .expect("u32 nonce BE bytes")
        .into()
}

/// Drop [PublicKey::LEN] + [U32_LEN] + [U32_LEN] bytes & parse the next
/// [TxnHash::V1_LEN] bytes
pub fn txn_hash_of_key(key: &[u8]) -> TxnHash {
    TxnHash::from_bytes(key[PublicKey::LEN..][U32_LEN..][U32_LEN..][..TxnHash::V1_LEN].to_vec())
        .expect("txn hash")
}

/// Drop [PublicKey::LEN] + [U32_LEN] + [U32_LEN] + [TxnHash::V1_LEN] bytes &
/// parse the remaining [StateHash::LEN] bytes
pub fn pk_txn_sort_key_state_hash(key: &[u8]) -> StateHash {
    state_hash_suffix(key).expect("state hash bytes")
}

/// Right-pad v2 txn hashes to match v1 length
pub fn txn_block_key(
    txn_hash: &TxnHash,
    state_hash: &StateHash,
) -> [u8; TxnHash::V1_LEN + StateHash::LEN] {
    let mut key = [0; TxnHash::V1_LEN + StateHash::LEN];
    key[..TxnHash::V1_LEN].copy_from_slice(&txn_hash.right_pad_v2());
    key[TxnHash::V1_LEN..].copy_from_slice(state_hash.0.as_bytes());
    key
}

/// u32 prefix from `key`
/// - keep the first U32_LEN bytes
/// - used for global slot & block height
/// - [user_commands_slot_iterator] & [user_commands_height_iterator]
pub fn user_commands_iterator_u32_prefix(key: &[u8]) -> u32 {
    u32_from_be_bytes(&key[..U32_LEN]).expect("u32 bytes")
}

/// Transaction hash from `key`
/// - discard [U32_LEN] bytes, keep [TxnHash::V1_LEN] bytes
/// - [user_commands_slot_iterator] & [user_commands_height_iterator]
pub fn user_commands_iterator_txn_hash(key: &[u8]) -> anyhow::Result<TxnHash> {
    TxnHash::from_bytes(key[U32_LEN..][..TxnHash::V1_LEN].to_vec())
}

/// State hash from `key`
/// - discard the first [U32_LEN] + [TxnHash::V1_LEN] bytes
/// - [user_commands_slot_iterator] & [user_commands_height_iterator]
pub fn user_commands_iterator_state_hash(key: &[u8]) -> anyhow::Result<StateHash> {
    StateHash::from_bytes(&key[U32_LEN..][TxnHash::V1_LEN..])
}
