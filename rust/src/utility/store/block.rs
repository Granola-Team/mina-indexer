//! Block store helpers

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::precomputed::PrecomputedBlock,
    utility::store::common::U32_LEN,
};

/// Key format
/// ```
/// {height}{state}
/// where
/// - height: [u32] BE bytes
/// - state:  [StateHash] bytes
pub fn block_height_key(block: &PrecomputedBlock) -> [u8; U32_LEN + StateHash::LEN] {
    let mut key = [0; U32_LEN + StateHash::LEN];

    key[..U32_LEN].copy_from_slice(&block.blockchain_length().to_be_bytes());
    key[U32_LEN..].copy_from_slice(block.state_hash().0.as_bytes());

    key
}

/// Key format
/// ```
/// {slot}{state}
/// where
/// - slot:  [u32] BE bytes
/// - state: [StateHash] bytes
pub fn block_global_slot_key(block: &PrecomputedBlock) -> [u8; U32_LEN + StateHash::LEN] {
    let mut key = [0; U32_LEN + StateHash::LEN];

    key[..U32_LEN].copy_from_slice(&block.global_slot_since_genesis().to_be_bytes());
    key[U32_LEN..].copy_from_slice(block.state_hash().0.as_bytes());

    key
}

/// Key format
/// ```
/// {pk}{sort_value}{state_hash}
/// where
/// - pk:         [PublicKey] bytes
/// - sort_value: [u32] BE bytes
/// - state_hash: [StateHash] bytes
pub fn pk_block_sort_key(
    pk: &PublicKey,
    sort_value: u32,
    state_hash: &StateHash,
) -> [u8; PublicKey::LEN + U32_LEN + StateHash::LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN + StateHash::LEN];

    key[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[PublicKey::LEN..][..U32_LEN].copy_from_slice(&sort_value.to_be_bytes());
    key[PublicKey::LEN..][U32_LEN..].copy_from_slice(state_hash.0.as_bytes());

    key
}

/// Key format
/// ```
/// {prefix}{num}
/// where
/// - prefix: [u32] BE bytes (blockchain length, global slot, etc)
/// - num:    [u32] BE bytes
pub fn block_num_key(prefix: u32, num: u32) -> [u8; U32_LEN + U32_LEN] {
    let mut key = [0; U32_LEN + U32_LEN];

    key[..U32_LEN].copy_from_slice(&prefix.to_be_bytes());
    key[U32_LEN..].copy_from_slice(&num.to_be_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
pub fn epoch_key(genesis_state_hash: &StateHash, epoch: u32) -> [u8; StateHash::LEN + U32_LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..].copy_from_slice(&epoch.to_be_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{pk}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - pk:      [PublicKey] bytes
pub fn epoch_pk_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    pk: &PublicKey,
) -> [u8; StateHash::LEN + U32_LEN + PublicKey::LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + PublicKey::LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..].copy_from_slice(pk.0.as_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{pk}{num}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - pk:      [PublicKey] bytes
/// - num:     [u32] BE bytes
pub fn epoch_pk_num_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    pk: &PublicKey,
    num: u32,
) -> [u8; StateHash::LEN + U32_LEN + PublicKey::LEN + U32_LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + PublicKey::LEN + U32_LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[StateHash::LEN..][U32_LEN..][PublicKey::LEN..].copy_from_slice(&num.to_be_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{num}{pk}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - num:     [u32] BE bytes
/// - pk:      [PublicKey] bytes
pub fn epoch_num_pk_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    num: u32,
    pk: &PublicKey,
) -> [u8; StateHash::LEN + U32_LEN + U32_LEN + PublicKey::LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + U32_LEN + PublicKey::LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][..U32_LEN].copy_from_slice(&num.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][U32_LEN..].copy_from_slice(pk.0.as_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{num}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - num:     [u32] BE bytes
pub fn epoch_num_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    num: u32,
) -> [u8; StateHash::LEN + U32_LEN + U32_LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + U32_LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..].copy_from_slice(&num.to_be_bytes());

    key
}

#[cfg(test)]
mod tests {
    use crate::{
        base::{public_key::PublicKey, state_hash::StateHash},
        block::precomputed::{PcbVersion, PrecomputedBlock},
        utility::store::common::U32_LEN,
    };
    use quickcheck::{Arbitrary, Gen};
    use std::path::PathBuf;

    const GEN_SIZE: usize = 1000;

    #[test]
    fn block_height_key() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let key = super::block_height_key(&block);

        assert_eq!(
            block.state_hash().0.as_bytes(),
            "3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh".as_bytes()
        );
        assert_eq!(key[..U32_LEN], block.blockchain_length().to_be_bytes());
        assert_eq!(key[U32_LEN..], *block.state_hash().0.as_bytes());
        Ok(())
    }

    #[test]
    fn block_global_slot_key() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let result = super::block_global_slot_key(&block);

        assert_eq!(
            result[..U32_LEN],
            block.global_slot_since_genesis().to_be_bytes()
        );
        assert_eq!(result[U32_LEN..], *block.state_hash().0.as_bytes());

        Ok(())
    }

    #[test]
    fn pk_block_sort_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let sort_value = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);
        let state_hash = StateHash::arbitrary(g);

        let key = super::pk_block_sort_key(&pk, sort_value, &state_hash);

        assert_eq!(key[..PublicKey::LEN], *pk.0.as_bytes());
        assert_eq!(key[PublicKey::LEN..][..U32_LEN], sort_value.to_be_bytes());
        assert_eq!(key[PublicKey::LEN..][U32_LEN..], *state_hash.0.as_bytes());
    }

    #[test]
    fn block_num_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let prefix = u32::arbitrary(g);
        let num = u32::arbitrary(g);

        let key = super::block_num_key(prefix, num);

        assert_eq!(key[..U32_LEN], prefix.to_be_bytes());
        assert_eq!(key[U32_LEN..], num.to_be_bytes());
    }

    #[test]
    fn epoch_pk_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);

        let key = super::epoch_pk_key(&genesis_state_hash, epoch, &pk);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(key[StateHash::LEN..][U32_LEN..], *pk.0.as_bytes());
    }

    #[test]
    fn epoch_pk_num_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);
        let num = u32::arbitrary(g);

        let key = super::epoch_pk_num_key(&genesis_state_hash, epoch, &pk, num);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][..PublicKey::LEN],
            *pk.0.as_bytes()
        );
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][PublicKey::LEN..],
            num.to_be_bytes()
        );
    }

    #[test]
    fn epoch_num_pk_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let num = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);

        let key = super::epoch_num_pk_key(&genesis_state_hash, epoch, num, &pk);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][..U32_LEN],
            num.to_be_bytes()
        );
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][U32_LEN..],
            *pk.0.as_bytes()
        );
    }

    #[test]
    fn epoch_num_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let num = u32::arbitrary(g);

        let key = super::epoch_num_key(&genesis_state_hash, epoch, num);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][..U32_LEN],
            num.to_be_bytes()
        );
    }
}
