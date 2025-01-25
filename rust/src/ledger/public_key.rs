use crate::{
    proof_systems::signer::pubkey::{CompressedPubKey, PubKey},
    protocol::serialization_types::signatures::{PublicKey2V1, PublicKeyV1},
};
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicKey(pub String);

impl PublicKey {
    pub const LEN: usize = 55;
    pub const PREFIX: &'static str = "B62q";

    pub fn new<S: Into<String>>(key: S) -> Self {
        Self(key.into())
    }

    pub fn to_address(&self) -> String {
        self.0.to_owned()
    }

    /// Convert to bytes (length [Self::LEN])
    pub fn to_bytes(self) -> [u8; Self::LEN] {
        let mut res = [0u8; PublicKey::LEN];
        res.copy_from_slice(self.0.as_bytes());
        res
    }

    /// Convert from bytes (length [Self::LEN])
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let res = String::from_utf8(bytes.to_vec())?;

        if Self::is_valid(&res) {
            return Ok(Self(res));
        }

        bail!("Invalid public key from bytes")
    }

    /// Checks length & prefix
    pub fn is_valid(pk: &str) -> bool {
        pk.starts_with(Self::PREFIX) && pk.len() == Self::LEN
    }

    /// [PublicKey] upper bound
    pub fn upper_bound() -> Self {
        Self("B62qZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ".into())
    }

    /// [PublicKey] lower bound
    pub fn lower_bound() -> Self {
        Self("B62q000000000000000000000000000000000000000000000000000".into())
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::mina_blocks::common::from_str(deserializer)
    }
}

/////////////
// default //
/////////////

impl std::default::Default for PublicKey {
    fn default() -> Self {
        Self("B62qDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTPK".into())
    }
}

/////////////////
// conversions //
/////////////////

impl std::str::FromStr for PublicKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid public key: {}", s)
        }
    }
}

impl From<&str> for PublicKey {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for PublicKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<PublicKey> for String {
    fn from(value: PublicKey) -> Self {
        value.0
    }
}

impl From<PublicKeyV1> for PublicKey {
    fn from(v1: PublicKeyV1) -> Self {
        let pk = CompressedPubKey::from(&v1.0.inner().inner());
        Self(pk.into_address())
    }
}

impl From<PublicKey2V1> for PublicKey {
    fn from(v1: PublicKey2V1) -> Self {
        v1.0.t.into()
    }
}

impl From<PublicKey> for PublicKeyV1 {
    fn from(value: PublicKey) -> Self {
        let pk = CompressedPubKey::from_address(&value.0).unwrap();
        pk.into()
    }
}

impl From<PublicKey> for PublicKey2V1 {
    fn from(value: PublicKey) -> Self {
        let pk = CompressedPubKey::from_address(&value.0).unwrap();
        pk.into()
    }
}

impl From<PublicKey> for PubKey {
    fn from(value: PublicKey) -> Self {
        PubKey::from_address(&value.0).unwrap()
    }
}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::PublicKey;

    #[test]
    fn fixed_pks_are_valid() {
        assert!(PublicKey::is_valid(&PublicKey::default().0));
        assert!(PublicKey::is_valid(&PublicKey::lower_bound().0));
        assert!(PublicKey::is_valid(&PublicKey::upper_bound().0));
    }

    #[test]
    fn parse_public_keys() -> anyhow::Result<()> {
        // public keys from
        // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
        let pks = [
            "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
            "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
            "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
            "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
            "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
            "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
        ];
        for pk in pks {
            let bytes = pk.as_bytes();

            assert_eq!(PublicKey(pk.to_owned()).to_bytes(), bytes, "to_bytes");
            assert_eq!(
                PublicKey(pk.to_owned()),
                PublicKey::from_bytes(bytes)?,
                "from_bytes"
            );
        }
        Ok(())
    }

    #[test]
    fn serde() -> anyhow::Result<()> {
        let pk = PublicKey::default();

        // roundtrip
        let bytes = serde_json::to_vec(&pk)?;
        let res = serde_json::from_slice::<PublicKey>(&bytes)?;

        assert_eq!(pk, res);

        // matches string deserialization
        let str = pk.0.clone();
        let str_bytes = serde_json::to_vec(&str)?;

        assert_eq!(bytes, str_bytes);

        Ok(())
    }
}
