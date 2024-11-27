#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreviousStateHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateHashPreviousStateHash {
    pub state_hash: String,
    pub previous_state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Height(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LastVrfOutput(pub String);
