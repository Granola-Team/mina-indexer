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
    BulkSnarkCanonicity,
    TransitionFrontier,
    UserCommandLog,
    CanonicalUserCommandLog,
    BatchCanonicalUserCommandLog,
    InternalCommandLog,
    CanonicalInternalCommandLog,
    DoubleEntryTransaction,
    NewAccount,
    BlockConfirmation,
    PreExistingAccount,
    ActorHeight,
    EpochStakeDelegation,
    RunningAvgHeightSpread,
    StakingLedgerFilePath,
    StakingLedgerEntry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
