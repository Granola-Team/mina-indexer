use crate::{
    block::BlockHash,
    ledger::public_key::PublicKey,
    utility::store::{U32_LEN, U64_LEN},
};

/// Key format
/// ```
/// {fee}{slot}{pk}{hash}{num}
/// where
/// fee:   u64 BE bytes
/// slot:  u32 BE bytes
/// pk:    [PublicKey] bytes
/// hash:  [BlockHash] bytes
/// index: u32 BE bytes
pub fn snark_fee_prefix_key(
    fee: u64,
    global_slot: u32,
    pk: &PublicKey,
    state_hash: &BlockHash,
    index: u32,
) -> [u8; U64_LEN + U32_LEN + PublicKey::LEN + BlockHash::LEN + U32_LEN] {
    let mut key = [0; U64_LEN + U32_LEN + PublicKey::LEN + BlockHash::LEN + U32_LEN];
    key[..U64_LEN].copy_from_slice(&fee.to_be_bytes());
    key[U64_LEN..][..U32_LEN].copy_from_slice(&global_slot.to_be_bytes());
    key[U64_LEN..][U32_LEN..][..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][..BlockHash::LEN]
        .copy_from_slice(state_hash.0.as_bytes());
    key[U64_LEN..][U32_LEN..][PublicKey::LEN..][BlockHash::LEN..]
        .copy_from_slice(&index.to_be_bytes());
    key
}

/// Key format
/// ```
/// {prover}{slot}{index}
/// where
/// - prover: [PublicKey] bytes
/// - slot:   u32 BE bytes
/// - index:  u32 BE bytes
pub fn snark_prover_prefix_key(
    prover: &PublicKey,
    global_slot: u32,
    index: u32,
) -> [u8; PublicKey::LEN + U32_LEN + U32_LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN + U32_LEN];
    key[..PublicKey::LEN].copy_from_slice(prover.0.as_bytes());
    key[PublicKey::LEN..][..U32_LEN].copy_from_slice(&global_slot.to_be_bytes());
    key[PublicKey::LEN..][U32_LEN..].copy_from_slice(&index.to_be_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snark_fee_prefix_key() {
        let fee = 100;
        let index = 25;
        let global_slot = 50;
        let pk = PublicKey::default();
        let state_hash = BlockHash::default();
        let key = snark_fee_prefix_key(fee, global_slot, &pk, &state_hash, index);

        assert_eq!(&key[..U64_LEN], &fee.to_be_bytes());
        assert_eq!(&key[U64_LEN..][..U32_LEN], &global_slot.to_be_bytes());
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
    fn test_snark_prover_prefix_key() {
        let index = 25;
        let global_slot = 50;
        let pk = PublicKey::default();
        let key = snark_prover_prefix_key(&pk, global_slot, index);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(
            &key[PublicKey::LEN..][..U32_LEN],
            &global_slot.to_be_bytes()
        );
        assert_eq!(&key[PublicKey::LEN..][U32_LEN..], &index.to_be_bytes());
    }
}
