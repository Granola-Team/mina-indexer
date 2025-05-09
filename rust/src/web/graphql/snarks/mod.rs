//! GraphQL `snarks` endpoint

use super::{db, gen::BlockQueryInput, get_block, get_block_canonicity, pk::ProverPK};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    snark_work::{store::SnarkStore, SnarkWorkSummaryWithStateHash},
    store::IndexerStore,
    utility::store::common::{from_be_bytes, state_hash_suffix, U32_LEN},
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(SimpleObject, Debug)]
pub struct Snark {
    /// Value SNARK fee (nanomina)
    pub fee: u64,

    /// Value SNARK prover
    #[graphql(flatten)]
    pub prover: ProverPK,

    /// Value SNARK block
    pub block: SnarkBlock,

    /// Value epoch SNARKs count
    #[graphql(name = "epoch_num_snarks")]
    epoch_num_snarks: u32,

    /// Value total SNARKs count
    #[graphql(name = "total_num_snarks")]
    total_num_snarks: u32,
}

#[derive(SimpleObject, Debug)]
pub struct SnarkBlock {
    pub state_hash: String,
}

#[derive(SimpleObject, Debug)]
#[graphql(complex)]
pub struct SnarkWithCanonicity {
    /// Value canonicity
    pub canonical: bool,

    /// Value optional block
    #[graphql(skip)]
    pub pcb: PrecomputedBlock,

    /// Value snark
    #[graphql(flatten)]
    pub snark: Snark,
}

#[ComplexObject]
impl SnarkWithCanonicity {
    /// Value state hash
    async fn state_hash(&self) -> String {
        self.pcb.state_hash().0.to_owned()
    }

    /// Value block height
    async fn block_height(&self) -> u32 {
        self.pcb.blockchain_length()
    }

    /// Value date time
    async fn date_time(&self) -> String {
        millis_to_iso_date_string(self.pcb.timestamp() as i64)
    }
}

#[derive(InputObject)]
pub struct SnarkQueryInput {
    canonical: Option<bool>,
    prover: Option<String>,
    block_height: Option<u32>,
    block: Option<BlockQueryInput>,

    #[graphql(name = "blockHeight_gt")]
    block_height_gt: Option<u32>,

    #[graphql(name = "blockHeight_gte")]
    block_height_gte: Option<u32>,

    #[graphql(name = "blockHeight_lt")]
    block_height_lt: Option<u32>,

    #[graphql(name = "blockHeight_lte")]
    block_height_lte: Option<u32>,

    and: Option<Vec<SnarkQueryInput>>,
    or: Option<Vec<SnarkQueryInput>>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum SnarkSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[default]
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(Default)]
pub struct SnarkQueryRoot;

///////////
// impls //
///////////

#[Object]
impl SnarkQueryRoot {
    #[allow(clippy::too_many_lines)]
    #[graphql(cache_control(max_age = 3600))]
    async fn snarks(
        &self,
        ctx: &Context<'_>,
        query: Option<SnarkQueryInput>,
        sort_by: Option<SnarkSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<SnarkWithCanonicity>> {
        let db = db(ctx);
        let mut snarks = <Vec<SnarkWithCanonicity>>::new();
        let sort_by = sort_by.unwrap_or(SnarkSortByInput::BlockHeightDesc);

        // state hash
        if let Some(state_hash) = query
            .as_ref()
            .and_then(|q| q.block.as_ref())
            .and_then(|block| block.state_hash.as_ref())
        {
            // validate state hash
            if !StateHash::is_valid(state_hash) {
                return Err(async_graphql::Error::new(format!(
                    "Invalid state hash: {}",
                    state_hash
                )));
            }

            let state_hash: StateHash = state_hash.clone().into();
            if let Some(block_snarks) = db.get_block_snark_work(&state_hash)? {
                snarks = block_snarks
                    .into_iter()
                    .flat_map(|snark| {
                        snark_summary_matches_query(
                            db,
                            &query,
                            SnarkWorkSummaryWithStateHash {
                                fee: snark.fee,
                                prover: snark.prover,
                                state_hash: state_hash.clone(),
                            },
                        )
                        .ok()
                        .flatten()
                    })
                    .collect();
            }

            match sort_by {
                SnarkSortByInput::BlockHeightAsc => snarks.reverse(),
                SnarkSortByInput::BlockHeightDesc => (),
            }

            snarks.truncate(limit);
            return Ok(snarks);
        }

        // block height
        if let Some(block_height) = query.as_ref().and_then(|q| q.block_height) {
            let mut snarks: Vec<SnarkWithCanonicity> = db
                .get_blocks_at_height(block_height)?
                .iter()
                .flat_map(|state_hash| {
                    let block = get_block(db, state_hash);
                    SnarkWorkSummaryWithStateHash::from_precomputed(&block)
                        .into_iter()
                        .filter_map(|s| snark_summary_matches_query(db, &query, s).ok().flatten())
                        .collect::<Vec<SnarkWithCanonicity>>()
                })
                .collect();

            match sort_by {
                SnarkSortByInput::BlockHeightAsc => snarks.reverse(),
                SnarkSortByInput::BlockHeightDesc => (),
            }

            snarks.truncate(limit);
            return Ok(snarks);
        }

        // prover query filter and sort by height
        if let (Some(prover), Some(block_height_lte)) = (
            query.as_ref().and_then(|q| q.prover.clone()),
            query.as_ref().and_then(|q| q.block_height_lte),
        ) {
            // validate prover pk
            if let Ok(prover) = PublicKey::new(prover.to_owned()) {
                let mut start = prover.0.as_bytes().to_vec();
                let mode = match sort_by {
                    SnarkSortByInput::BlockHeightAsc => {
                        speedb::IteratorMode::From(&start, speedb::Direction::Forward)
                    }
                    SnarkSortByInput::BlockHeightDesc => {
                        start.append(&mut block_height_lte.to_be_bytes().to_vec());
                        start.append(&mut u32::MAX.to_be_bytes().to_vec());
                        speedb::IteratorMode::From(&start, speedb::Direction::Reverse)
                    }
                };

                'outer: for (key, snark) in db.snark_prover_block_height_iterator(mode).flatten() {
                    // exit if prover isn't the same
                    if key[..PublicKey::LEN] != *prover.0.as_bytes() {
                        break;
                    }

                    let block_height = from_be_bytes(key[PublicKey::LEN..][..U32_LEN].to_vec());
                    let blocks_at_height = db.get_blocks_at_height(block_height)?;

                    for state_hash in blocks_at_height {
                        // avoid deserializing PCB if possible
                        let canonical = get_block_canonicity(db, &state_hash);
                        if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                            if canonical != query_canonicity {
                                continue;
                            }
                        }

                        let pcb = get_block(db, &state_hash);
                        let snark = serde_json::from_slice(&snark)?;
                        let sw = SnarkWithCanonicity {
                            canonical,
                            pcb,
                            snark: Snark::new(
                                db,
                                SnarkWorkSummaryWithStateHash::from(snark, state_hash),
                                db.get_snarks_epoch_count(None, None)
                                    .expect("epoch snarks count"),
                                db.get_snarks_total_count().expect("total snarks count"),
                            ),
                        };

                        if query.as_ref().is_none_or(|q| q.matches(&sw)) {
                            snarks.push(sw);

                            if snarks.len() >= limit {
                                break 'outer;
                            }
                        }
                    }
                }
            } else {
                return Err(async_graphql::Error::new(format!(
                    "Invalid prover public key: {}",
                    prover
                )));
            }

            return Ok(snarks);
        }

        // prover query
        if let Some(prover) = query.as_ref().and_then(|q| q.prover.clone()) {
            // validate prover pk
            if let Ok(prover) = PublicKey::new(prover.to_owned()) {
                let mut start = prover.0.as_bytes().to_vec();

                let mode = match sort_by {
                    SnarkSortByInput::BlockHeightAsc => {
                        speedb::IteratorMode::From(&start, speedb::Direction::Forward)
                    }
                    SnarkSortByInput::BlockHeightDesc => {
                        let mut pk_prefix = PublicKey::PREFIX.as_bytes().to_vec();

                        *pk_prefix.last_mut().unwrap_or(&mut 0) += 1;
                        start.append(&mut u32::MAX.to_be_bytes().to_vec());
                        start.append(&mut pk_prefix);

                        speedb::IteratorMode::From(&start, speedb::Direction::Reverse)
                    }
                };

                'outer: for (key, snark) in db.snark_prover_block_height_iterator(mode).flatten() {
                    if key[..PublicKey::LEN] != *prover.0.as_bytes() {
                        break;
                    }

                    let block_height = from_be_bytes(key[PublicKey::LEN..][..U32_LEN].to_vec());
                    let blocks_at_slot = db.get_blocks_at_height(block_height)?;

                    for state_hash in blocks_at_slot {
                        // avoid deserializing PCB if possible
                        let canonical = get_block_canonicity(db, &state_hash);
                        if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                            if canonical != query_canonicity {
                                continue;
                            }
                        }

                        let pcb = get_block(db, &state_hash);
                        let snark = serde_json::from_slice(&snark)?;
                        let sw = SnarkWithCanonicity {
                            canonical,
                            pcb,
                            snark: Snark::new(
                                db,
                                SnarkWorkSummaryWithStateHash::from(snark, state_hash),
                                db.get_snarks_epoch_count(None, None)
                                    .expect("epoch snarks count"),
                                db.get_snarks_total_count().expect("total snarks count"),
                            ),
                        };

                        if query.as_ref().is_none_or(|q| q.matches(&sw)) {
                            snarks.push(sw);

                            if snarks.len() >= limit {
                                break 'outer;
                            }
                        }
                    }
                }
            } else {
                return Err(async_graphql::Error::new(format!(
                    "Invalid prover public key: {}",
                    prover
                )));
            }

            return Ok(snarks);
        }

        // block height bounded query
        if query.as_ref().is_some_and(|q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let (min, max) = {
                let SnarkQueryInput {
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

            let mut block_heights: Vec<u32> = (min..=max).collect();
            if sort_by == SnarkSortByInput::BlockHeightDesc {
                block_heights.reverse()
            }

            'outer: for height in block_heights {
                for state_hash in db.get_blocks_at_height(height)? {
                    // avoid deserializing PCB if possible
                    let canonical = get_block_canonicity(db, &state_hash);
                    if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                        if canonical != query_canonicity {
                            continue;
                        }
                    }

                    let block = get_block(db, &state_hash);
                    let snark_work = db.get_block_snark_work(&state_hash)?;
                    let snarks_with_canonicity = snark_work.map_or(vec![], |summaries| {
                        summaries
                            .into_iter()
                            .map(|snark| SnarkWithCanonicity {
                                canonical,
                                pcb: block.clone(),
                                snark: Snark::new(
                                    db,
                                    SnarkWorkSummaryWithStateHash::from(snark, state_hash.clone()),
                                    db.get_snarks_epoch_count(None, None)
                                        .expect("epoch snarks count"),
                                    db.get_snarks_total_count().expect("total snarks count"),
                                ),
                            })
                            .collect()
                    });

                    for sw in snarks_with_canonicity {
                        if query.as_ref().is_none_or(|q| q.matches(&sw)) {
                            snarks.push(sw);

                            if snarks.len() >= limit {
                                break 'outer;
                            }
                        }
                    }
                }
            }

            return Ok(snarks);
        }

        // general query
        let mode = match sort_by {
            SnarkSortByInput::BlockHeightAsc => speedb::IteratorMode::Start,
            SnarkSortByInput::BlockHeightDesc => speedb::IteratorMode::End,
        };

        'outer: for (key, _) in db.blocks_height_iterator(mode).flatten() {
            let state_hash = state_hash_suffix(&key)?;

            // avoid deserializing PCB if possible
            let canonical = get_block_canonicity(db, &state_hash);
            if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
                if canonical != query_canonicity {
                    continue;
                }
            }

            let snark_work = db.get_block_snark_work(&state_hash)?;
            let snarks_with_canonicity = snark_work.map_or(vec![], |summaries| {
                summaries
                    .into_iter()
                    .map(|snark| SnarkWithCanonicity {
                        canonical,
                        pcb: get_block(db, &state_hash),
                        snark: Snark::new(
                            db,
                            SnarkWorkSummaryWithStateHash::from(snark, state_hash.clone()),
                            db.get_snarks_epoch_count(None, None)
                                .expect("epoch snarks count"),
                            db.get_snarks_total_count().expect("total snarks count"),
                        ),
                    })
                    .collect()
            });

            for sw in snarks_with_canonicity {
                if query.as_ref().is_none_or(|q| q.matches(&sw)) {
                    snarks.push(sw);

                    if snarks.len() >= limit {
                        break 'outer;
                    }
                }
            }
        }

        Ok(snarks)
    }
}

fn snark_summary_matches_query(
    db: &Arc<IndexerStore>,
    query: &Option<SnarkQueryInput>,
    snark: SnarkWorkSummaryWithStateHash,
) -> anyhow::Result<Option<SnarkWithCanonicity>> {
    let canonical = get_block_canonicity(db, &snark.state_hash);
    let snark_with_canonicity = SnarkWithCanonicity {
        pcb: get_block(db, &snark.state_hash),
        canonical,
        snark: Snark::new(
            db,
            snark,
            db.get_snarks_epoch_count(None, None)
                .expect("epoch snarks count"),
            db.get_snarks_total_count().expect("total snarks count"),
        ),
    };

    if query
        .as_ref()
        .is_none_or(|q| q.matches(&snark_with_canonicity))
    {
        Ok(Some(snark_with_canonicity))
    } else {
        Ok(None)
    }
}

impl Snark {
    fn new(
        db: &Arc<IndexerStore>,
        snark: SnarkWorkSummaryWithStateHash,
        epoch_num_snarks: u32,
        total_num_snarks: u32,
    ) -> Self {
        Snark {
            fee: snark.fee.0,
            prover: ProverPK::new(db, snark.prover),
            block: SnarkBlock {
                state_hash: snark.state_hash.0,
            },
            epoch_num_snarks,
            total_num_snarks,
        }
    }
}

impl SnarkQueryInput {
    pub fn matches(&self, snark: &SnarkWithCanonicity) -> bool {
        let Self {
            block,
            canonical,
            prover,
            block_height,
            block_height_gt,
            block_height_lt,
            block_height_gte,
            block_height_lte,
            and,
            or,
        } = self;
        let blockchain_length = snark.pcb.blockchain_length();

        // block height
        if let Some(block_height) = block_height {
            if snark.pcb.blockchain_length() != *block_height {
                return false;
            }
        }

        if let Some(height) = block_height_gt {
            if blockchain_length <= *height {
                return false;
            }
        }

        if let Some(height) = block_height_gte {
            if blockchain_length < *height {
                return false;
            }
        }

        if let Some(height) = block_height_lt {
            if blockchain_length >= *height {
                return false;
            }
        }

        if let Some(height) = block_height_lte {
            if blockchain_length > *height {
                return false;
            }
        }

        // block
        if let Some(block_query_input) = block {
            if let Some(state_hash) = &block_query_input.state_hash {
                if snark.pcb.state_hash().0 != *state_hash {
                    return false;
                }
            }
        }

        // prover
        if let Some(prover) = prover {
            if !snark
                .pcb
                .prover_keys()
                .contains(&<String as Into<PublicKey>>::into(prover.clone()))
            {
                return false;
            }
        }

        // canonicity
        if let Some(canonical) = canonical {
            if snark.canonical != *canonical {
                return false;
            }
        }

        // boolean
        if let Some(query) = and {
            if !query.iter().all(|and| and.matches(snark)) {
                return false;
            }
        }

        if let Some(query) = or {
            if !query.is_empty() && !query.iter().any(|or| or.matches(snark)) {
                return false;
            }
        }

        true
    }
}
