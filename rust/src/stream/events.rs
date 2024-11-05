#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    PrecomputedBlockPath,
    BerkeleyBlockPath,
    MainnetBlockPath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
