//! GraphQL `block` & `blocks` endpoint

pub mod block;

use super::{db, get_block_canonicity, millis_to_iso_date_string, pk::PK};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    snark_work::store::SnarkStore,
    store::IndexerStore,
    utility::store::common::{block_u32_prefix_from_key, state_hash_suffix, U32_LEN},
    web::{
        common::unique_block_producers_last_n_blocks,
        graphql::{
            gen::{BlockProtocolStateConsensusStateQueryInput, BlockQueryInput},
            get_block,
        },
    },
};
use async_graphql::{self, Context, Object, Result};
use block::{Block, BlockSortByInput};
use std::sync::Arc;

#[derive(Default)]
pub struct BlocksQueryRoot;

//////////
// impl //
//////////

#[Object]
impl BlocksQueryRoot {
    #[graphql(cache_control(max_age = 3600))]
    async fn block(
        &self,
        ctx: &Context<'_>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<Block>> {
        let db = db(ctx);

        let epoch = query.as_ref().and_then(|q| {
            q.protocol_state
                .as_ref()
                .and_then(|ps| ps.consensus_state.as_ref().and_then(|cs| cs.epoch))
        });
        let genesis_state_hash = query
            .as_ref()
            .and_then(|q| q.genesis_state_hash.clone())
            .map(Into::into);

        // no query filters => get the best block
        if query.is_none() {
            let counts = get_counts(db, epoch, genesis_state_hash.as_ref())?;

            return Ok(db
                .get_best_block()
                .map(|b| b.map(|pcb| Block::from_precomputed(db, &pcb, counts)))?);
        }

        // Use constant time access if we have the state hash
        if let Some(state_hash) = query.as_ref().and_then(|input| input.state_hash.as_ref()) {
            // validate state hash
            let state_hash = match StateHash::new(state_hash) {
                Ok(state_hash) => state_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid state hash: {}",
                        state_hash
                    )))
                }
            };

            let pcb = match db.get_block(&state_hash)? {
                Some((pcb, _)) => pcb,
                None => return Ok(None),
            };
            let block = Block::from_precomputed(
                db,
                &pcb,
                get_counts(db, epoch, genesis_state_hash.as_ref())?,
            );

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
            let block = Block::from_precomputed(
                db,
                &pcb,
                get_counts(db, epoch, genesis_state_hash.as_ref())?,
            );

            if query.as_ref().is_none_or(|q| q.matches(&block)) {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }

    #[allow(clippy::too_many_lines)]
    #[graphql(cache_control(max_age = 3600))]
    async fn blocks(
        &self,
        ctx: &Context<'_>,
        query: Option<BlockQueryInput>,
        #[graphql(default = 100)] limit: usize,
        sort_by: Option<BlockSortByInput>,
    ) -> Result<Vec<Block>> {
        use speedb::{Direction::*, IteratorMode::*};
        use BlockSortByInput::*;
        let db = db(ctx);

        // unique block producer query
        if let Some(num_blocks) = query
            .as_ref()
            .and_then(|q| q.unique_block_producers_last_n_blocks)
        {
            return match unique_block_producers_last_n_blocks(db, num_blocks) {
                Ok(num_unique_block_producers_last_n_blocks) => Ok(vec![Block {
                    num_unique_block_producers_last_n_blocks,
                    ..Default::default()
                }]),
                Err(e) => Err(async_graphql::Error::new(e.to_string())),
            };
        }

        let epoch = query.as_ref().and_then(|q| {
            q.protocol_state
                .as_ref()
                .and_then(|ps| ps.consensus_state.as_ref().and_then(|cs| cs.epoch))
        });
        let genesis_state_hash = query
            .as_ref()
            .and_then(|q| q.genesis_state_hash.clone())
            .map(Into::into);

        let counts = get_counts(db, epoch, genesis_state_hash.as_ref())?;
        let mut blocks = Vec::new();
        let sort_by = sort_by.unwrap_or(BlockHeightDesc);

        // state hash query
        if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.as_ref()) {
            // validate state hash
            let state_hash = match StateHash::new(state_hash) {
                Ok(state_hash) => state_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid state hash: {}",
                        state_hash
                    )))
                }
            };

            let block = db.get_block(&state_hash)?;
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
                .and_then(|cb| cb.public_key.as_ref())
        }) {
            // validate coinbase receiver
            let coinbase_receiver = match PublicKey::new(coinbase_receiver) {
                Ok(coinbase_receiver) => coinbase_receiver,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid coinbase receiver public key: {}",
                        coinbase_receiver
                    )))
                }
            };

            let start = coinbase_receiver.0.as_bytes();
            let mut end = [0; PublicKey::LEN + U32_LEN];
            end[..PublicKey::LEN].copy_from_slice(start);
            end[PublicKey::LEN..].copy_from_slice(&u32::MAX.to_be_bytes());

            let iter = match sort_by {
                BlockHeightAsc | GlobalSlotAsc => {
                    db.coinbase_receiver_block_height_iterator(From(start, Forward))
                }
                BlockHeightDesc | GlobalSlotDesc => {
                    db.coinbase_receiver_block_height_iterator(From(&end, Reverse))
                }
            };

            for (key, _) in iter.flatten() {
                if key[..PublicKey::LEN] != *coinbase_receiver.0.as_bytes() {
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
        if let Some(creator) = query.as_ref().and_then(|q| {
            q.creator_account
                .as_ref()
                .and_then(|cb| cb.public_key.as_ref())
        }) {
            // validate creator
            let creator = match PublicKey::new(creator) {
                Ok(creator) => creator,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid creator public key: {}",
                        creator
                    )))
                }
            };

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

            let start = creator.0.as_bytes();
            let mut end = [0; PublicKey::LEN + U32_LEN];
            end[..PublicKey::LEN].copy_from_slice(start);
            end[PublicKey::LEN..].copy_from_slice(&upper_bound.to_be_bytes());

            let iter = match sort_by {
                BlockHeightAsc | GlobalSlotAsc => {
                    db.block_creator_block_height_iterator(From(start, Forward))
                }
                BlockHeightDesc | GlobalSlotDesc => {
                    db.block_creator_block_height_iterator(From(&end, Reverse))
                }
            };

            for (key, _) in iter.flatten() {
                if key[..PublicKey::LEN] != *creator.0.as_bytes() {
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
        if query.as_ref().is_some_and(|q| {
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

            // min/max block height BE bytes & iterator mode
            let start = min.to_be_bytes();
            let end = (max + 1).to_be_bytes();
            let mode = match sort_by {
                BlockHeightAsc => From(&start, Forward),
                _ => From(&end, Reverse),
            };

            for (key, _) in db.blocks_height_iterator(mode).flatten() {
                let height = block_u32_prefix_from_key(&key)?;

                // out of bounds
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
        if consensus_state.is_some_and(|q| {
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

            // min/max global slot BE bytes & iterator mode
            let start = min.to_be_bytes();
            let end = (max + 1).to_be_bytes();
            let mode = match sort_by {
                GlobalSlotAsc => From(&start, Forward),
                _ => From(&end, Reverse),
            };

            for (key, _) in db.blocks_global_slot_iterator(mode).flatten() {
                let slot = block_u32_prefix_from_key(&key)?;

                // out of bounds
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
            genesis_state_hash,
            block_stake_winner,
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            protocol_state,
            unique_block_producers_last_n_blocks: _,
        } = self;
        // canonical
        if let Some(canonical) = canonical {
            if block.canonical != *canonical {
                return false;
            }
        }

        // state hash
        if let Some(state_hash) = state_hash {
            if block.block.state_hash != *state_hash {
                return false;
            }
        }

        // blockchain length
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
            .is_some_and(|global_slot| {
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

        // slot
        if protocol_state
            .as_ref()
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.slot)
            .is_some_and(|slot| block.block.protocol_state.consensus_state.slot != slot)
        {
            return false;
        }

        // epoch
        if protocol_state
            .as_ref()
            .and_then(|protocol_state| protocol_state.consensus_state.as_ref())
            .and_then(|consensus_state| consensus_state.epoch)
            .is_some_and(|epoch| block.block.protocol_state.consensus_state.epoch != epoch)
        {
            return false;
        }

        // genesis state hash
        if let Some(genesis_state_hash) = genesis_state_hash {
            if block.block.genesis_state_hash != *genesis_state_hash {
                return false;
            }
        }

        // block stake winner
        if let Some(block_stake_winner) = block_stake_winner {
            if block.block.winner_account.public_key != *block_stake_winner {
                return false;
            }
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
                if block.block.creator.creator != *public_key {
                    return false;
                }
            }
        }

        // coinbase receiver
        if let Some(coinbase_receiver) = coinbase_receiver {
            if let Some(public_key) = coinbase_receiver.public_key.as_ref() {
                if block.block.coinbase_receiver.coinbase_receiver != *public_key {
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

/////////////
// helpers //
/////////////

use std::cmp::Ordering::{self, *};

fn reorder(db: &Arc<IndexerStore>, blocks: &mut [Block], sort_by: BlockSortByInput) {
    use BlockSortByInput::*;

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

fn precomputed_matches_query(
    db: &Arc<IndexerStore>,
    query: &Option<BlockQueryInput>,
    block: &PrecomputedBlock,
    counts: [u32; 15],
) -> Option<Block> {
    let block_with_canonicity = Block::from_precomputed(db, block, counts);
    if query
        .as_ref()
        .is_none_or(|q| q.matches(&block_with_canonicity))
    {
        Some(block_with_canonicity)
    } else {
        None
    }
}

pub fn get_counts(
    db: &Arc<IndexerStore>,
    epoch: Option<u32>,
    genesis_state_hash: Option<&StateHash>,
) -> Result<[u32; 15]> {
    let epoch_num_blocks = db.get_block_production_epoch_count(epoch, genesis_state_hash)?;
    let total_num_blocks = db.get_block_production_total_count()?;

    let epoch_num_canonical_blocks =
        db.get_block_production_canonical_epoch_count(epoch, genesis_state_hash)?;
    let total_num_canonical_blocks = db.get_block_production_canonical_total_count()?;

    let epoch_num_supercharged_blocks =
        db.get_block_production_supercharged_epoch_count(epoch, genesis_state_hash)?;
    let total_num_supercharged_blocks = db.get_block_production_supercharged_total_count()?;

    let epoch_num_snarks = db
        .get_snarks_epoch_count(epoch, genesis_state_hash)
        .expect("epoch SNARK count");
    let total_num_snarks = db.get_snarks_total_count().expect("total SNARK count");

    let epoch_num_user_commands = db
        .get_user_commands_epoch_count(epoch, genesis_state_hash)
        .expect("epoch user command count");
    let total_num_user_commands = db
        .get_user_commands_total_count()
        .expect("total user command count");

    let epoch_num_zkapp_commands = db
        .get_zkapp_commands_epoch_count(epoch, genesis_state_hash)
        .expect("epoch zkapp command count");
    let total_num_zkapp_commands = db
        .get_zkapp_commands_total_count()
        .expect("total zkapp command count");

    let epoch_num_internal_commands = db
        .get_internal_commands_epoch_count(epoch, genesis_state_hash)
        .expect("epoch internal command count");
    let total_num_internal_commands = db
        .get_internal_commands_total_count()
        .expect("total internal command count");

    let epoch_num_slots_produced = db.get_epoch_slots_produced_count(epoch, genesis_state_hash)?;

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
        epoch_num_zkapp_commands,
        total_num_zkapp_commands,
    ])
}
