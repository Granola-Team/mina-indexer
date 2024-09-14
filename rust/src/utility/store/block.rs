use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::public_key::PublicKey,
    utility::store::U32_LEN,
};

/// `{block height BE}{state hash}`
pub fn block_height_key(block: &PrecomputedBlock) -> [u8; U32_LEN + BlockHash::LEN] {
    let mut key = [0; U32_LEN + BlockHash::LEN];
    key[..U32_LEN].copy_from_slice(&block.blockchain_length().to_be_bytes());
    key[U32_LEN..].copy_from_slice(block.state_hash().0.as_bytes());
    key
}

/// `{global slot BE}{state hash}`
pub fn block_global_slot_key(block: &PrecomputedBlock) -> [u8; U32_LEN + BlockHash::LEN] {
    let mut key = [0; U32_LEN + BlockHash::LEN];
    key[..U32_LEN].copy_from_slice(&block.global_slot_since_genesis().to_be_bytes());
    key[U32_LEN..].copy_from_slice(block.state_hash().0.as_bytes());
    key
}

/// Key format
/// ```
/// {pk}{sort_value}{state_hash}
/// where
/// - pk:         [PublicKey] bytes
/// - sort_value: u32 BE bytes
/// - state_hash: [BlockHash] bytes
pub fn pk_block_sort_key(
    pk: &PublicKey,
    sort_value: u32,
    state_hash: &BlockHash,
) -> [u8; PublicKey::LEN + U32_LEN + BlockHash::LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN + BlockHash::LEN];
    key[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[PublicKey::LEN..][..U32_LEN].copy_from_slice(&sort_value.to_be_bytes());
    key[PublicKey::LEN..][U32_LEN..][..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key
}

#[cfg(test)]
mod block_store_impl_tests {
    use super::*;
    use crate::block::precomputed::PcbVersion;
    use std::path::PathBuf;

    #[test]
    fn test_block_height_key() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let key = block_height_key(&block);

        assert_eq!(
            block.state_hash().0.as_bytes(),
            "3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh".as_bytes()
        );
        assert_eq!(&key[..U32_LEN], &105489u32.to_be_bytes());
        assert_eq!(&key[U32_LEN..], block.state_hash().0.as_bytes());
        Ok(())
    }

    #[test]
    fn test_block_global_slot_key() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let result = block_global_slot_key(&block);

        assert_eq!(
            &result[..U32_LEN],
            &block.global_slot_since_genesis().to_be_bytes()
        );
        assert_eq!(&result[U32_LEN..], block.state_hash().0.as_bytes());
        Ok(())
    }

    #[test]
    fn test_pk_block_sort_key() {
        let sort_value = 500;
        let pk = PublicKey::default();
        let state_hash = BlockHash::default();
        let key = pk_block_sort_key(&pk, sort_value, &state_hash);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(&key[PublicKey::LEN..][..U32_LEN], &sort_value.to_be_bytes());
        assert_eq!(&key[PublicKey::LEN..][U32_LEN..], state_hash.0.as_bytes());
    }
}
