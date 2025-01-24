use crate::{constants::MINA_SCALE_DEC, utility::functions::nanomina_to_mina};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
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

/////////////
// options //
/////////////

/// Deserialize from `Option<&str>`
pub(crate) fn from_str_opt<'de, T, D>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    Ok(<Option<String>>::deserialize(de)?.and_then(|x| x.parse().ok()))
}

/// Serialize to `Option<String>`
pub(crate) fn to_str_opt<T, S>(value: &Option<T>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    match value {
        Some(s) => {
            let s = s.to_string();
            ser.serialize_some(&s)
        }
        None => ser.serialize_none(),
    }
}

/////////////
// vectors //
/////////////

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

pub(crate) fn vec_to_str<T, S>(value: &[T], ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    let strings: Vec<_> = value.iter().map(ToString::to_string).collect();
    let mut seq = ser.serialize_seq(Some(strings.len()))?;

    for s in strings {
        seq.serialize_element(&s)?;
    }

    seq.end()
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
