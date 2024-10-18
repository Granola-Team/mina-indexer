use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, LedgerHash},
    utility::store::{balance_key_prefix, pk_key_prefix, u32_from_be_bytes, U32_LEN, U64_LEN},
};
use anyhow::bail;

/// Split [staking_ledger_epoch_key] into constituent parts
pub fn split_staking_ledger_epoch_key(key: &[u8]) -> anyhow::Result<(BlockHash, u32, LedgerHash)> {
    if key.len() == BlockHash::LEN + U32_LEN + LedgerHash::LEN {
        let genesis_state_hash = BlockHash::from_bytes(&key[..BlockHash::LEN])?;
        let epoch = u32_from_be_bytes(&key[BlockHash::LEN..][..U32_LEN])?;
        let ledger_hash = LedgerHash::from_bytes(key[BlockHash::LEN..][U32_LEN..].to_vec())?;
        return Ok((genesis_state_hash, epoch, ledger_hash));
    }
    bail!("Invlid staking_ledger_epoch_key length")
}

/// Split [staking_ledger_sort_key] into constituent parts
pub fn split_staking_ledger_sort_key(key: &[u8]) -> anyhow::Result<(u32, u64, PublicKey)> {
    if key.len() == U32_LEN + U64_LEN + PublicKey::LEN {
        let epoch = u32_from_be_bytes(&key[..U32_LEN])?;
        let balance_or_stake = balance_key_prefix(&key[U32_LEN..]);
        let pk = pk_key_prefix(&key[U32_LEN..][U64_LEN..]);
        return Ok((epoch, balance_or_stake, pk));
    }
    bail!("Invlid staking_ledger_sort_key length")
}

/// Staking ledger amount sort key
/// ```
/// {epoch}{amount}{pk}
/// where
/// - epoch:  [u32] BE bytes
/// - amount: [u64] BE bytes
/// - pk:     [PublicKey::LEN] bytes
pub fn staking_ledger_sort_key(
    epoch: u32,
    amount: u64,
    pk: &PublicKey,
) -> [u8; U32_LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; U32_LEN + U64_LEN + PublicKey::LEN];
    key[..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[U32_LEN..][..U64_LEN].copy_from_slice(&amount.to_be_bytes());
    key[U32_LEN..][U64_LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Staking ledger account key
/// ```
/// {genesis_hash}{epoch}{ledger_hash}{pk}
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        [u32] BE bytes
/// - ledger_hash:  [LedgerHash::LEN] bytes
/// - pk:           [PublicKey::LEN] bytes
pub fn staking_ledger_account_key(
    genesis_state_hash: &BlockHash,
    epoch: u32,
    ledger_hash: &LedgerHash,
    pk: &PublicKey,
) -> [u8; BlockHash::LEN + U32_LEN + LedgerHash::LEN + PublicKey::LEN] {
    let mut key = [0; BlockHash::LEN + U32_LEN + LedgerHash::LEN + PublicKey::LEN];
    key[..BlockHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[BlockHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[BlockHash::LEN..][U32_LEN..][..LedgerHash::LEN].copy_from_slice(ledger_hash.0.as_bytes());
    key[BlockHash::LEN..][U32_LEN..][LedgerHash::LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Staking ledger epoch key
/// ```
/// {genesis_hash}{epoch}{ledger_hash}
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        [u32] BE bytes
/// - ledger_hash:  [LedgerHash::LEN] bytes
pub fn staking_ledger_epoch_key(
    genesis_state_hash: &BlockHash,
    epoch: u32,
    ledger_hash: &LedgerHash,
) -> [u8; BlockHash::LEN + U32_LEN + LedgerHash::LEN] {
    let mut key = [0; BlockHash::LEN + U32_LEN + LedgerHash::LEN];
    key[..BlockHash::LEN + U32_LEN]
        .copy_from_slice(&staking_ledger_epoch_key_prefix(genesis_state_hash, epoch));
    key[BlockHash::LEN..][U32_LEN..].copy_from_slice(ledger_hash.0.as_bytes());
    key
}

/// Prefix of [staking_ledger_epoch_key]
/// ```
/// - key: {genesis_hash}{epoch}
/// - val: aggregated epoch delegations serde bytes
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        [u32] BE bytes
pub fn staking_ledger_epoch_key_prefix(
    genesis_state_hash: &BlockHash,
    epoch: u32,
) -> [u8; BlockHash::LEN + U32_LEN] {
    let mut key = [0; BlockHash::LEN + U32_LEN];
    key[..BlockHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[BlockHash::LEN..].copy_from_slice(&epoch.to_be_bytes());
    key
}

#[cfg(test)]
mod staking_ledger_store_impl_tests {
    use super::*;

    #[test]
    fn test_staking_ledger_epoch_key_prefix() {
        let epoch = 42;
        let genesis_state_hash = BlockHash::default();
        let key = staking_ledger_epoch_key_prefix(&genesis_state_hash, epoch);

        // key == {gensis_state_hash}{epoch BE bytes}
        assert_eq!(&key[..BlockHash::LEN], genesis_state_hash.0.as_bytes());
        assert_eq!(&key[BlockHash::LEN..], &epoch.to_be_bytes());
    }

    #[test]
    fn test_staking_ledger_epoch_key() {
        let epoch = 42;
        let ledger_hash = LedgerHash::default();
        let genesis_state_hash = BlockHash::default();
        let key = staking_ledger_epoch_key(&genesis_state_hash, epoch, &ledger_hash);

        // key == {gensis_state_hash bytes}{epoch BE bytes}{ledger_hash bytes}
        assert_eq!(&key[..BlockHash::LEN], genesis_state_hash.0.as_bytes());
        assert_eq!(&key[BlockHash::LEN..][..U32_LEN], &epoch.to_be_bytes());
        assert_eq!(&key[BlockHash::LEN..][U32_LEN..], ledger_hash.0.as_bytes());
    }

    #[test]
    fn test_staking_ledger_account_key() {
        let epoch = 42;
        let pk = PublicKey::default();
        let state_hash = BlockHash::default();
        let ledger_hash = LedgerHash::default();
        let key = staking_ledger_account_key(&state_hash, epoch, &ledger_hash, &pk);

        // key == {gensis_state_hash}{epoch BE bytes}{ledger_hash bytes}{pk bytes}
        assert_eq!(&key[..BlockHash::LEN], state_hash.0.as_bytes());
        assert_eq!(&key[BlockHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            &key[BlockHash::LEN..][U32_LEN..][..LedgerHash::LEN],
            ledger_hash.0.as_bytes()
        );
        assert_eq!(
            &key[BlockHash::LEN..][U32_LEN..][LedgerHash::LEN..],
            pk.0.as_bytes()
        );
    }
}
