#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    GenesisBlock,
    PrecomputedBlockPath,
    BerkeleyBlockPath,
    MainnetBlockPath,
    BlockAncestor,
    BerkeleyBlock,
    MainnetBlock,
    BlockAddedToTree,
    BlockCanonicityUpdate,
    BestBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
