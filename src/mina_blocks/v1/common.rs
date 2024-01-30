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
