use serde::{Deserialize, Serialize};

/// 32 bytes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEvent(pub String);

// conversions

impl<T> From<T> for ZkappEvent
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
