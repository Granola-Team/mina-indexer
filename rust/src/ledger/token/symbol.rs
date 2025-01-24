use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize)]
pub struct TokenSymbol(pub String);

impl TokenSymbol {
    pub fn new<S>(symbol: S) -> Self
    where
        S: Into<String>,
    {
        Self(symbol.into())
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for TokenSymbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::mina_blocks::common::from_str(deserializer)
    }
}

impl std::default::Default for TokenSymbol {
    /// MINA token symbol
    fn default() -> Self {
        Self::new("MINA")
    }
}

impl std::str::FromStr for TokenSymbol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl<T> From<T> for TokenSymbol
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for TokenSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
