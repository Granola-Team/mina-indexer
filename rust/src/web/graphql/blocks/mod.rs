use super::{
    db, gen::BlockProtocolStateConsensusStateQueryInput, get_block_canonicity,
    millis_to_iso_date_string, transactions::Transaction, MAINNET_COINBASE_REWARD, PK,
};
use crate::{
    block::{is_valid_state_hash, precomputed::PrecomputedBlock, store::BlockStore},
    command::{
        internal::{InternalCommand, InternalCommandWithData},
        signed::SignedCommandWithData,
    },
    ledger::LedgerHash,
    proof_systems::signer::pubkey::CompressedPubKey,
    protocol::serialization_types::{
        common::Base58EncodableVersionedType, staged_ledger_diff::TransactionStatusFailedType,
        version_bytes,
    },
    snark_work::SnarkWorkSummary,
    store::{
        blocks_global_slot_idx_iterator, blocks_global_slot_idx_state_hash_from_key, IndexerStore,
    },
    web::graphql::gen::BlockQueryInput,
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(Default)]
pub struct BlocksQueryRoot;

#[Object]
impl BlocksQueryRoot {
    async fn block<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<BlockWithCanonicity>> {
        let db = db(ctx);
        let total_num_blocks = db.get_block_production_total_count()?;

        // no query filters => get the best block
        if query.is_none() {
            return Ok(db.get_best_block().map(|b| {
                b.map(|pcb| {
                    let canonical = get_block_canonicity(db, &pcb.state_hash().0);
                    BlockWithCanonicity {
                        canonical,
                        total_num_blocks,
                        block: Block::new(pcb, canonical),
                    }
                })
            })?);
        }

        // Use constant time access if we have state hash
        if let Some(state_hash) = query.as_ref().and_then(|input| input.state_hash.clone()) {
            if !is_valid_state_hash(&state_hash) {
                return Ok(None);
            }

            let pcb = match db.get_block(&state_hash.clone().into())? {
                Some(pcb) => pcb,
                None => return Ok(None),
            };
            let canonical = get_block_canonicity(db, &state_hash);
            let block = BlockWithCanonicity {
                canonical,
                total_num_blocks,
                block: Block::new(pcb, canonical),
            };

            if query.unwrap().matches(&block) {
                return Ok(Some(block));
            }
            return Ok(None);
        }

        // else iterate from the beginning
        // TODO bound query search space if given any inputs
        for entry in blocks_global_slot_idx_iterator(db, speedb::IteratorMode::Start).flatten() {
            let state_hash = blocks_global_slot_idx_state_hash_from_key(&entry.0)?;
            let pcb = db
                .get_block(&state_hash.clone().into())?
                .expect("block to be returned");
            let canonical = get_block_canonicity(db, &state_hash);
            let block = BlockWithCanonicity {
                canonical,
                total_num_blocks,
                block: Block::new(pcb, canonical),
            };

            if query.as_ref().map_or(true, |q| q.matches(&block)) {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }

    async fn blocks<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
        #[graphql(default = 100)] limit: usize,
        sort_by: Option<BlockSortByInput>,
    ) -> Result<Vec<BlockWithCanonicity>> {
        let db = db(ctx);
        let total_num_blocks = db.get_block_production_total_count()?;

        let mut blocks: Vec<BlockWithCanonicity> = Vec::with_capacity(limit);
        let sort_by = sort_by.unwrap_or(BlockSortByInput::BlockHeightDesc);

        // state hash query
        if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
            let block = db.get_block(&state_hash.clone().into())?;
            return Ok(block
                .into_iter()
                .filter_map(|b| precomputed_matches_query(db, &query, b, total_num_blocks))
                .collect());
        }

        // block height query
        if let Some(block_height) = query.as_ref().and_then(|q| q.block_height) {
            let mut blocks: Vec<BlockWithCanonicity> = db
                .get_blocks_at_height(block_height)?
                .into_iter()
                .filter_map(|b| precomputed_matches_query(db, &query, b, total_num_blocks))
                .collect();

            reorder_asc(&mut blocks, sort_by);
            blocks.truncate(limit);
            return Ok(blocks);
        }

        // global slot query
        if let Some(global_slot_since_genesis) = query
            .as_ref()
            .and_then(|q| q.protocol_state.as_ref())
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot_since_genesis)
        {
            let mut blocks: Vec<BlockWithCanonicity> = db
                .get_blocks_at_slot(global_slot_since_genesis as u32)?
                .into_iter()
                .filter_map(|b| precomputed_matches_query(db, &query, b, total_num_blocks))
                .collect();

            reorder_asc(&mut blocks, sort_by);
            blocks.truncate(limit);
            return Ok(blocks);
        }

        // coinbase receiver query
        if let Some(coinbase_receiver) = query.as_ref().and_then(|q| {
            q.coinbase_receiver
                .as_ref()
                .and_then(|cb| cb.public_key.clone())
        }) {
            let mut blocks: Vec<BlockWithCanonicity> = db
                .get_blocks_at_public_key(&coinbase_receiver.into())?
                .into_iter()
                .filter_map(|b| precomputed_matches_query(db, &query, b, total_num_blocks))
                .collect();

            reorder_asc(&mut blocks, sort_by);
            blocks.truncate(limit); // TODO exit earlier
            return Ok(blocks);
        }

        // creator account query
        if let Some(creator_account) = query.as_ref().and_then(|q| {
            q.creator_account
                .as_ref()
                .and_then(|cb| cb.public_key.clone())
        }) {
            let mut blocks: Vec<BlockWithCanonicity> = db
                .get_blocks_at_public_key(&creator_account.into())?
                .into_iter()
                .filter_map(|b| precomputed_matches_query(db, &query, b, total_num_blocks))
                .collect();

            reorder_asc(&mut blocks, sort_by);
            blocks.truncate(limit); // TODO exit earlier
            return Ok(blocks);
        }

        // block height bounded query
        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let (min, max) = match query.as_ref() {
                Some(block_query_input) => {
                    let BlockQueryInput {
                        block_height_gt,
                        block_height_gte,
                        block_height_lt,
                        block_height_lte,
                        ..
                    } = block_query_input;
                    (
                        // min = max of the gt(e) heights or 1
                        block_height_gt
                            .map(|h| h.max(block_height_gte.unwrap_or_default()))
                            .unwrap_or(1),
                        // max = max of the lt(e) heights or best tip height
                        block_height_lt
                            .map(|h| h.max(block_height_lte.unwrap_or_default()))
                            .unwrap_or(db.get_best_block()?.unwrap().blockchain_length())
                            .min(db.get_best_block()?.unwrap().blockchain_length()),
                    )
                }
                None => (1, db.get_best_block()?.unwrap().blockchain_length()),
            };

            let block_heights: Vec<u32> = (min..=max).collect();

            for height in block_heights {
                for block in db.get_blocks_at_height(height)? {
                    if let Some(block_with_canonicity) =
                        precomputed_matches_query(db, &query, block, total_num_blocks)
                    {
                        blocks.push(block_with_canonicity);

                        if blocks.len() == limit {
                            break;
                        }
                    }
                }
            }

            reorder_desc(&mut blocks, sort_by);
            return Ok(blocks);
        }

        // global slot bounded query
        let consensus_state = query
            .as_ref()
            .and_then(|f| f.protocol_state.as_ref())
            .and_then(|f| f.consensus_state.as_ref());
        if consensus_state.map_or(false, |q| {
            q.slot_since_genesis_gt.is_some()
                || q.slot_since_genesis_gte.is_some()
                || q.slot_since_genesis_lt.is_some()
                || q.slot_since_genesis_lte.is_some()
        }) {
            let (min, max) = match consensus_state {
                Some(block_query_input) => {
                    let BlockProtocolStateConsensusStateQueryInput {
                        slot_since_genesis_lte,
                        slot_since_genesis_lt,
                        slot_since_genesis_gte,
                        slot_since_genesis_gt,
                        ..
                    } = block_query_input;
                    (
                        // min = max of the gt(e) heights or 1
                        slot_since_genesis_gt
                            .map(|h| h.max(slot_since_genesis_gte.unwrap_or_default()))
                            .unwrap_or(1),
                        // max = max of the lt(e) heights or best tip height
                        slot_since_genesis_lt
                            .map(|h| h.max(slot_since_genesis_lte.unwrap_or_default()))
                            .unwrap_or(db.get_best_block()?.unwrap().blockchain_length())
                            .min(db.get_best_block()?.unwrap().global_slot_since_genesis()),
                    )
                }
                None => (1, db.get_best_block()?.unwrap().global_slot_since_genesis()),
            };

            let block_slots: Vec<u32> = (min..=max).collect();

            for global_slot in block_slots {
                for block in db.get_blocks_at_slot(global_slot)? {
                    if let Some(block_with_canonicity) =
                        precomputed_matches_query(db, &query, block, total_num_blocks)
                    {
                        blocks.push(block_with_canonicity);

                        if blocks.len() == limit {
                            break;
                        }
                    }
                }
            }

            reorder_desc(&mut blocks, sort_by);
            return Ok(blocks);
        }

        // handle general search with global slot iterator
        let mode = match sort_by {
            BlockSortByInput::BlockHeightAsc => speedb::IteratorMode::Start,
            BlockSortByInput::BlockHeightDesc => speedb::IteratorMode::End,
        };
        for entry in blocks_global_slot_idx_iterator(db, mode).flatten() {
            let state_hash = blocks_global_slot_idx_state_hash_from_key(&entry.0)?;
            let pcb = db
                .get_block(&state_hash.clone().into())?
                .expect("block to be returned");
            let canonical = get_block_canonicity(db, &state_hash);
            let block = BlockWithCanonicity::from_precomputed(pcb, total_num_blocks, canonical);

            if query.as_ref().map_or(true, |q| q.matches(&block)) {
                blocks.push(block);
            }

            if blocks.len() == limit {
                break;
            }
        }

        Ok(blocks)
    }
}

fn reorder_asc<T>(values: &mut [T], sort_by: BlockSortByInput) {
    match sort_by {
        BlockSortByInput::BlockHeightAsc => values.reverse(),
        BlockSortByInput::BlockHeightDesc => (),
    }
}

fn reorder_desc<T>(values: &mut [T], sort_by: BlockSortByInput) {
    match sort_by {
        BlockSortByInput::BlockHeightAsc => (),
        BlockSortByInput::BlockHeightDesc => values.reverse(),
    }
}

fn precomputed_matches_query(
    db: &Arc<IndexerStore>,
    query: &Option<BlockQueryInput>,
    block: PrecomputedBlock,
    total_num_blocks: u32,
) -> Option<BlockWithCanonicity> {
    let canonical = get_block_canonicity(db, &block.state_hash().0);
    let block_with_canonicity =
        BlockWithCanonicity::from_precomputed(block, total_num_blocks, canonical);
    if query
        .as_ref()
        .map_or(true, |q| q.matches(&block_with_canonicity))
    {
        Some(block_with_canonicity)
    } else {
        None
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BlockSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(SimpleObject)]
pub struct BlockWithCanonicity {
    /// Value canonical
    pub canonical: bool,

    /// Value total_num_blocks
    #[graphql(name = "total_num_blocks")]
    pub total_num_blocks: u32,

    /// Value block
    #[graphql(flatten)]
    pub block: Block,
}

#[derive(SimpleObject)]
pub struct Block {
    /// Value state_hash
    state_hash: String,

    /// Value block_height
    block_height: u32,

    /// Value global_slot_since_genesis
    global_slot_since_genesis: u32,

    /// The public_key for the winner account
    winner_account: PK,

    /// Value date_time as ISO 8601 string
    date_time: String,

    /// Value received_time as ISO 8601 string
    received_time: String,

    /// The public_key for the creator account
    creator_account: PK,

    /// The public_key for the coinbase_receiver
    coinbase_receiver: PK,

    /// Value creator public key
    creator: String,

    /// Value protocol state
    protocol_state: ProtocolState,

    /// Value transaction fees
    tx_fees: String,

    /// Value SNARK fees
    snark_fees: String,

    /// Value transactions
    transactions: Transactions,

    /// Value snark jobs
    snark_jobs: Vec<SnarkJob>,
}

#[derive(SimpleObject)]

struct SnarkJob {
    /// Value block state hash
    block_state_hash: String,

    /// Valuable block height
    block_height: u32,

    /// Value date time
    date_time: String,

    /// Value fee
    fee: u64,

    /// Value prover
    prover: String,
}

#[derive(SimpleObject)]
struct Transactions {
    /// Value coinbase
    coinbase: String,

    /// The public key for the coinbase receiver account
    coinbase_receiver_account: PK,

    /// Value fee transfer
    fee_transfer: Vec<BlockFeetransfer>,

    /// Value user commands
    user_commands: Vec<Transaction>,
}

#[derive(SimpleObject)]
struct BlockFeetransfer {
    pub fee: String,
    pub recipient: String,
    #[graphql(name = "type")]
    pub feetransfer_kind: String,
}

#[derive(SimpleObject)]
struct ConsensusState {
    /// Value total currency
    total_currency: u64,

    /// Value block height
    blockchain_length: u32,

    /// Value block height
    block_height: u32,

    /// Value epoch count
    epoch_count: u32,

    /// Value epoch count
    epoch: u32,

    /// Value has ancestors the same checkpoint window
    has_ancestor_in_same_checkpoint_window: bool,

    /// Value last VRF output
    last_vrf_output: String,

    /// Value minimum window density
    min_window_density: u32,

    /// Value current slot
    slot: u32,

    /// Value global slot
    slot_since_genesis: u32,

    /// Value next epoch data
    next_epoch_data: NextEpochData,

    /// Value next epoch data
    staking_epoch_data: StakingEpochData,
}

#[derive(SimpleObject)]
struct StakingEpochData {
    /// Value seed
    seed: String,

    /// Value epoch length
    epoch_length: u32,

    /// Value start checkpoint
    start_checkpoint: String,

    /// Value lock checkpoint
    lock_checkpoint: String,

    /// Value staking ledger
    ledger: StakingEpochDataLedger,
}

#[derive(SimpleObject)]
struct NextEpochData {
    /// Value seed
    seed: String,

    /// Value epoch length
    epoch_length: u32,

    /// Value start checkpoint
    start_checkpoint: String,

    /// Value lock checkpoint
    lock_checkpoint: String,

    /// Value next ledger
    ledger: NextEpochDataLedger,
}

#[derive(SimpleObject)]
struct NextEpochDataLedger {
    /// Value hash
    hash: String,

    /// Value total currency
    total_currency: u64,
}

#[derive(SimpleObject)]
struct StakingEpochDataLedger {
    /// Value hash
    hash: String,

    /// Value total currency
    total_currency: u64,
}

#[derive(SimpleObject)]
struct BlockchainState {
    /// Value utc_date as numeric string
    utc_date: String,

    /// Value date as numeric string
    date: String,

    /// Value snarked ledger hash
    snarked_ledger_hash: String,

    /// Value staged ledger hash
    staged_ledger_hash: String,
}

#[derive(SimpleObject)]
struct ProtocolState {
    /// Value parent state hash
    previous_state_hash: String,

    /// Value blockchain state
    blockchain_state: BlockchainState,

    /// Value consensus state
    consensus_state: ConsensusState,
}

impl Block {
    pub fn new(block: PrecomputedBlock, canonical: bool) -> Self {
        let winner_account = block.block_creator().0;
        let date_time = millis_to_iso_date_string(block.timestamp().try_into().unwrap());
        let pk_creator = block.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();
        let scheduled_time = block.scheduled_time().clone();
        let received_time = millis_to_iso_date_string(scheduled_time.parse::<i64>().unwrap());
        let previous_state_hash = block.previous_state_hash().0;
        let tx_fees = block.tx_fees();
        let snark_fees = block.snark_fees();
        let utc_date = block.timestamp().to_string();

        // blockchain state
        let blockchain_state = block.blockchain_state();
        let snarked_ledger_hash =
            LedgerHash::from_hashv1(blockchain_state.clone().snarked_ledger_hash).0;
        let staged_ledger_hashv1 = blockchain_state
            .staged_ledger_hash
            .t
            .t
            .non_snark
            .t
            .ledger_hash;
        let staged_ledger_hash = LedgerHash::from_hashv1(staged_ledger_hashv1).0;

        // consensus state
        let consensus_state = block.consensus_state();

        let total_currency = consensus_state.total_currency.t.t;
        let blockchain_length = block.blockchain_length();
        let block_height = blockchain_length;
        let epoch_count = block.epoch_count();
        let epoch = epoch_count;
        let has_ancestor_in_same_checkpoint_window =
            consensus_state.has_ancestor_in_same_checkpoint_window;
        let last_vrf_output = block.last_vrf_output();
        let min_window_density = consensus_state.min_window_density.t.t;
        let slot_since_genesis = consensus_state.global_slot_since_genesis.t.t;
        let slot = consensus_state.curr_global_slot.t.t.slot_number.t.t;

        // NextEpochData
        let seed_hashv1 = consensus_state.next_epoch_data.t.t.seed;
        let seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            seed_hashv1.into();
        let seed = seed_bs58.to_base58_string().expect("bs58 encoded seed");
        let epoch_length = consensus_state.next_epoch_data.t.t.epoch_length.t.t;

        let start_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.start_checkpoint;
        let start_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            start_checkpoint_hashv1.into();
        let start_checkpoint = start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let lock_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.lock_checkpoint;
        let lock_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            lock_checkpoint_hashv1.into();
        let lock_checkpoint = lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let ledger_hashv1 = consensus_state.next_epoch_data.t.t.ledger.t.t.hash;
        let ledger_hash_bs58: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            ledger_hashv1.into();
        let ledger_hash = ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let ledger_total_currency = consensus_state
            .next_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;

        // StakingEpochData
        let staking_seed_hashv1 = consensus_state.staking_epoch_data.t.t.seed;
        let staking_seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            staking_seed_hashv1.into();
        let staking_seed = staking_seed_bs58
            .to_base58_string()
            .expect("bs58 encoded seed");

        let staking_epoch_length = consensus_state.staking_epoch_data.t.t.epoch_length.t.t;

        let staking_start_checkpoint_hashv1 =
            consensus_state.staking_epoch_data.t.t.start_checkpoint;
        let staking_start_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_start_checkpoint_hashv1.into();
        let staking_start_checkpoint = staking_start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let staking_lock_checkpoint_hashv1 = consensus_state.staking_epoch_data.t.t.lock_checkpoint;
        let staking_lock_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_lock_checkpoint_hashv1.into();
        let staking_lock_checkpoint = staking_lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let staking_ledger_hashv1 = consensus_state.staking_epoch_data.t.t.ledger.t.t.hash;
        let staking_ledger_hash_bs58: Base58EncodableVersionedType<
            { version_bytes::LEDGER_HASH },
            _,
        > = staking_ledger_hashv1.into();
        let staking_ledger_hash = staking_ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let staking_ledger_total_currency = consensus_state
            .staking_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;

        let coinbase_receiver_account = block.coinbase_receiver().0;
        let supercharged = consensus_state.supercharge_coinbase;
        let coinbase: u64 = if supercharged {
            2 * MAINNET_COINBASE_REWARD
        } else {
            MAINNET_COINBASE_REWARD
        };

        let fee_transfers: Vec<BlockFeetransfer> = InternalCommand::from_precomputed(&block)
            .into_iter()
            .map(|cmd| InternalCommandWithData::from_internal_cmd(cmd, &block))
            .filter(|x| matches!(x, InternalCommandWithData::FeeTransfer { .. }))
            .map(|ft| ft.into())
            .collect();

        let user_commands: Vec<Transaction> = SignedCommandWithData::from_precomputed(&block)
            .into_iter()
            .map(|cmd| Transaction::new(cmd, canonical))
            .collect();

        let snark_jobs: Vec<SnarkJob> = SnarkWorkSummary::from_precomputed(&block)
            .into_iter()
            .map(|snark| (snark, block.state_hash().0, block_height, date_time.clone()).into())
            .collect();

        Self {
            date_time,
            snark_jobs,
            state_hash: block.state_hash().0,
            block_height: block.blockchain_length(),
            global_slot_since_genesis: block.global_slot_since_genesis(),
            coinbase_receiver: PK {
                public_key: block.coinbase_receiver().0,
            },
            winner_account: PK {
                public_key: winner_account,
            },
            creator_account: PK {
                public_key: creator.clone(),
            },
            creator,
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
                    epoch_count,
                    has_ancestor_in_same_checkpoint_window,
                    last_vrf_output,
                    min_window_density,
                    slot,
                    slot_since_genesis,
                    next_epoch_data: NextEpochData {
                        seed,
                        epoch_length,
                        start_checkpoint,
                        lock_checkpoint,
                        ledger: NextEpochDataLedger {
                            hash: ledger_hash,
                            total_currency: ledger_total_currency,
                        },
                    },
                    staking_epoch_data: StakingEpochData {
                        seed: staking_seed,
                        epoch_length: staking_epoch_length,
                        start_checkpoint: staking_start_checkpoint,
                        lock_checkpoint: staking_lock_checkpoint,
                        ledger: StakingEpochDataLedger {
                            hash: staking_ledger_hash,
                            total_currency: staking_ledger_total_currency,
                        },
                    },
                },
            },
            tx_fees: tx_fees.to_string(),
            snark_fees: snark_fees.to_string(),
            transactions: Transactions {
                coinbase: coinbase.to_string(),
                coinbase_receiver_account: PK {
                    public_key: coinbase_receiver_account,
                },
                fee_transfer: fee_transfers,
                user_commands,
            },
        }
    }
}

fn check_option<T: PartialEq>(matches: &mut bool, opt: &Option<T>, block_value: &T) {
    if let Some(value) = opt {
        *matches &= block_value == value;
    }
}

impl BlockQueryInput {
    pub fn matches(&self, block: &BlockWithCanonicity) -> bool {
        let mut matches = true;
        let Self {
            creator_account,
            coinbase_receiver,
            canonical,
            or,
            and,
            state_hash,
            block_height: blockchain_length,
            global_slot_since_genesis,
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            protocol_state,
            ..
        } = self;

        check_option(&mut matches, canonical, &block.canonical);
        check_option(&mut matches, state_hash, &block.block.state_hash);
        check_option(&mut matches, blockchain_length, &block.block.block_height);
        check_option(
            &mut matches,
            global_slot_since_genesis,
            &block.block.global_slot_since_genesis,
        );

        // block_height_gt(e) & block_height_lt(e)
        if let Some(height) = block_height_gt {
            matches &= block.block.block_height > *height;
        }
        if let Some(height) = block_height_gte {
            matches &= block.block.block_height >= *height;
        }
        if let Some(height) = block_height_lt {
            matches &= block.block.block_height < *height;
        }
        if let Some(height) = block_height_lte {
            matches &= block.block.block_height <= *height;
        }

        // global_slot_gt(e) & global_slot_lt(e)
        if let Some(global_slot) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_gt)
        {
            matches &= block.block.global_slot_since_genesis > global_slot;
        }
        if let Some(global_slot) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_gte)
        {
            matches &= block.block.global_slot_since_genesis >= global_slot;
        }
        if let Some(global_slot) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_lt)
        {
            matches &= block.block.global_slot_since_genesis < global_slot;
        }
        if let Some(global_slot) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_lte)
        {
            matches &= block.block.global_slot_since_genesis <= global_slot;
        }

        // slot
        matches &= protocol_state
            .as_ref()
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot)
            .map_or(matches, |slot| {
                block.block.protocol_state.consensus_state.slot as i64 == slot
            });

        // creator account
        if let Some(creator_account) = creator_account {
            if let Some(public_key) = creator_account.public_key.as_ref() {
                matches &= block.block.creator_account.public_key == *public_key;
            }
        }

        // coinbase receiver
        if let Some(coinbase_receiver) = coinbase_receiver {
            if let Some(public_key) = coinbase_receiver.public_key.as_ref() {
                matches &= block.block.coinbase_receiver.public_key == *public_key;
            }
        }

        // conjunction
        if let Some(query) = and {
            matches &= query.iter().all(|and| and.matches(block));
        }

        // disjunction
        if let Some(query) = or {
            if !query.is_empty() {
                matches &= query.iter().any(|or| or.matches(block));
            }
        }
        matches
    }
}

impl BlockWithCanonicity {
    pub fn from_precomputed(
        block: PrecomputedBlock,
        total_num_blocks: u32,
        canonical: bool,
    ) -> Self {
        Self {
            canonical,
            total_num_blocks,
            block: Block::new(block, canonical),
        }
    }
}

impl From<InternalCommandWithData> for BlockFeetransfer {
    fn from(int_cmd: InternalCommandWithData) -> Self {
        match int_cmd {
            InternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount.to_string(),
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
            InternalCommandWithData::Coinbase {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount.to_string(),
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
        }
    }
}

impl From<(SnarkWorkSummary, String, u32, String)> for SnarkJob {
    fn from(value: (SnarkWorkSummary, String, u32, String)) -> Self {
        Self {
            block_state_hash: value.1,
            block_height: value.2,
            date_time: value.3,
            fee: value.0.fee,
            prover: value.0.prover.to_string(),
        }
    }
}

impl std::fmt::Display for TransactionStatusFailedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatusFailedType::Predicate => write!(f, "Predicate"),
            TransactionStatusFailedType::SourceNotPresent => write!(f, "Source_not_present"),
            TransactionStatusFailedType::ReceiverNotPresent => write!(f, "Receiver_not_present"),
            TransactionStatusFailedType::AmountInsufficientToCreateAccount => {
                write!(f, "Amount_insufficient_to_create_account")
            }
            TransactionStatusFailedType::CannotPayCreationFeeInToken => {
                write!(f, "Cannot_pay_creation_fee_in_token")
            }
            TransactionStatusFailedType::SourceInsufficientBalance => {
                write!(f, "Source_insufficient_balance")
            }
            TransactionStatusFailedType::SourceMinimumBalanceViolation => {
                write!(f, "Source_minimum_balance_violation")
            }
            TransactionStatusFailedType::ReceiverAlreadyExists => {
                write!(f, "Receiver_already_exists")
            }
            TransactionStatusFailedType::NotTokenOwner => write!(f, "Not_token_owner"),
            TransactionStatusFailedType::MismatchedTokenPermissions => {
                write!(f, "Mismatched_token_permissions")
            }
            TransactionStatusFailedType::Overflow => write!(f, "Overflow"),
            TransactionStatusFailedType::SignedCommandOnSnappAccount => {
                write!(f, "Signed_command_on_snapp_account")
            }
            TransactionStatusFailedType::SnappAccountNotPresent => {
                write!(f, "Snapp_account_not_present")
            }
            TransactionStatusFailedType::UpdateNotPermitted => write!(f, "Update_not_permitted"),
            TransactionStatusFailedType::IncorrectNonce => write!(f, "Incorrect_nonce"),
        }
    }
}
