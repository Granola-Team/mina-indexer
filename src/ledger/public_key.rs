use mina_serialization_types::v1::PublicKeyV1;
use mina_signer::{CompressedPubKey, PubKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PublicKey(PublicKeyV1);

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PublicKey {}

impl PartialOrd for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.to_address().cmp(&other.to_address()))
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_address().cmp(&other.to_address())
    }
}

impl PublicKey {
    pub fn from_v1(v1: PublicKeyV1) -> Self {
        Self(v1)
    }

    pub fn to_address(&self) -> String {
        CompressedPubKey::from(&self.0).into_address()
    }

    pub fn from_address(value: &str) -> anyhow::Result<Self> {
        CompressedPubKey::from_address(value)
            .map(|x| Self(PublicKeyV1::from(x)))
            .map_err(anyhow::Error::from)
    }
}

impl From<&str> for PublicKey {
    fn from(value: &str) -> Self {
        Self::from_address(value).unwrap()
    }
}

impl From<String> for PublicKey {
    fn from(value: String) -> Self {
        Self::from_address(&value).unwrap()
    }
}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.clone().0.inner().inner().x.hash(state);
    }
}

impl From<PublicKey> for String {
    fn from(value: PublicKey) -> Self {
        value.to_address()
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_address())
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_address())
    }
}

impl From<PublicKeyV1> for PublicKey {
    fn from(value: PublicKeyV1) -> Self {
        PublicKey(value)
    }
}

impl From<PublicKey> for PublicKeyV1 {
    fn from(value: PublicKey) -> Self {
        value.0
    }
}

impl From<PublicKey> for PubKey {
    fn from(value: PublicKey) -> Self {
        PubKey::from_address(&CompressedPubKey::from(&value.0).into_address()).unwrap()
    }
}

pub fn is_valid(pk: &str) -> bool {
    pk.starts_with("B62q") && pk.len() == 55
}

#[cfg(test)]
mod test {
    use super::PublicKey;

    #[test]
    fn parse_public_keys() {
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
            assert_eq!(PublicKey::from_address(pk).unwrap().to_address(), pk);
        }
    }
}
