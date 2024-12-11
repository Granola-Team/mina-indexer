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

pub(crate) fn from_keep_or_ignore<'de, D>(de: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    // keep/ignore
    match <(String,)>::deserialize(de) {
        Ok(s) => {
            if s.0 == "Keep" || s.0 == "Ignore" {
                return Ok(());
            }

            panic!("invalid keep or ignore: {}", s.0)
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn from_set_or_check<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Display,
{
    // set/check
    match <(String, T)>::deserialize(de) {
        Ok(s) => {
            if s.0 == "Set" || s.0 == "Check" {
                return Ok(s.1);
            }

            panic!("invalid set or check: {}", s.0)
        }
        Err(e) => Err(e),
    }
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
