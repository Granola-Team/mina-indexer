use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSymbol(pub String);

impl TokenSymbol {
    pub fn new<S>(symbol: S) -> Self
    where
        S: Into<String>,
    {
        Self(symbol.into())
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

impl std::fmt::Display for TokenSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
