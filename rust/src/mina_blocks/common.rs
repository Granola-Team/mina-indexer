use crate::constants::MINA_SCALE_DEC;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

pub(crate) fn from_str<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    String::deserialize(de)?
        .parse()
        .map_err(serde::de::Error::custom)
}

pub(crate) fn from_str_opt<'de, T, D>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    Ok(<Option<String>>::deserialize(de)?.and_then(|x| x.parse().ok()))
}

pub(crate) fn vec_from_str<'de, T, D>(de: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    Ok(<Vec<String>>::deserialize(de)?
        .iter()
        .map(|x| x.parse().unwrap())
        .collect())
}

pub(crate) fn from_decimal_str<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(de)?.parse::<Decimal>() {
        Ok(res) => Ok((res * MINA_SCALE_DEC).to_u64().unwrap()),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}
