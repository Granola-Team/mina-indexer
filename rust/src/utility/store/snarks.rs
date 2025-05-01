//! SNARK store helpers

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    utility::store::common::{U32_LEN, U64_LEN},
};

/// Key format
/// ```
/// {genesis}{epoch}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
pub fn snarks_epoch_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
) -> [u8; StateHash::LEN + U32_LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..].copy_from_slice(&epoch.to_be_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{prover}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - prover:  [PublicKey] bytes
pub fn snarks_pk_epoch_key(
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
/// {genesis}{epoch}{prover}{height}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - prover:  [PublicKey] bytes
/// - height:  [u32] BE bytes
pub fn snarks_epoch_pk_height_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    prover: &PublicKey,
    block_height: u32,
) -> [u8; StateHash::LEN + U32_LEN + PublicKey::LEN + U32_LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + PublicKey::LEN + U32_LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][..PublicKey::LEN].copy_from_slice(prover.0.as_bytes());
    key[StateHash::LEN..][U32_LEN..][PublicKey::LEN..].copy_from_slice(&block_height.to_be_bytes());

    key
}

/// Key format
/// ```
/// {fee}{sort}{pk}{hash}{index}
/// where
/// fee:   [u64] BE bytes
/// sort:  [u32] BE bytes
/// pk:    [PublicKey] bytes
/// hash:  [StateHash] bytes
/// index: [u32] BE bytes
pub fn snark_fee_sort_key(
    fee: u64,
    u32_sort: u32,
    pk: &PublicKey,
    state_hash: &StateHash,
    index: u32,
) -> [u8; U64_LEN + U32_LEN + PublicKey::LEN + StateHash::LEN + U32_LEN] {
    let mut key = [0; U64_LEN + U32_LEN + PublicKey::LEN + StateHash::LEN + U32_LEN];

    key[..U64_LEN].copy_from_slice(&fee.to_be_bytes());
    key[U64_LEN..][..U32_LEN].copy_from_slice(&u32_sort.to_be_bytes());
    key[U64_LEN..][U32_LEN..][..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][..StateHash::LEN]
        .copy_from_slice(state_hash.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][StateHash::LEN..]
        .copy_from_slice(&index.to_be_bytes());

    key
}

/// Key format
/// ```
/// {prover}{sort}{index}
/// where
/// - prover: [PublicKey] bytes
/// - sort:   [u32] BE bytes
/// - index:  [u32] BE bytes
pub fn snark_prover_sort_key(
    prover: &PublicKey,
    u32_sort: u32,
    index: u32,
) -> [u8; PublicKey::LEN + U32_LEN + U32_LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN + U32_LEN];

    key[..PublicKey::LEN].copy_from_slice(prover.0.as_bytes());
    key[PublicKey::LEN..][..U32_LEN].copy_from_slice(&u32_sort.to_be_bytes());
    key[PublicKey::LEN..][U32_LEN..].copy_from_slice(&index.to_be_bytes());

    key
}

/// Key format
/// ```
/// {genesis}{epoch}{fee}{prover}
/// where
/// - genesis: [StateHash] bytes
/// - epoch:   [u32] BE bytes
/// - fee:     [u64] BE bytes
/// - prover:  [PublicKey] bytes
pub fn snark_fee_epoch_sort_key(
    genesis_state_hash: &StateHash,
    epoch: u32,
    fee: u64,
    prover: &PublicKey,
) -> [u8; StateHash::LEN + U32_LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; StateHash::LEN + U32_LEN + U64_LEN + PublicKey::LEN];

    key[..StateHash::LEN].copy_from_slice(genesis_state_hash.0.as_bytes());
    key[StateHash::LEN..][..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][..U64_LEN].copy_from_slice(&fee.to_be_bytes());
    key[StateHash::LEN..][U32_LEN..][U64_LEN..].copy_from_slice(prover.0.as_bytes());

    key
}

#[cfg(test)]
mod tests {
    use crate::{
        base::{public_key::PublicKey, state_hash::StateHash},
        utility::store::common::{U32_LEN, U64_LEN},
    };
    use quickcheck::{Arbitrary, Gen};

    const GEN_SIZE: usize = 1000;

    #[test]
    fn snarks_epoch_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);

        let key = super::snarks_epoch_key(&genesis_state_hash, epoch);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..], epoch.to_be_bytes());
    }

    #[test]
    fn snarks_pk_epoch_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let prover = PublicKey::arbitrary(g);

        let key = super::snarks_pk_epoch_key(&genesis_state_hash, epoch, &prover);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(key[StateHash::LEN..][U32_LEN..], *prover.0.as_bytes());
    }

    #[test]
    fn snarks_epoch_pk_height_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let prover = PublicKey::arbitrary(g);
        let block_height = u32::arbitrary(g);

        let key =
            super::snarks_epoch_pk_height_key(&genesis_state_hash, epoch, &prover, block_height);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][..PublicKey::LEN],
            *prover.0.as_bytes()
        );
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][PublicKey::LEN..],
            block_height.to_be_bytes()
        );
    }

    #[test]
    fn snark_fee_sort_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let fee = u64::arbitrary(g);
        let index = u32::arbitrary(g);
        let block_height = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);
        let state_hash = StateHash::arbitrary(g);

        let key = super::snark_fee_sort_key(fee, block_height, &pk, &state_hash, index);

        assert_eq!(key[..U64_LEN], fee.to_be_bytes());
        assert_eq!(key[U64_LEN..][..U32_LEN], block_height.to_be_bytes());
        assert_eq!(
            key[U64_LEN..][U32_LEN..][..PublicKey::LEN],
            *pk.0.as_bytes()
        );
        assert_eq!(
            key[U64_LEN..][U32_LEN..][PublicKey::LEN..][..StateHash::LEN],
            *state_hash.0.as_bytes()
        );
        assert_eq!(
            key[U64_LEN..][U32_LEN..][PublicKey::LEN..][StateHash::LEN..],
            index.to_be_bytes()
        );
    }

    #[test]
    fn snark_prover_sort_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let index = u32::arbitrary(g);
        let block_height = u32::arbitrary(g);
        let pk = PublicKey::arbitrary(g);

        let key = super::snark_prover_sort_key(&pk, block_height, index);

        assert_eq!(key[..PublicKey::LEN], *pk.0.as_bytes());
        assert_eq!(key[PublicKey::LEN..][..U32_LEN], block_height.to_be_bytes());
        assert_eq!(key[PublicKey::LEN..][U32_LEN..], index.to_be_bytes());
    }

    #[test]
    fn snark_fee_epoch_sort_key() {
        let g = &mut Gen::new(GEN_SIZE);

        let genesis_state_hash = StateHash::arbitrary(g);
        let epoch = u32::arbitrary(g);
        let fee = u64::arbitrary(g);
        let prover = PublicKey::arbitrary(g);

        let key = super::snark_fee_epoch_sort_key(&genesis_state_hash, epoch, fee, &prover);

        assert_eq!(key[..StateHash::LEN], *genesis_state_hash.0.as_bytes());
        assert_eq!(key[StateHash::LEN..][..U32_LEN], epoch.to_be_bytes());
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][..U64_LEN],
            fee.to_be_bytes()
        );
        assert_eq!(
            key[StateHash::LEN..][U32_LEN..][U64_LEN..],
            *prover.0.as_bytes()
        );
    }
}
