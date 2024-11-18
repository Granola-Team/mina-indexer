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
    BlockLog,
    CanonicalBlockLog,
    SnarkWorkSummary,
    SnarkCanonicitySummary,
    TransitionFrontier,
    UserCommandLog,
    UserCommandCanonicityUpdate,
    InternalCommand,
    InternalCommandCanonicityUpdate,
    DoubleEntryTransaction,
    NewAccount,
    BlockConfirmation,
    PreExistingAccount,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
