use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(untagged)]
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

    pub fn new<S: Into<String>>(txn_hash: S) -> anyhow::Result<Self> {
        let txn_hash: String = txn_hash.into();

        if Self::is_valid_v1(&txn_hash) {
            return Ok(Self::V1(txn_hash));
        }

        if Self::is_valid_v2(&txn_hash) {
            return Ok(Self::V2(txn_hash));
        }

        bail!("Invalid txn hash: '{txn_hash}'")
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

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for TxnHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str::<String, D>(deserializer).map(|txn_hash| {
            if txn_hash.starts_with(Self::V1_PREFIX) {
                Self::V1(txn_hash)
            } else {
                Self::V2(txn_hash)
            }
        })
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for TxnHash {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        if bool::arbitrary(g) {
            Self::arbitrary_v1(g)
        } else {
            Self::arbitrary_v2(g)
        }
    }
}

#[cfg(test)]
impl TxnHash {
    pub fn arbitrary_v1(g: &mut quickcheck::Gen) -> Self {
        use quickcheck::Arbitrary;

        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..TxnHash::V1_LEN - TxnHash::V1_PREFIX.len() {
            let idx = usize::arbitrary(g) % alphabet.len();

            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self::V1(Self::V1_PREFIX.to_string() + &chars.iter().collect::<String>())
    }

    pub fn arbitrary_v2(g: &mut quickcheck::Gen) -> Self {
        use quickcheck::Arbitrary;

        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..TxnHash::V2_LEN - TxnHash::V2_PREFIX.len() {
            let idx = usize::arbitrary(g) % alphabet.len();

            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self::V2(Self::V2_PREFIX.to_string() + &chars.iter().collect::<String>())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::Gen;

    const V1_TXN_HASH: &str = "CkpBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUS";
    const V2_TXN_HASH: &str = "5JBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUSBOGUS";

    #[test]
    fn arbitrary_is_valid() -> anyhow::Result<()> {
        let g = &mut Gen::new(1000);

        for _ in 0..100 {
            let v1 = TxnHash::arbitrary_v1(g);
            assert!(v1.is_valid());

            let v2 = TxnHash::arbitrary_v2(g);
            assert!(v2.is_valid());
        }

        Ok(())
    }

    #[test]
    fn round_trip_padding() -> anyhow::Result<()> {
        // v1
        let v1 = TxnHash::new(V1_TXN_HASH)?;
        let v1_padded_bytes = v1.right_pad_v2().to_vec();
        assert_eq!(v1, TxnHash::from_bytes(v1_padded_bytes)?);

        // v2
        let v2 = TxnHash::new(V2_TXN_HASH)?;
        let v2_padded_bytes = v2.right_pad_v2().to_vec();
        assert_eq!(v2, TxnHash::from_bytes(v2_padded_bytes)?);

        Ok(())
    }

    #[test]
    fn right_pad_txn_hashes() -> anyhow::Result<()> {
        // v1 - no right padding
        let v1 = TxnHash::new(V1_TXN_HASH)?;
        assert!(matches!(v1, TxnHash::V1(_)));
        assert_eq!(&v1.right_pad_v2(), v1.to_string().as_bytes());

        // v2 - single 0 byte right padding
        let v2 = TxnHash::new(V2_TXN_HASH)?;
        assert!(matches!(v2, TxnHash::V2(_)));

        let mut v2_right_pad = [0; TxnHash::V1_LEN];
        v2_right_pad[..TxnHash::V1_LEN - 1].copy_from_slice(v2.to_string().as_bytes());
        *v2_right_pad.last_mut().unwrap() = 0;

        assert_eq!(v2.right_pad_v2(), v2_right_pad);

        Ok(())
    }
}
