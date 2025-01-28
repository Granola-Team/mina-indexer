//! Common (de)serialization functions

use crate::{constants::MINA_SCALE_DEC, utility::functions::nanomina_to_mina};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;

/////////////
// strings //
/////////////

/// Deserialize from `str`
pub fn from_str<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    String::deserialize(de)?
        .parse()
        .map_err(serde::de::Error::custom)
}

/// Serialize to `str`
pub(crate) fn to_str<T, S>(value: T, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    let s = value.to_string();
    ser.serialize_str(&s)
}

//////////////
// nanomina //
//////////////

pub fn from_nanomina_str<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(de)?.parse::<Decimal>() {
        Ok(res) => Ok((res * MINA_SCALE_DEC).to_u64().unwrap()),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}

pub(crate) fn to_nanomina_str<S>(nanomina: &u64, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = nanomina_to_mina(*nanomina);
    ser.serialize_str(&s)
}
