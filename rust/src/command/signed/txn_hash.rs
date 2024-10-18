use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum TxnHash {
    V1(String),
    V2(String),
}

impl TxnHash {
    // v1 pre-hardfork
    pub const V1_LEN: usize = 53;
    pub const V1_PREFIX: &'static str = "Ckp";

    // v2 post-hardfork
    pub const V2_LEN: usize = 52;
    pub const V2_PREFIX: &'static str = "5J";

    pub fn inner(self) -> String {
        match self {
            Self::V1(v1) => v1,
            Self::V2(v2) => v2,
        }
    }

    pub fn ref_inner(&self) -> &String {
        match self {
            Self::V1(v1) => v1,
            Self::V2(v2) => v2,
        }
    }

    pub fn new(txn_hash: String) -> anyhow::Result<Self> {
        if Self::is_valid_v1(&txn_hash) {
            return Ok(Self::V1(txn_hash.to_string()));
        }

        if Self::is_valid_v2(&txn_hash) {
            return Ok(Self::V2(txn_hash.to_string()));
        }

        bail!("Invalid txn hash {txn_hash}")
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::V1(hash) => Self::is_valid_v1(hash),
            Self::V2(hash) => Self::is_valid_v2(hash),
        }
    }

    pub fn is_valid_v1(txn_hash: &str) -> bool {
        txn_hash.starts_with(TxnHash::V1_PREFIX) && txn_hash.len() == TxnHash::V1_LEN
    }

    pub fn is_valid_v2(txn_hash: &str) -> bool {
        txn_hash.starts_with(TxnHash::V2_PREFIX) && txn_hash.len() == TxnHash::V2_LEN
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        // v2
        if bytes.starts_with(Self::V2_PREFIX.as_bytes()) {
            let mut bytes = bytes;
            bytes.remove(Self::V2_LEN);
            return Self::new(String::from_utf8(bytes)?);
        }

        // v1 or fail
        Self::new(String::from_utf8(bytes)?)
    }

    /// Right-pads v2 txn hash to match v1 length
    pub fn right_pad_v2(&self) -> [u8; Self::V1_LEN] {
        let mut bytes = [0; Self::V1_LEN];

        match self {
            Self::V1(_) => {
                bytes.copy_from_slice(self.ref_inner().as_bytes());
                bytes
            }
            Self::V2(_) => {
                bytes[..Self::V2_LEN].copy_from_slice(self.ref_inner().as_bytes());
                bytes[Self::V2_LEN] = 0;
                bytes
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const V1_TXN_HASH: &str = "CkpBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUS";
    const V2_TXN_HASH: &str = "5JBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUS";

    #[test]
    fn round_trip_padding() -> anyhow::Result<()> {
        // v1
        let v1 = TxnHash::new(V1_TXN_HASH.into())?;
        let v1_padded_bytes = v1.right_pad_v2().to_vec();
        assert_eq!(v1, TxnHash::from_bytes(v1_padded_bytes)?);

        // v2
        let v2 = TxnHash::new(V2_TXN_HASH.into())?;
        let v2_padded_bytes = v2.right_pad_v2().to_vec();
        assert_eq!(v2, TxnHash::from_bytes(v2_padded_bytes)?);

        Ok(())
    }

    #[test]
    fn right_pad_txn_hashes() -> anyhow::Result<()> {
        // v1 - no right padding
        let v1 = TxnHash::new(V1_TXN_HASH.into())?;
        assert!(matches!(v1, TxnHash::V1(_)));
        assert_eq!(&v1.right_pad_v2(), v1.to_string().as_bytes());

        // v2 - single 0 byte right padding
        let v2 = TxnHash::new(V2_TXN_HASH.into())?;
        assert!(matches!(v2, TxnHash::V2(_)));

        let mut v2_right_pad = [0; TxnHash::V1_LEN];
        v2_right_pad[..TxnHash::V1_LEN - 1].copy_from_slice(v2.to_string().as_bytes());
        *v2_right_pad.last_mut().unwrap() = 0;

        assert_eq!(v2.right_pad_v2(), v2_right_pad);

        Ok(())
    }
}
