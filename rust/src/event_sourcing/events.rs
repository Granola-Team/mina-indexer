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
    CanonicalUserCommandLog,
    InternalCommandLog,
    CanonicalInternalCommandLog,
    DoubleEntryTransaction,
    NewAccount,
    BlockConfirmation,
    PreExistingAccount,
    ActorHeight,
    EpochStakeDelegation,
    HeightSpread,
    StakingLedgerFilePath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
