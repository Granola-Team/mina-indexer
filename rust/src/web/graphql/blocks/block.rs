//! GraphQL block representation

use super::{millis_to_iso_date_string, MAINNET_EPOCH_SLOT_COUNT, PK};
use crate::{
    block::precomputed::PrecomputedBlock,
    command::{
        internal::{store::InternalCommandStore, DbInternalCommandWithData},
        signed::SignedCommandWithData,
        store::UserCommandStore,
    },
    ledger::coinbase::Coinbase,
    snark_work::{store::SnarkStore, SnarkWorkSummary},
    store::IndexerStore,
    web::graphql::{
        get_block_canonicity,
        pk::{CoinbaseReceiverPK, CreatorPK, ProverPK, RecipientPK},
        transactions::TransactionWithoutBlock,
    },
};
use async_graphql::{self, Enum, SimpleObject};
use serde::Serialize;
use std::sync::Arc;

#[derive(Default, SimpleObject, Serialize)]
pub struct Block {
    /// Value canonical
    pub canonical: bool,

    /// Value epoch num blocks
    #[graphql(name = "epoch_num_blocks")]
    pub epoch_num_blocks: u32,

    /// Value epoch num canonical blocks
    #[graphql(name = "epoch_num_canonical_blocks")]
    pub epoch_num_canonical_blocks: u32,

    /// Value epoch num supercharged blocks
    #[graphql(name = "epoch_num_supercharged_blocks")]
    pub epoch_num_supercharged_blocks: u32,

    /// Value total num blocks
    #[graphql(name = "total_num_blocks")]
    pub total_num_blocks: u32,

    /// Value total num supercharged blocks
    #[graphql(name = "total_num_supercharged_blocks")]
    pub total_num_supercharged_blocks: u32,

    /// Value block num snarks
    #[graphql(name = "block_num_snarks")]
    pub block_num_snarks: u32,

    /// Value block num user commands
    #[graphql(name = "block_num_user_commands")]
    pub block_num_user_commands: u32,

    /// Value block num zkapp commands
    #[graphql(name = "block_num_zkapp_commands")]
    pub block_num_zkapp_commands: u32,

    /// Value block num internal commands
    #[graphql(name = "block_num_internal_commands")]
    pub block_num_internal_commands: u32,

    /// Value epoch num slots produced
    #[graphql(name = "epoch_num_slots_produced")]
    pub epoch_num_slots_produced: u32,

    /// Value num unique block producers last n blocks
    #[graphql(name = "num_unique_block_producers_last_n_blocks")]
    pub num_unique_block_producers_last_n_blocks: Option<u32>,

    /// Value block
    #[graphql(flatten)]
    pub block: BlockWithoutCanonicity,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BlockSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,

    #[graphql(name = "GLOBALSLOT_ASC")]
    GlobalSlotAsc,
    #[graphql(name = "GLOBALSLOT_DESC")]
    GlobalSlotDesc,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct BlockWithoutCanonicity {
    /// Value state hash
    pub state_hash: String,

    /// Value block height
    pub block_height: u32,

    /// Value global slot since genesis
    pub global_slot_since_genesis: u32,

    /// Value genesis state hash
    pub genesis_state_hash: String,

    /// The public_key for the winner account
    pub winner_account: PK,

    /// Value date_time as ISO 8601 string
    pub date_time: String,

    /// Value received_time as ISO 8601 string
    pub received_time: String,

    /// The public_key for the creator account
    #[graphql(deprecation = "Use creator instead")]
    pub creator_account: PK,

    /// Value creator public key
    #[graphql(flatten)]
    pub creator: CreatorPK,

    /// The public_key for the coinbase_receiver
    pub coinbase_receiver: PK,

    /// Value protocol state
    pub protocol_state: ProtocolState,

    /// Value transaction fees
    pub tx_fees: String,

    /// Value SNARK fees (MINA)
    pub snark_fees: String,

    /// Value transactions
    pub transactions: Transactions,

    /// Value snark jobs
    pub snark_jobs: Vec<SnarkJob>,
}

#[derive(SimpleObject, Serialize)]
pub struct SnarkJob {
    /// Value block state hash
    pub block_state_hash: String,

    /// Valuable block height
    pub block_height: u32,

    /// Value date time
    pub date_time: String,

    /// Value fee (nanomina)
    pub fee: u64,

    /// Value SNARK prover public key
    #[graphql(flatten)]
    pub prover: ProverPK,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct Transactions {
    /// Value coinbase amount
    pub coinbase: String,

    /// Value coinbase receiver account
    #[graphql(deprecation = "Use coinbase_receiver instead")]
    pub coinbase_receiver_account: PK,

    /// Value coinbase receiver public key
    #[graphql(flatten)]
    pub coinbase_receiver: CoinbaseReceiverPK,

    /// Value block fee transfers
    pub fee_transfer: Vec<BlockFeeTransfer>,

    /// Value block user commands
    pub user_commands: Vec<TransactionWithoutBlock>,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct BlockFeeTransfer {
    /// Value fee (MINA)
    pub fee: String,

    /// Value fee transfer recipient public key
    #[graphql(flatten)]
    pub recipient: RecipientPK,

    #[graphql(name = "type")]
    pub feetransfer_kind: String,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct ConsensusState {
    /// Value total currency
    pub total_currency: u64,

    /// Value block height
    pub blockchain_length: u32,

    /// Value block height
    pub block_height: u32,

    /// Value epoch count
    pub epoch: u32,

    /// Value has ancestors the same checkpoint window
    pub has_ancestor_in_same_checkpoint_window: bool,

    /// Value last VRF output
    pub last_vrf_output: String,

    /// Value minimum window density
    pub min_window_density: u32,

    /// Value current slot
    pub slot: u32,

    /// Value global slot
    pub slot_since_genesis: u32,

    /// Value next epoch data
    pub next_epoch_data: NextEpochData,

    /// Value next epoch data
    pub staking_epoch_data: StakingEpochData,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct StakingEpochDataLedger {
    /// Value staking epoch hash
    pub hash: String,

    /// Value staking epoch total currency (nanomina)
    pub total_currency: u64,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct StakingEpochData {
    /// Value staking epoch seed
    pub seed: String,

    /// Value staking epoch length
    pub epoch_length: u32,

    /// Value staking epoch start checkpoint
    pub start_checkpoint: String,

    /// Value staking epoch lock checkpoint
    pub lock_checkpoint: String,

    /// Value epoch staking ledger
    pub ledger: StakingEpochDataLedger,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct NextEpochDataLedger {
    /// Value next epoch hash
    pub hash: String,

    /// Value next epoch total currency (nanomina)
    pub total_currency: u64,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct NextEpochData {
    /// Value next epoch seed
    pub seed: String,

    /// Value next epoch length
    pub epoch_length: u32,

    /// Value next epoch start checkpoint
    pub start_checkpoint: String,

    /// Value next epoch lock checkpoint
    pub lock_checkpoint: String,

    /// Value next epoch staking ledger
    pub ledger: NextEpochDataLedger,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct BlockchainState {
    /// Value utc_date as numeric string
    pub utc_date: String,

    /// Value date as numeric string
    pub date: String,

    /// Value snarked ledger hash
    pub snarked_ledger_hash: String,

    /// Value staged ledger hash
    pub staged_ledger_hash: String,
}

#[derive(Default, SimpleObject, Serialize)]
pub struct ProtocolState {
    /// Value parent state hash
    pub previous_state_hash: String,

    /// Value blockchain state
    pub blockchain_state: BlockchainState,

    /// Value consensus state
    pub consensus_state: ConsensusState,
}

///////////
// impls //
///////////

impl Block {
    pub fn from_precomputed(
        db: &Arc<IndexerStore>,
        block: &PrecomputedBlock,
        counts: [u32; 15],
    ) -> Self {
        let state_hash = block.state_hash();
        let canonical = get_block_canonicity(db, &state_hash);

        let epoch_num_blocks = counts[0];
        let epoch_num_canonical_blocks = counts[1];
        let epoch_num_supercharged_blocks = counts[2];

        let total_num_blocks = counts[3];
        let _total_num_canonical_blocks = counts[4];
        let total_num_supercharged_blocks = counts[5];

        let epoch_num_user_commands = counts[8];
        let total_num_user_commands = counts[9];

        let block_num_snarks = db
            .get_block_snarks_count(&state_hash)
            .expect("snark counts")
            .unwrap_or_default();

        let block_num_user_commands = db
            .get_block_user_commands_count(&state_hash)
            .expect("user command counts")
            .unwrap_or_default();

        let block_num_zkapp_commands = db
            .get_block_zkapp_commands_count(&state_hash)
            .expect("zkapp command counts")
            .unwrap_or_default();

        let block_num_internal_commands = db
            .get_block_internal_commands_count(&state_hash)
            .expect("internal command counts")
            .unwrap_or_default();

        let epoch_num_slots_produced = counts[12];

        // zkapp command counts
        let epoch_num_zkapp_commands = counts[13];
        let total_num_zkapp_commands = counts[14];

        let num_commands = [
            epoch_num_user_commands,
            total_num_user_commands,
            epoch_num_zkapp_commands,
            total_num_zkapp_commands,
        ];

        Self {
            canonical,
            epoch_num_blocks,
            epoch_num_canonical_blocks,
            epoch_num_supercharged_blocks,
            total_num_blocks,
            total_num_supercharged_blocks,
            block_num_snarks,
            block_num_user_commands,
            block_num_zkapp_commands,
            block_num_internal_commands,
            block: BlockWithoutCanonicity::new(db, block, canonical, num_commands),
            epoch_num_slots_produced,
            num_unique_block_producers_last_n_blocks: None,
        }
    }
}

impl BlockWithoutCanonicity {
    pub fn new(
        db: &Arc<IndexerStore>,
        block: &PrecomputedBlock,
        canonical: bool,
        num_commands: [u32; 4],
    ) -> Self {
        let scheduled_time = block.scheduled_time();
        let date_time = millis_to_iso_date_string(block.timestamp() as i64);
        let utc_date = block.timestamp().to_string();
        let received_time = millis_to_iso_date_string(scheduled_time.parse::<i64>().unwrap());

        let state_hash = block.state_hash().0;
        let previous_state_hash = block.previous_state_hash().0;

        let tx_fees = block.tx_fees();
        let snark_fees = block.snark_fees();

        let creator = block.block_creator();

        // blockchain state
        let snarked_ledger_hash = block.snarked_ledger_hash().0;
        let staged_ledger_hash = block.staged_ledger_hash().0;

        // consensus state
        let total_currency = block.total_currency();
        let blockchain_length = block.blockchain_length();
        let block_height = blockchain_length;
        let epoch = block.epoch_count();
        let has_ancestor_in_same_checkpoint_window = block.has_ancestor_in_same_checkpoint_window();
        let last_vrf_output = block.last_vrf_output();
        let min_window_density = block.min_window_density();
        let slot_since_genesis = block.global_slot_since_genesis();
        let slot = slot_since_genesis % MAINNET_EPOCH_SLOT_COUNT;

        // next epoch data
        let next_epoch_seed = block.next_epoch_seed();
        let next_epoch_length = block.next_epoch_length();
        let next_epoch_start_checkpoint = block.next_epoch_start_checkpoint().0;
        let next_epoch_lock_checkpoint = block.next_epoch_lock_checkpoint().0;
        let next_epoch_ledger_hash = block.next_epoch_ledger_hash().0;
        let next_epoch_total_currency = block.next_epoch_total_currency();

        // staking epoch data
        let staking_epoch_seed = block.staking_epoch_seed();
        let staking_epoch_length = block.staking_epoch_length();
        let staking_epoch_start_checkpoint = block.staking_epoch_start_checkpoint().0;
        let staking_epoch_lock_checkpoint = block.staking_epoch_lock_checkpoint().0;
        let staking_epoch_ledger_hash = block.staking_epoch_ledger_hash().0;
        let staking_epoch_total_currency = block.staking_epoch_total_currency();

        // internal commands
        let coinbase = Coinbase::from_precomputed(block);
        let fee_transfers: Vec<_> = DbInternalCommandWithData::from_precomputed(block)
            .into_iter()
            .filter(|x| matches!(x, DbInternalCommandWithData::FeeTransfer { .. }))
            .map(|ft| BlockFeeTransfer::new(db, ft))
            .collect();

        // user commands
        let user_commands: Vec<_> = SignedCommandWithData::from_precomputed(block)
            .into_iter()
            .map(|cmd| TransactionWithoutBlock::new(db, cmd, canonical, num_commands))
            .collect();

        // SNARKs
        let snark_jobs: Vec<_> = SnarkWorkSummary::from_precomputed(block)
            .into_iter()
            .map(|snark| {
                SnarkJob::new(
                    db,
                    snark,
                    state_hash.clone(),
                    block_height,
                    date_time.clone(),
                )
            })
            .collect();

        Self {
            date_time,
            snark_jobs,
            state_hash,
            block_height: block.blockchain_length(),
            global_slot_since_genesis: block.global_slot_since_genesis(),
            genesis_state_hash: block.genesis_state_hash().0,
            coinbase_receiver: PK::new(db, block.coinbase_receiver()),
            winner_account: PK::new(db, block.block_stake_winner()),
            creator: CreatorPK::new(db, creator.clone()),
            creator_account: PK::new(db, creator),
            received_time,
            protocol_state: ProtocolState {
                previous_state_hash,
                blockchain_state: BlockchainState {
                    date: utc_date.clone(),
                    utc_date,
                    snarked_ledger_hash,
                    staged_ledger_hash,
                },
                consensus_state: ConsensusState {
                    total_currency,
                    blockchain_length,
                    block_height,
                    epoch,
                    has_ancestor_in_same_checkpoint_window,
                    last_vrf_output,
                    min_window_density,
                    slot,
                    slot_since_genesis,
                    next_epoch_data: NextEpochData {
                        seed: next_epoch_seed,
                        epoch_length: next_epoch_length,
                        start_checkpoint: next_epoch_start_checkpoint,
                        lock_checkpoint: next_epoch_lock_checkpoint,
                        ledger: NextEpochDataLedger {
                            hash: next_epoch_ledger_hash,
                            total_currency: next_epoch_total_currency,
                        },
                    },
                    staking_epoch_data: StakingEpochData {
                        seed: staking_epoch_seed,
                        epoch_length: staking_epoch_length,
                        start_checkpoint: staking_epoch_start_checkpoint,
                        lock_checkpoint: staking_epoch_lock_checkpoint,
                        ledger: StakingEpochDataLedger {
                            hash: staking_epoch_ledger_hash,
                            total_currency: staking_epoch_total_currency,
                        },
                    },
                },
            },
            tx_fees: tx_fees.to_string(),
            snark_fees: snark_fees.to_string(),
            transactions: Transactions {
                coinbase: coinbase.amount().to_string(),
                coinbase_receiver_account: PK::new(db, coinbase.receiver.clone()),
                coinbase_receiver: CoinbaseReceiverPK::new(db, coinbase.receiver),
                fee_transfer: fee_transfers,
                user_commands,
            },
        }
    }
}

/////////////////
// conversions //
/////////////////

impl BlockFeeTransfer {
    fn new(db: &Arc<IndexerStore>, int_cmd: DbInternalCommandWithData) -> Self {
        match int_cmd {
            DbInternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                kind,
                ..
            }
            | DbInternalCommandWithData::Coinbase {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount.to_string(),
                recipient: RecipientPK::new(db, receiver),
                feetransfer_kind: kind.to_string(),
            },
        }
    }
}

impl SnarkJob {
    fn new(
        db: &Arc<IndexerStore>,
        snark: SnarkWorkSummary,
        block_state_hash: String,
        block_height: u32,
        date_time: String,
    ) -> Self {
        Self {
            block_state_hash,
            block_height,
            date_time,
            fee: snark.fee.0,
            prover: ProverPK::new(db, snark.prover),
        }
    }
}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}
