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
    DoubleEntryTxn,
    SnarkWorkSummary,
    SnarkCanonicitySummary,
    TransitionFrontier,
    UserCommandSummary,
    UserCommandCanonicityUpdate,
    CoinbaseTransfer,
    FeeTransferViaCoinbase,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}
