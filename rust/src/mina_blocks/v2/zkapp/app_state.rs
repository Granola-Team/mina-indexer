use serde::{Deserialize, Serialize};

/// 32 bytes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct AppState(pub String);

/////////////////
// conversions //
/////////////////

impl<T> From<T> for AppState
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let app_state: String = value.into();

        // 32 bytes = 64 hex + 2 prefix chars
        assert!(app_state.starts_with("0x"));
        assert_eq!(app_state.len(), 66);

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
