//! Receipt chain hash representation

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ReceiptChainHash(pub String);

impl<T> From<T> for ReceiptChainHash
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for ReceiptChainHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}
