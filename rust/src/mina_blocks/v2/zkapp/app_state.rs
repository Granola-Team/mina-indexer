use serde::{Deserialize, Serialize};

/// 32 bytes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct AppState(pub String);

//////////
// impl //
//////////

impl AppState {
    pub const PREFIX: &'static str = "0x";

    // 32 bytes = 64 hex + 2 prefix chars
    pub const LEN: usize = 66;
}

/////////////////
// conversions //
/////////////////

impl<T> From<T> for AppState
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let app_state: String = value.into();

        assert!(app_state.starts_with(Self::PREFIX));
        assert_eq!(app_state.len(), Self::LEN);

        Self(app_state)
    }
}

/////////////
// default //
/////////////

impl std::default::Default for AppState {
    fn default() -> Self {
        Self("0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for AppState {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut bytes = [0u8; 32];

        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }

        Self(format!("0x{}", hex::encode(bytes)))
    }
}
