use strum_macros::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum EventType {
    AccountLogBalanceDelta,
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
    CanonicalMainnetBlock,
    CanonicalBerkeleyBlock,
    SnarkWorkSummary,
    SnarkCanonicitySummary,
    BulkSnarkCanonicity,
    TransitionFrontier,
    UserCommandLog,
    ZkAppCommandLog,
    CanonicalBatchZkappCommandLog,
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
    PreForkStakingLedgerFilePath,
    PostForkStakingLedgerFilePath,
    StakingLedgerEntry,
    Username,
    Test,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
