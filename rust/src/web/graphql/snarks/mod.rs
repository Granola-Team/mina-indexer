use super::gen::BlockQueryInput;
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    ledger::public_key::PublicKey,
    snark_work::{store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash},
    store::{
        blocks_global_slot_idx_iterator, blocks_global_slot_idx_state_hash_from_key, IndexerStore,
    },
    web::graphql::{db, get_block_canonicity},
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
            let mut snarks =
                db.get_snark_work_by_public_key(&prover.into())?
                    .map_or(vec![], |snarks| {
                        snarks
                            .into_iter()
                            .filter_map(|s| {
                                snark_summary_matches_query(db, &query, s).ok().flatten()
                            })
                            .collect()
                    });

            match sort_by {
                SnarkSortByInput::BlockHeightAsc => snarks.reverse(),
                SnarkSortByInput::BlockHeightDesc => (),
            }

            snarks.truncate(limit);
            return Ok(snarks);
        }

        // general query
        let mode = match sort_by {
            SnarkSortByInput::BlockHeightAsc => speedb::IteratorMode::Start,
            SnarkSortByInput::BlockHeightDesc => speedb::IteratorMode::End,
        };
        for entry in blocks_global_slot_idx_iterator(db, mode).flatten() {
            let state_hash = blocks_global_slot_idx_state_hash_from_key(&entry.0)?;
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
                }

                if snarks.len() == limit {
                    break;
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
        let mut matches = true;
        let Self {
            block,
            canonical,
            prover,
            block_height,
            and,
            or,
        } = self;

        if let Some(block_query_input) = block {
            if let Some(state_hash) = &block_query_input.state_hash {
                matches &= snark.pcb.state_hash().0 == *state_hash;
            }
        }
        if let Some(block_height) = block_height {
            matches &= snark.pcb.blockchain_length() == *block_height;
        }
        if let Some(prover) = prover {
            matches &= snark
                .pcb
                .prover_keys()
                .contains(&<String as Into<PublicKey>>::into(prover.clone()));
        }
        if let Some(canonical) = canonical {
            matches &= snark.canonical == *canonical;
        }
        if let Some(query) = and {
            matches &= query.iter().all(|and| and.matches(snark));
        }
        if let Some(query) = or {
            if !query.is_empty() {
                matches &= query.iter().any(|or| or.matches(snark));
            }
        }
        matches
    }
}
