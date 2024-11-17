#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    GenesisBlock,
    PrecomputedBlockPath,
    BerkeleyBlockPath,
    MainnetBlockPath,
    BlockAncestor,
    BerkeleyBlock,
    MainnetBlock,
    NewBlock,
    BlockCanonicityUpdate,
    BestBlock,
    BlockSummary,
    SnarkWorkSummary,
    SnarkCanonicitySummary,
    TransitionFrontier,
    UserCommandSummary,
    UserCommandCanonicityUpdate,
    InternalCommand,
    InternalCommandCanonicityUpdate,
    DoubleEntryTransaction,
    NewAccount,
    BlockConfirmation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
