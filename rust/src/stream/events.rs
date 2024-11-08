#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    PrecomputedBlockPath,
    BerkeleyBlockPath,
    MainnetBlockPath,
    BlockAncestor,
    BerkeleyBlock,
    MainnetBlock,
    BlockAddedToTree,
    BlockCanonicityUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
