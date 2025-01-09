use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChainHash(pub String);

impl<T> From<T> for ReceiptChainHash
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
