use serde::{Deserialize, Serialize};

/// 32 bytes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ActionState(pub String);

/////////////////
// conversions //
/////////////////

impl<T> From<T> for ActionState
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let action_state = value.into();

        // 32 bytes = 64 hex + 2 prefix chars
        assert!(action_state.starts_with("0x"));
        assert_eq!(action_state.len(), 66);

        Self(action_state)
    }
}

/////////////
// default //
/////////////

impl std::default::Default for ActionState {
    fn default() -> Self {
        Self("0x3772BC5435B957F81F86F752E93F2E29E886AC24580B3D1EC879C1DAD26965F9".to_string())
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for ActionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
