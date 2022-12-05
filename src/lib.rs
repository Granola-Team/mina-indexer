use serde::{Serialize, Deserialize};

pub mod constants;
pub mod websocket;
pub mod queries;
pub mod subchain;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Block {
    #[serde(rename = "creatorAccount")]
    creator_account: Account,
    #[serde(rename = "winnerAccount")]
    winner_account: Account,
    #[serde(rename = "stateHash")]
    state_hash: String,
    #[serde(rename = "stateHashField")]
    state_hash_field: String,
    #[serde(rename = "protocolState")]
    protocol_state: ProtocolState,
    #[serde(rename = "protocolStateProof")]
    protocol_state_proof: ProtocolStateProof,
    transactions: Transactions,
    #[serde(rename = "snarkJobs")]
    snark_jobs: Vec<SnarkJob>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Account {
    #[serde(rename = "publicKey")]
    public_key: String,
    token: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    #[serde(rename = "previousStateHash")]
    previous_state_hash: String,
    #[serde(rename = "blockchainState")]
    blockchain_state: BlockchainState,
    #[serde(rename = "consensusState")]
    consensus_state: ConsensusState,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct BlockchainState {
    #[serde(rename = "utcDate")]
    utc_date: String,
    #[serde(rename = "snarkedLedgerHash")]
    snarked_ledger_hash: String,
    #[serde(rename = "stagedLedgerHash")]
    staged_ledger_hash: String,
    #[serde(rename = "stagedLedgerProofEmitted")]
    staged_ledger_proof_emitted: bool
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    #[serde(rename = "blockHeight")]
    block_height: String,
    #[serde(rename = "epochCount")]
    epoch_count: String,
    slot: String,
    #[serde(rename = "slotSinceGenesis")]
    slot_since_genesis: String,
    #[serde(rename = "minWindowDensity")]
    min_window_density: String,
    #[serde(rename = "lastVrfOutput")]
    last_vrf_output: String,
    #[serde(rename = "totalCurrency")]
    total_currency: String,
    #[serde(rename = "hasAncestorInSameCheckpointWindow")]
    has_ancestor_in_same_checkpoint_window: bool,
    #[serde(rename = "stakingEpochData")]
    staking_epoch_data: EpochData,
    #[serde(rename = "nextEpochData")]
    next_epoch_data: EpochData,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct EpochData {
    seed: String,
    #[serde(rename = "startCheckpoint")]
    start_checkpoint: String,
    #[serde(rename = "lockCheckpoint")]
    lock_checkpoint: String,
    #[serde(rename = "epochLength")]
    epoch_length: String,
    ledger: LedgerState,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct LedgerState {
    hash: String,
    #[serde(rename = "totalCurrency")]
    total_currency: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct ProtocolStateProof {
    base64: String
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Transactions {
    #[serde(rename = "feeTransfer")]
    fee_transfer: Vec<FeeTransfer>,
    coinbase: String,
    #[serde(rename = "coinbaseReceiverAccount")]
    coinbase_reciever_account: Account,
    #[serde(rename = "userCommands")]
    user_commands: Vec<UserCommand>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct FeeTransfer {
    #[serde(rename = "type")]
    __type: String,
    recipient: String,
    fee: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct UserCommand {
    __typename: String,
    source: Account,
    receiver: Account,
    kind: String,
    id: String,
    memo: String,
    amount: String
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct SnarkJob {
    prover: String,
    fee: String,
    #[serde(rename = "workIds")]
    work_ids: Vec<u64>,
}