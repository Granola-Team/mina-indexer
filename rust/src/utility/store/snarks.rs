use crate::{
    block::BlockHash,
    ledger::public_key::PublicKey,
    utility::store::{U32_LEN, U64_LEN},
};

/// Key format
/// ```
/// {epoch}{prover}
/// where
/// epoch:  [u32] BE bytes
/// prover: [PublicKey] bytes
pub fn snark_epoch_key(epoch: u32, pk: &PublicKey) -> [u8; U32_LEN + PublicKey::LEN] {
    let mut key = [0; U32_LEN + PublicKey::LEN];
    key[..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[U32_LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Key format
/// ```
/// {fee}{sort}{pk}{hash}{index}
/// where
/// fee:   [u64] BE bytes
/// sort:  [u32] BE bytes
/// pk:    [PublicKey] bytes
/// hash:  [BlockHash] bytes
/// index: [u32] BE bytes
pub fn snark_fee_sort_key(
    fee: u64,
    u32_sort: u32,
    pk: &PublicKey,
    state_hash: &BlockHash,
    index: u32,
) -> [u8; U64_LEN + U32_LEN + PublicKey::LEN + BlockHash::LEN + U32_LEN] {
    let mut key = [0; U64_LEN + U32_LEN + PublicKey::LEN + BlockHash::LEN + U32_LEN];
    key[..U64_LEN].copy_from_slice(&fee.to_be_bytes());
    key[U64_LEN..][..U32_LEN].copy_from_slice(&u32_sort.to_be_bytes());
    key[U64_LEN..][U32_LEN..][..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][..BlockHash::LEN]
        .copy_from_slice(state_hash.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][BlockHash::LEN..]
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
/// {epoch}{fee}{prover}
/// where
/// - epoch:  [u32] BE bytes
/// - fee:    [u64] BE bytes
/// - prover: [PublicKey] bytes
pub fn snark_fee_epoch_sort_key(
    epoch: u32,
    fee: u64,
    prover: &PublicKey,
) -> [u8; U32_LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; U32_LEN + U64_LEN + PublicKey::LEN];
    key[..U32_LEN].copy_from_slice(&epoch.to_be_bytes());
    key[U32_LEN..][..U64_LEN].copy_from_slice(&fee.to_be_bytes());
    key[U32_LEN..][U64_LEN..].copy_from_slice(prover.0.as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snark_fee_sort_key() {
        let fee = 100;
        let index = 25;
        let block_height = 50;
        let pk = PublicKey::default();
        let state_hash = BlockHash::default();
        let key = snark_fee_sort_key(fee, block_height, &pk, &state_hash, index);

        assert_eq!(&key[..U64_LEN], &fee.to_be_bytes());
        assert_eq!(&key[U64_LEN..][..U32_LEN], &block_height.to_be_bytes());
        assert_eq!(
            &key[U64_LEN..][U32_LEN..][..PublicKey::LEN],
            pk.0.as_bytes()
        );
        assert_eq!(
            &key[U64_LEN..][U32_LEN..][PublicKey::LEN..][..BlockHash::LEN],
            state_hash.0.as_bytes()
        );
        assert_eq!(
            &key[U64_LEN..][U32_LEN..][PublicKey::LEN..][BlockHash::LEN..],
            &index.to_be_bytes()
        );
    }

    #[test]
    fn test_snark_prover_sort_key() {
        let index = 25;
        let block_height = 50;
        let pk = PublicKey::default();
        let key = snark_prover_sort_key(&pk, block_height, index);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(
            &key[PublicKey::LEN..][..U32_LEN],
            &block_height.to_be_bytes()
        );
        assert_eq!(&key[PublicKey::LEN..][U32_LEN..], &index.to_be_bytes());
    }
}
