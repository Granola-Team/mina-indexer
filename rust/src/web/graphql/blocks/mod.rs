use super::{
    db, get_block_canonicity, millis_to_iso_date_string, transactions::TransactionWithoutBlock,
    MAINNET_COINBASE_REWARD, MAINNET_EPOCH_SLOT_COUNT, PK,
};
use crate::{
    block::{is_valid_state_hash, precomputed::PrecomputedBlock, store::BlockStore},
    command::{
        internal::{store::InternalCommandStore, DbInternalCommand, DbInternalCommandWithData},
        signed::SignedCommandWithData,
        store::UserCommandStore,
    },
    ledger::public_key::PublicKey,
    snark_work::{store::SnarkStore, SnarkWorkSummary},
    store::IndexerStore,
    utility::store::{block_u32_prefix_from_key, from_be_bytes, state_hash_suffix, U32_LEN},
    web::graphql::{
        gen::{BlockProtocolStateConsensusStateQueryInput, BlockQueryInput},
        get_block,
    },
};
use async_graphql::{self, Enum, Object, Result, SimpleObject};
use log::error;
use serde::Serialize;
use speedb::{Direction, IteratorMode};
use std::{collections::HashSet, sync::Arc};

#[derive(Default)]
pub struct BlocksQueryRoot;

#[Object]
impl BlocksQueryRoot {
    async fn block<'ctx>(
        &self,
        ctx: &async_graphql::Context<'ctx>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<Block>> {
        let db = db(ctx);

        // no query filters => get the best block
        if query.is_none() {
            let counts = get_counts(db).await?;
            return Ok(db
                .get_best_block()
                .map(|b| b.map(|pcb| Block::from_precomputed(db, &pcb, counts)))?);
        }

        // Use constant time access if we have the state hash
        if let Some(state_hash) = query.as_ref().and_then(|input| input.state_hash.clone()) {
            if !is_valid_state_hash(&state_hash) {
                return Ok(None);
            }

            let pcb = match db.get_block(&state_hash.into())? {
                Some((pcb, _)) => pcb,
                None => return Ok(None),
            };
            let block = Block::from_precomputed(db, &pcb, get_counts(db).await?);
            if query.unwrap().matches(&block) {
                return Ok(Some(block));
            }
            return Ok(None);
        }

        // else iterate over height-sorted blocks
        for (key, _) in db
            .blocks_height_iterator(speedb::IteratorMode::End)
            .flatten()
        {
            let state_hash = state_hash_suffix(&key)?;
            let pcb = get_block(db, &state_hash);

            let block = Block::from_precomputed(db, &pcb, get_counts(db).await?);

            if query.as_ref().map_or(true, |q| q.matches(&block)) {
                return Ok(Some(block));
            }
        }
        Ok(None)
    }

    async fn blocks<'ctx>(
        &self,
        ctx: &async_graphql::Context<'ctx>,
        query: Option<BlockQueryInput>,
        #[graphql(default = 100)] limit: usize,
        sort_by: Option<BlockSortByInput>,
    ) -> Result<Vec<Block>> {
        use speedb::{Direction::*, IteratorMode::*};
        use BlockSortByInput::*;
        let db = db(ctx);

        // unique block producer query
        if let Some(mut num_blocks) = query
            .as_ref()
            .and_then(|q| q.unique_block_producers_last_n_blocks)
        {
            const MAX_NUM_BLOCKS: u32 = 1000;
            num_blocks = num_blocks.min(MAX_NUM_BLOCKS);

            if let Some(best_height) = db.get_best_block_height()? {
                let start_height = 1.max(best_height.saturating_sub(num_blocks));
                let mut producers = HashSet::new();
                for (key, _) in db
                    .blocks_height_iterator(IteratorMode::From(
                        &(best_height + 1).to_be_bytes(),
                        Direction::Reverse,
                    ))
                    .flatten()
                {
                    let height = from_be_bytes(key[..U32_LEN].to_vec());
                    if height <= start_height {
                        break;
                    }

                    let state_hash = state_hash_suffix(&key)?;
                    if let Some(creator) = db.get_block_creator(&state_hash)? {
                        producers.insert(creator);
                        continue;
                    }
                    error!("Block creator index missing (length {height}) {state_hash}")
                }
                return Ok(vec![Block {
                    num_unique_block_producers_last_n_blocks: Some(producers.len() as u32),
                    ..Default::default()
                }]);
            }
        }

        let counts = get_counts(db).await?;
        let mut blocks = Vec::new();
        let sort_by = sort_by.unwrap_or(BlockHeightDesc);

        // state hash query
        if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
            let block = db.get_block(&state_hash.into())?;
            return Ok(block
                .iter()
                .filter_map(|(b, _)| precomputed_matches_query(db, &query, b, counts))
                .collect());
        }

        // block height query
        if let Some(block_height) = query.as_ref().and_then(|q| q.block_height) {
            for state_hash in db.get_blocks_at_height(block_height)?.iter() {
                let pcb = get_block(db, state_hash);
                if let Some(block) = precomputed_matches_query(db, &query, &pcb, counts) {
                    blocks.push(block);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(blocks);
        }

        // global slot query
        if let Some(global_slot) = query
            .as_ref()
            .and_then(|q| q.protocol_state.as_ref())
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot_since_genesis)
            .or(query.as_ref().and_then(|q| q.global_slot_since_genesis))
        {
            for state_hash in db.get_blocks_at_slot(global_slot)?.iter() {
                let pcb = get_block(db, state_hash);
                if let Some(block) = precomputed_matches_query(db, &query, &pcb, counts) {
                    blocks.push(block);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(blocks);
        }

        // coinbase receiver query
        if let Some(coinbase_receiver) = query.as_ref().and_then(|q| {
            q.coinbase_receiver
                .as_ref()
                .and_then(|cb| cb.public_key.clone())
        }) {
            let start = coinbase_receiver.as_bytes();
            let mut end = [0; PublicKey::LEN + U32_LEN];
            end[..PublicKey::LEN].copy_from_slice(start);
            end[PublicKey::LEN..].copy_from_slice(&u32::MAX.to_be_bytes());

            let iter = match sort_by {
                BlockHeightAsc => db.coinbase_receiver_block_height_iterator(From(start, Forward)),
                BlockHeightDesc => db.coinbase_receiver_block_height_iterator(From(&end, Reverse)),
                GlobalSlotAsc => db.coinbase_receiver_global_slot_iterator(From(start, Forward)),
                GlobalSlotDesc => db.coinbase_receiver_global_slot_iterator(From(&end, Reverse)),
            };
            for (key, _) in iter.flatten() {
                if key[..PublicKey::LEN] != *coinbase_receiver.as_bytes() {
                    break;
                }

                // avoid deserializing PCB if possible
                let state_hash = state_hash_suffix(&key)?;
                if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                    if get_block_canonicity(db, &state_hash) != query_canonicity {
                        continue;
                    }
                }

                let pcb = get_block(db, &state_hash);
                if let Some(block) = precomputed_matches_query(db, &query, &pcb, counts) {
                    blocks.push(block);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(blocks);
        }

        // creator account query
        if let Some(creator_account) = query.as_ref().and_then(|q| {
            q.creator_account
                .as_ref()
                .and_then(|cb| cb.public_key.clone())
        }) {
            // properly set the upper bound for block height
            let upper_bound = match (
                query.as_ref().and_then(|q| q.block_height_lt),
                query.as_ref().and_then(|q| q.block_height_lte),
            ) {
                (Some(lt), Some(lte)) => std::cmp::min(lte, lt - 1),
                (Some(lt), None) => lt - 1,
                (None, Some(lte)) => lte,
                (None, None) => u32::MAX,
            };
            let start = creator_account.as_bytes();
            let mut end = [0; PublicKey::LEN + U32_LEN];
            end[..PublicKey::LEN].copy_from_slice(start);
            end[PublicKey::LEN..].copy_from_slice(&upper_bound.to_be_bytes());

            let iter = match sort_by {
                BlockHeightAsc => db.block_creator_block_height_iterator(From(start, Forward)),
                BlockHeightDesc => db.block_creator_block_height_iterator(From(&end, Reverse)),
                GlobalSlotAsc => db.block_creator_global_slot_iterator(From(start, Forward)),
                GlobalSlotDesc => db.block_creator_global_slot_iterator(From(&end, Reverse)),
            };
            for (key, _) in iter.flatten() {
                if key[..PublicKey::LEN] != *creator_account.as_bytes() {
                    break;
                }

                // avoid deserializing PCB if possible
                let state_hash = state_hash_suffix(&key)?;
                if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                    if get_block_canonicity(db, &state_hash) != query_canonicity {
                        continue;
                    }
                }

                let pcb = get_block(db, &state_hash);
                if let Some(block) = precomputed_matches_query(db, &query, &pcb, counts) {
                    blocks.push(block);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            return Ok(blocks);
        }

        // block height bounded query
        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let (min, max) = {
                let BlockQueryInput {
                    block_height_gt,
                    block_height_gte,
                    block_height_lt,
                    block_height_lte,
                    ..
                } = query.as_ref().expect("query will contain a value");
                let min_bound = match (*block_height_gte, *block_height_gt) {
                    (Some(gte), Some(gt)) => std::cmp::max(gte, gt + 1),
                    (Some(gte), None) => gte,
                    (None, Some(gt)) => gt + 1,
                    (None, None) => 1,
                };

                let max_bound = match (*block_height_lte, *block_height_lt) {
                    (Some(lte), Some(lt)) => std::cmp::min(lte, lt - 1),
                    (Some(lte), None) => lte,
                    (None, Some(lt)) => lt - 1,
                    (None, None) => db.get_best_block_height()?.unwrap(),
                };
                (min_bound, max_bound)
            };

            let start = min.to_be_bytes();
            let end = (max + 1).to_be_bytes();
            let mode = match sort_by {
                BlockHeightAsc => From(&start, Forward),
                _ => From(&end, Reverse),
            };
            for (key, _) in db.blocks_height_iterator(mode).flatten() {
                let height = block_u32_prefix_from_key(&key)?;
                if height < min || height > max {
                    break;
                }

                // avoid deserializing PCB if possible
                let state_hash = state_hash_suffix(&key)?;
                if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                    if get_block_canonicity(db, &state_hash) != query_canonicity {
                        continue;
                    }
                }

                let pcb = get_block(db, &state_hash);
                if let Some(block_with_canonicity) =
                    precomputed_matches_query(db, &query, &pcb, counts)
                {
                    blocks.push(block_with_canonicity);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            reorder(db, &mut blocks, sort_by);
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
            let (min, max) = {
                let BlockProtocolStateConsensusStateQueryInput {
                    slot_since_genesis_lte,
                    slot_since_genesis_lt,
                    slot_since_genesis_gte,
                    slot_since_genesis_gt,
                    ..
                } = consensus_state
                    .as_ref()
                    .expect("consensus will have a value");
                let min_bound = match (*slot_since_genesis_gte, *slot_since_genesis_gt) {
                    (Some(gte), Some(gt)) => std::cmp::max(gte, gt + 1),
                    (Some(gte), None) => gte,
                    (None, Some(gt)) => gt + 1,
                    (None, None) => 0,
                };

                let max_bound = match (*slot_since_genesis_lte, *slot_since_genesis_lt) {
                    (Some(lte), Some(lt)) => std::cmp::min(lte, lt - 1),
                    (Some(lte), None) => lte,
                    (None, Some(lt)) => lt - 1,
                    (None, None) => db.get_best_block_global_slot()?.unwrap(),
                };
                (min_bound, max_bound)
            };

            let start = min.to_be_bytes();
            let end = (max + 1).to_be_bytes();
            let mode = match sort_by {
                GlobalSlotAsc => From(&start, Forward),
                _ => From(&end, Reverse),
            };
            for (key, _) in db.blocks_global_slot_iterator(mode).flatten() {
                let slot = block_u32_prefix_from_key(&key)?;
                if slot < min || slot > max {
                    break;
                }

                // avoid deserializing PCB if possible
                let state_hash = state_hash_suffix(&key)?;
                if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                    if get_block_canonicity(db, &state_hash) != query_canonicity {
                        continue;
                    }
                }

                let pcb = get_block(db, &state_hash);
                if let Some(block_with_canonicity) =
                    precomputed_matches_query(db, &query, &pcb, counts)
                {
                    blocks.push(block_with_canonicity);
                    if blocks.len() >= limit {
                        break;
                    }
                }
            }
            reorder(db, &mut blocks, sort_by);
            return Ok(blocks);
        }

        // default query handler
        let start = 0u32.to_be_bytes();
        let end = u32::MAX.to_be_bytes();
        let iter = match sort_by {
            BlockHeightAsc => db.blocks_height_iterator(From(&start, Forward)),
            BlockHeightDesc => db.blocks_height_iterator(From(&end, Reverse)),
            GlobalSlotAsc => db.blocks_global_slot_iterator(From(&start, Forward)),
            GlobalSlotDesc => db.blocks_global_slot_iterator(From(&end, Reverse)),
        };
        for (key, _) in iter.flatten() {
            // avoid deserializing PCB if possible
            let state_hash = state_hash_suffix(&key)?;
            if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                if get_block_canonicity(db, &state_hash) != query_canonicity {
                    continue;
                }
            }

            let pcb = get_block(db, &state_hash);
            if let Some(block_with_canonicity) = precomputed_matches_query(db, &query, &pcb, counts)
            {
                blocks.push(block_with_canonicity);
                if blocks.len() >= limit {
                    break;
                }
            }
        }
        Ok(blocks)
    }
}

fn precomputed_matches_query(
    db: &Arc<IndexerStore>,
    query: &Option<BlockQueryInput>,
    block: &PrecomputedBlock,
    counts: [u32; 13],
) -> Option<Block> {
    let block_with_canonicity = Block::from_precomputed(db, block, counts);
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

    #[graphql(name = "GLOBALSLOT_ASC")]
    GlobalSlotAsc,
    #[graphql(name = "GLOBALSLOT_DESC")]
    GlobalSlotDesc,
}

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

    /// Value total num canonical blocks
    #[graphql(name = "total_num_canonical_blocks")]
    pub total_num_canonical_blocks: u32,

    /// Value total num supercharged blocks
    #[graphql(name = "total_num_supercharged_blocks")]
    pub total_num_supercharged_blocks: u32,

    /// Value block num snarks
    #[graphql(name = "block_num_snarks")]
    pub block_num_snarks: u32,

    /// Value block num user commands
    #[graphql(name = "block_num_user_commands")]
    pub block_num_user_commands: u32,

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

#[derive(Default, SimpleObject, Serialize)]
pub struct BlockWithoutCanonicity {
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

#[derive(SimpleObject, Serialize)]
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

#[derive(Default, SimpleObject, Serialize)]
struct Transactions {
    /// Value coinbase
    coinbase: String,

    /// The public key for the coinbase receiver account
    coinbase_receiver_account: PK,

    /// Value fee transfer
    fee_transfer: Vec<BlockFeetransfer>,

    /// Value user commands
    user_commands: Vec<TransactionWithoutBlock>,
}

#[derive(Default, SimpleObject, Serialize)]
struct BlockFeetransfer {
    pub fee: String,
    pub recipient: String,

    #[graphql(name = "type")]
    pub feetransfer_kind: String,
}

#[derive(Default, SimpleObject, Serialize)]
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

#[derive(Default, SimpleObject, Serialize)]
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

#[derive(Default, SimpleObject, Serialize)]
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

#[derive(Default, SimpleObject, Serialize)]
struct NextEpochDataLedger {
    /// Value hash
    hash: String,

    /// Value total currency
    total_currency: u64,
}

#[derive(Default, SimpleObject, Serialize)]
struct StakingEpochDataLedger {
    /// Value hash
    hash: String,

    /// Value total currency
    total_currency: u64,
}

#[derive(Default, SimpleObject, Serialize)]
struct BlockchainState {
    /// Value utc_date as numeric string
    utc_date: String,

    /// Value date as numeric string
    date: String,

    /// Value snarked ledger hash
    snarked_ledger_hash: Option<String>,

    /// Value staged ledger hash
    staged_ledger_hash: String,
}

#[derive(Default, SimpleObject, Serialize)]
struct ProtocolState {
    /// Value parent state hash
    previous_state_hash: String,

    /// Value blockchain state
    blockchain_state: BlockchainState,

    /// Value consensus state
    consensus_state: ConsensusState,
}

impl BlockWithoutCanonicity {
    pub fn new(
        block: &PrecomputedBlock,
        canonical: bool,
        epoch_num_user_commands: u32,
        total_num_user_commands: u32,
    ) -> Self {
        let winner_account = block.block_creator().0;
        let date_time = millis_to_iso_date_string(block.timestamp() as i64);
        let creator = block.block_creator().0;
        let scheduled_time = block.scheduled_time();
        let received_time = millis_to_iso_date_string(scheduled_time.parse::<i64>().unwrap());
        let previous_state_hash = block.previous_state_hash().0;
        let tx_fees = block.tx_fees();
        let snark_fees = block.snark_fees();
        let utc_date = block.timestamp().to_string();

        // blockchain state
        let snarked_ledger_hash = block.snarked_ledger_hash().map(|hash| hash.0);
        let staged_ledger_hash = block.staged_ledger_hash().0;

        // consensus state
        let total_currency = block.total_currency();
        let blockchain_length = block.blockchain_length();
        let block_height = blockchain_length;
        let epoch_count = block.epoch_count();
        let epoch = epoch_count;
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

        let coinbase_receiver_account = block.coinbase_receiver().0;
        let supercharged = block.supercharge_coinbase();
        let coinbase: u64 = if supercharged {
            2 * MAINNET_COINBASE_REWARD
        } else {
            MAINNET_COINBASE_REWARD
        };

        let fee_transfers: Vec<BlockFeetransfer> = DbInternalCommand::from_precomputed(block)
            .into_iter()
            .map(|cmd| {
                DbInternalCommandWithData::from_internal_cmd(
                    cmd,
                    block.state_hash(),
                    block.blockchain_length(),
                    block.timestamp() as i64,
                )
            })
            .filter(|x| matches!(x, DbInternalCommandWithData::FeeTransfer { .. }))
            .map(|ft| ft.into())
            .collect();

        let user_commands: Vec<TransactionWithoutBlock> =
            SignedCommandWithData::from_precomputed(block)
                .into_iter()
                .map(|cmd| {
                    TransactionWithoutBlock::new(
                        cmd,
                        canonical,
                        epoch_num_user_commands,
                        total_num_user_commands,
                    )
                })
                .collect();

        let snark_jobs: Vec<SnarkJob> = SnarkWorkSummary::from_precomputed(block)
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

impl BlockQueryInput {
    pub fn matches(&self, block: &Block) -> bool {
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
            unique_block_producers_last_n_blocks: _,
        } = self;
        if let Some(canonical) = canonical {
            if block.canonical != *canonical {
                return false;
            }
        }
        if let Some(state_hash) = state_hash {
            if block.block.state_hash != *state_hash {
                return false;
            }
        }
        if let Some(blockchain_length) = blockchain_length {
            if block.block.block_height != *blockchain_length {
                return false;
            }
        }

        // global slot
        if protocol_state
            .as_ref()
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot_since_genesis)
            .or(*global_slot_since_genesis)
            .map_or(false, |global_slot| {
                block
                    .block
                    .protocol_state
                    .consensus_state
                    .slot_since_genesis
                    != global_slot
                    || block.block.global_slot_since_genesis != global_slot
            })
        {
            return false;
        }

        // epoch slot
        if protocol_state
            .as_ref()
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot)
            .map_or(false, |slot| {
                block.block.protocol_state.consensus_state.slot != slot
            })
        {
            return false;
        }

        // block_height_gt(e) & block_height_lt(e)
        if let Some(height) = block_height_gt {
            if block.block.block_height <= *height {
                return false;
            }
        }
        if let Some(height) = block_height_gte {
            if block.block.block_height < *height {
                return false;
            }
        }
        if let Some(height) = block_height_lt {
            if block.block.block_height >= *height {
                return false;
            }
        }
        if let Some(height) = block_height_lte {
            if block.block.block_height > *height {
                return false;
            }
        }

        // global_slot_gt(e) & global_slot_lt(e)
        if let Some(slot_since_genesis_gt) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_gt)
        {
            if block.block.global_slot_since_genesis <= slot_since_genesis_gt {
                return false;
            }
        }
        if let Some(slot_since_genesis_gte) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_gte)
        {
            if block.block.global_slot_since_genesis < slot_since_genesis_gte {
                return false;
            }
        }
        if let Some(slot_since_genesis_lt) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_lt)
        {
            if block.block.global_slot_since_genesis >= slot_since_genesis_lt {
                return false;
            }
        }
        if let Some(slot_since_genesis_lte) = protocol_state
            .as_ref()
            .and_then(|f| f.consensus_state.as_ref())
            .and_then(|f| f.slot_since_genesis_lte)
        {
            if block.block.global_slot_since_genesis > slot_since_genesis_lte {
                return false;
            }
        }

        // creator account
        if let Some(creator_account) = creator_account {
            if let Some(public_key) = creator_account.public_key.as_ref() {
                if block.block.creator_account.public_key != *public_key {
                    return false;
                }
            }
        }

        // coinbase receiver
        if let Some(coinbase_receiver) = coinbase_receiver {
            if let Some(public_key) = coinbase_receiver.public_key.as_ref() {
                if block.block.coinbase_receiver.public_key != *public_key {
                    return false;
                }
            }
        }

        // conjunction
        if let Some(query) = and {
            if !query.iter().all(|and| and.matches(block)) {
                return false;
            }
        }

        // disjunction
        if let Some(query) = or {
            if !query.is_empty() && !query.iter().any(|or| or.matches(block)) {
                return false;
            }
        }
        true
    }
}

fn reorder(db: &Arc<IndexerStore>, blocks: &mut [Block], sort_by: BlockSortByInput) {
    use std::cmp::Ordering::{self, *};
    use BlockSortByInput::*;

    fn height_cmp(db: &Arc<IndexerStore>, a: &Block, b: &Block, cmp: Ordering) -> Ordering {
        if cmp == Equal {
            match (a.canonical, b.canonical) {
                (true, _) => Less,
                (_, true) => Greater,
                _ => db
                    .block_cmp(
                        &a.block.state_hash.clone().into(),
                        &b.block.state_hash.clone().into(),
                    )
                    .unwrap()
                    .unwrap(),
            }
        } else {
            cmp
        }
    }
    fn slot_cmp(db: &Arc<IndexerStore>, a: &Block, b: &Block, cmp: Ordering) -> Ordering {
        if cmp == Equal {
            match (a.canonical, b.canonical) {
                (true, _) => Less,
                (_, true) => Greater,
                _ => db
                    .block_cmp(
                        &a.block.state_hash.clone().into(),
                        &b.block.state_hash.clone().into(),
                    )
                    .unwrap()
                    .unwrap(),
            }
        } else {
            cmp
        }
    }
    match sort_by {
        BlockHeightAsc => blocks
            .sort_by(|a, b| height_cmp(db, a, b, a.block.block_height.cmp(&b.block.block_height))),
        BlockHeightDesc => blocks
            .sort_by(|a, b| height_cmp(db, a, b, b.block.block_height.cmp(&a.block.block_height))),
        GlobalSlotAsc => blocks.sort_by(|a, b| {
            slot_cmp(
                db,
                a,
                b,
                a.block
                    .global_slot_since_genesis
                    .cmp(&b.block.global_slot_since_genesis),
            )
        }),
        GlobalSlotDesc => blocks.sort_by(|a, b| {
            slot_cmp(
                db,
                a,
                b,
                b.block
                    .global_slot_since_genesis
                    .cmp(&a.block.global_slot_since_genesis),
            )
        }),
    }
}

impl Block {
    pub fn from_precomputed(
        db: &Arc<IndexerStore>,
        block: &PrecomputedBlock,
        counts: [u32; 13],
    ) -> Self {
        let state_hash = block.state_hash();
        let epoch_num_blocks = counts[0];
        let epoch_num_canonical_blocks = counts[1];
        let epoch_num_supercharged_blocks = counts[2];
        let total_num_blocks = counts[3];
        let total_num_canonical_blocks = counts[4];
        let total_num_supercharged_blocks = counts[5];
        let epoch_num_user_commands = counts[8];
        let total_num_user_commands = counts[9];
        let canonical = get_block_canonicity(db, &state_hash);
        let block_num_snarks = db
            .get_block_snarks_count(&state_hash)
            .expect("snark counts")
            .unwrap_or_default();
        let block_num_user_commands = db
            .get_block_user_commands_count(&state_hash)
            .expect("user command counts")
            .unwrap_or_default();
        let block_num_internal_commands = db
            .get_block_internal_commands_count(&state_hash)
            .expect("internal command counts")
            .unwrap_or_default();
        let epoch_num_slots_produced = counts[12];
        Self {
            canonical,
            epoch_num_blocks,
            epoch_num_canonical_blocks,
            epoch_num_supercharged_blocks,
            total_num_blocks,
            total_num_canonical_blocks,
            total_num_supercharged_blocks,
            block_num_snarks,
            block_num_user_commands,
            block_num_internal_commands,
            block: BlockWithoutCanonicity::new(
                block,
                canonical,
                epoch_num_user_commands,
                total_num_user_commands,
            ),
            epoch_num_slots_produced,
            num_unique_block_producers_last_n_blocks: None,
        }
    }
}

pub async fn get_counts(db: &Arc<IndexerStore>) -> Result<[u32; 13]> {
    let epoch_num_blocks = db.get_block_production_epoch_count(None)?;
    let epoch_num_canonical_blocks = db.get_block_production_canonical_epoch_count(None)?;
    let epoch_num_supercharged_blocks = db.get_block_production_supercharged_epoch_count(None)?;
    let total_num_blocks = db.get_block_production_total_count()?;
    let total_num_canonical_blocks = db.get_block_production_canonical_total_count()?;
    let total_num_supercharged_blocks = db.get_block_production_supercharged_total_count()?;
    let epoch_num_snarks = db.get_snarks_epoch_count(None).expect("epoch SNARK count");
    let total_num_snarks = db.get_snarks_total_count().expect("total SNARK count");
    let epoch_num_user_commands = db
        .get_user_commands_epoch_count(None)
        .expect("epoch user command count");
    let total_num_user_commands = db
        .get_user_commands_total_count()
        .expect("total user command count");
    let epoch_num_internal_commands = db
        .get_internal_commands_epoch_count(None)
        .expect("epoch internal command count");
    let total_num_internal_commands = db
        .get_internal_commands_total_count()
        .expect("total internal command count");
    let epoch_num_slots_produced = db.get_epoch_slots_produced_count(None)?;

    Ok([
        epoch_num_blocks,
        epoch_num_canonical_blocks,
        epoch_num_supercharged_blocks,
        total_num_blocks,
        total_num_canonical_blocks,
        total_num_supercharged_blocks,
        epoch_num_snarks,
        total_num_snarks,
        epoch_num_user_commands,
        total_num_user_commands,
        epoch_num_internal_commands,
        total_num_internal_commands,
        epoch_num_slots_produced,
    ])
}

impl From<DbInternalCommandWithData> for BlockFeetransfer {
    fn from(int_cmd: DbInternalCommandWithData) -> Self {
        match int_cmd {
            DbInternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount.to_string(),
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
            DbInternalCommandWithData::Coinbase {
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
