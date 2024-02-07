use crate::ledger::public_key::PublicKey;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize_public_key<S>(
    public_key: &PublicKey,
    s: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let pub_key_address = public_key.to_address();
    s.serialize_str(&pub_key_address)
}

pub fn deserialize_public_key<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    PublicKey::from_address(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_public_key_opt<S>(
    public_key: &Option<PublicKey>,
    s: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    if let Some(pub_key_address) = public_key.clone().map(|pk| pk.to_address()) {
        s.serialize_some(&pub_key_address)
    } else {
        s.serialize_none()
    }
}

pub fn deserialize_public_key_opt<'de, D>(deserializer: D) -> Result<Option<PublicKey>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        return Ok(Some(
            PublicKey::from_address(&s).map_err(serde::de::Error::custom)?,
        ));
    }
    Ok(None)
}
