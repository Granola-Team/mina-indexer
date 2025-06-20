use serde::{Deserialize, Serialize};

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize)]
pub struct TokenSymbol(pub String);

//////////
// impl //
//////////

impl TokenSymbol {
    pub fn new<S>(symbol: S) -> Self
    where
        S: Into<String>,
    {
        Self(symbol.into())
    }

    /// `MINA` token symbol
    pub fn mina() -> Self {
        Self("MINA".to_string())
    }
}

///////////
// check //
///////////

impl crate::base::check::Check for Option<TokenSymbol> {
    fn check(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Some(self_symbol), Some(symbol)) => {
                let check = self_symbol != symbol;
                if check {
                    log::error!("Mismatching token symbols {:?} {:?}", self, other);
                }

                check
            }
            (Some(symbol), _) | (_, Some(symbol)) => {
                let check = !symbol.0.is_empty();
                if check {
                    log::error!("Mismatching token symbol {}", symbol);
                }

                check
            }
            _ => true,
        }
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
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////////
// conversions //
/////////////////

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

/////////////
// display //
/////////////

impl std::fmt::Display for TokenSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for TokenSymbol {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let length = u8::arbitrary(g) % 10;
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        let mut chars = vec![];
        for _ in 0..length {
            let idx = usize::arbitrary(g) % alphabet.len();
            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(chars.iter().collect())
    }
}
