use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    ledger::public_key::PublicKey,
    snark_work::{store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash},
    store::{blocks_global_slot_idx_state_hash_from_key, from_be_bytes, to_be_bytes, IndexerStore},
    web::graphql::{db, gen::BlockQueryInput, get_block_canonicity},
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(SimpleObject, Debug)]
pub struct Snark {
    pub fee: u64,
    pub prover: String,
    pub block: SnarkBlock,

    #[graphql(name = "epoch_num_snarks")]
    epoch_num_snarks: u32,

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

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SnarkSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(Default)]
pub struct SnarkQueryRoot;

#[Object]
impl SnarkQueryRoot {
    async fn snarks<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<SnarkQueryInput>,
        sort_by: Option<SnarkSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<SnarkWithCanonicity>> {
        let db = db(ctx);
        let mut snarks = Vec::with_capacity(limit);
        let sort_by = sort_by.unwrap_or(SnarkSortByInput::BlockHeightDesc);

        // state hash
        if let Some(state_hash) = query
            .as_ref()
            .and_then(|q| q.block.as_ref())
            .and_then(|block| block.state_hash.clone())
        {
            let mut snarks: Vec<SnarkWithCanonicity> = db
                .get_block(&state_hash.into())?
                .into_iter()
                .flat_map(|block| {
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

        // block height
        if let Some(block_height) = query.as_ref().and_then(|q| q.block_height) {
            let mut snarks: Vec<SnarkWithCanonicity> = db
                .get_blocks_at_height(block_height)?
                .into_iter()
                .flat_map(|block| {
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

        // prover query
        if let Some(prover) = query.as_ref().and_then(|q| q.prover.clone()) {
            let mut start = prover.as_bytes().to_vec();
            let mode = match sort_by {
                SnarkSortByInput::BlockHeightAsc => {
                    speedb::IteratorMode::From(&start, speedb::Direction::Forward)
                }
                SnarkSortByInput::BlockHeightDesc => {
                    let mut pk_prefix = PublicKey::PREFIX.as_bytes().to_vec();
                    *pk_prefix.last_mut().unwrap_or(&mut 0) += 1;
                    start.append(&mut to_be_bytes(u32::MAX));
                    start.append(&mut pk_prefix);
                    speedb::IteratorMode::From(&start, speedb::Direction::Reverse)
                }
            };

            let mut snarks = Vec::with_capacity(limit);
            for (key, snark) in db.snark_prover_iterator(mode).flatten() {
                if key[..PublicKey::LEN] != *prover.as_bytes() {
                    return Ok(snarks);
                }

                let global_slot = from_be_bytes(key[PublicKey::LEN..PublicKey::LEN + 4].to_vec());
                let blocks_at_slot = db.get_blocks_at_slot(global_slot)?;
                let pcb = blocks_at_slot[0].clone();
                let state_hash = pcb.state_hash().0;
                let canonical = get_block_canonicity(db, &pcb.state_hash().0);
                let snark = serde_json::from_slice(&snark)?;
                let sw = SnarkWithCanonicity {
                    canonical,
                    pcb,
                    snark: (
                        snark,
                        state_hash,
                        db.get_snarks_epoch_count(None).expect("epoch snarks count"),
                        db.get_snarks_total_count().expect("total snarks count"),
                    )
                        .into(),
                };
                if query.as_ref().map_or(true, |q| q.matches(&sw)) {
                    snarks.push(sw);

                    if snarks.len() == limit {
                        break;
                    }
                }
            }
            return Ok(snarks);
        }

        // block height bounded query
        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let mut snarks: Vec<SnarkWithCanonicity> = Vec::with_capacity(limit);
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
                    (None, None) => db.get_best_block()?.unwrap().blockchain_length(),
                };
                (min_bound, max_bound)
            };

            let mut block_heights: Vec<u32> = (min..=max).collect();
            if sort_by == SnarkSortByInput::BlockHeightDesc {
                block_heights.reverse()
            }

            'outer: for height in block_heights {
                for block in db.get_blocks_at_height(height)? {
                    let state_hash = block.state_hash().0;
                    let canonical = get_block_canonicity(db, &state_hash);
                    let snark_work = db.get_snark_work_in_block(&state_hash.clone().into())?;
                    let snarks_with_canonicity = snark_work.map_or(vec![], |summaries| {
                        summaries
                            .into_iter()
                            .map(|snark| SnarkWithCanonicity {
                                canonical,
                                pcb: block.clone(),
                                snark: (
                                    snark,
                                    state_hash.clone(),
                                    db.get_snarks_epoch_count(None).expect("epoch snarks count"),
                                    db.get_snarks_total_count().expect("total snarks count"),
                                )
                                    .into(),
                            })
                            .collect()
                    });

                    for sw in snarks_with_canonicity {
                        if query.as_ref().map_or(true, |q| q.matches(&sw)) {
                            snarks.push(sw);

                            if snarks.len() == limit {
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

        'outer: for (key, _) in db.blocks_global_slot_idx_iterator(mode).flatten() {
            let state_hash = blocks_global_slot_idx_state_hash_from_key(&key)?;
            let block = db
                .get_block(&state_hash.clone().into())?
                .expect("block to be returned");
            let canonical = get_block_canonicity(db, &state_hash);
            let snark_work = db.get_snark_work_in_block(&state_hash.clone().into())?;
            let snarks_with_canonicity = snark_work.map_or(vec![], |summaries| {
                summaries
                    .into_iter()
                    .map(|snark| SnarkWithCanonicity {
                        canonical,
                        pcb: block.clone(),
                        snark: (
                            snark,
                            state_hash.clone(),
                            db.get_snarks_epoch_count(None).expect("epoch snarks count"),
                            db.get_snarks_total_count().expect("total snarks count"),
                        )
                            .into(),
                    })
                    .collect()
            });

            for sw in snarks_with_canonicity {
                if query.as_ref().map_or(true, |q| q.matches(&sw)) {
                    snarks.push(sw);

                    if snarks.len() == limit {
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
    Ok(db
        .get_block(&snark.state_hash.clone().into())?
        .and_then(|block| {
            let snark_with_canonicity = SnarkWithCanonicity {
                pcb: block,
                canonical,
                snark: (
                    snark,
                    db.get_snarks_epoch_count(None).expect("epoch snarks count"),
                    db.get_snarks_total_count().expect("total snarks count"),
                )
                    .into(),
            };
            if query
                .as_ref()
                .map_or(true, |q| q.matches(&snark_with_canonicity))
            {
                Some(snark_with_canonicity)
            } else {
                None
            }
        }))
}

impl From<(SnarkWorkSummary, String, u32, u32)> for Snark {
    fn from(snark: (SnarkWorkSummary, String, u32, u32)) -> Self {
        Snark {
            fee: snark.0.fee,
            prover: snark.0.prover.0,
            block: SnarkBlock {
                state_hash: snark.1,
            },
            epoch_num_snarks: snark.2,
            total_num_snarks: snark.3,
        }
    }
}

impl From<(SnarkWorkSummaryWithStateHash, u32, u32)> for Snark {
    fn from(snark: (SnarkWorkSummaryWithStateHash, u32, u32)) -> Self {
        Snark {
            fee: snark.0.fee,
            prover: snark.0.prover.0,
            block: SnarkBlock {
                state_hash: snark.0.state_hash.clone(),
            },
            epoch_num_snarks: snark.1,
            total_num_snarks: snark.2,
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

        // block_height_gt(e) & block_height_lt(e)
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

        if let Some(block_query_input) = block {
            if let Some(state_hash) = &block_query_input.state_hash {
                if snark.pcb.state_hash().0 != *state_hash {
                    return false;
                }
            }
        }
        if let Some(block_height) = block_height {
            if snark.pcb.blockchain_length() != *block_height {
                return false;
            }
        }
        if let Some(prover) = prover {
            if !snark
                .pcb
                .prover_keys()
                .contains(&<String as Into<PublicKey>>::into(prover.clone()))
            {
                return false;
            }
        }
        if let Some(canonical) = canonical {
            if snark.canonical != *canonical {
                return false;
            }
        }
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
