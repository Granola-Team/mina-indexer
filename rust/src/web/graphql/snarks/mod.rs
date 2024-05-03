use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    ledger::public_key::PublicKey,
    snark_work::{store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash},
    store::{
        blocks_global_slot_idx_iterator, blocks_global_slot_idx_state_hash_from_entry, IndexerStore,
    },
    web::graphql::{db, get_block_canonicity},
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(SimpleObject, Debug)]
pub struct Snark {
    pub fee: u64,
    pub prover: String,
}

#[derive(SimpleObject, Debug)]
#[graphql(complex)]
pub struct SnarkWithCanonicity {
    /// Value canonicity
    pub canonical: bool,
    /// Value optional block
    #[graphql(skip)]
    pub block: PrecomputedBlock,
    /// Value snark
    #[graphql(flatten)]
    pub snark: Snark,
}

#[ComplexObject]
impl SnarkWithCanonicity {
    /// Value state hash
    async fn state_hash(&self) -> String {
        self.block.state_hash().0.to_owned()
    }
    /// Value block height
    async fn block_height(&self) -> u32 {
        self.block.blockchain_length()
    }
    /// Value date time
    async fn date_time(&self) -> String {
        millis_to_iso_date_string(self.block.timestamp() as i64)
    }
}

#[derive(InputObject, Clone)]
pub struct SnarkQueryInput {
    state_hash: Option<String>,
    canonical: Option<bool>,
    prover: Option<String>,
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
        for entry in blocks_global_slot_idx_iterator(db, mode) {
            let state_hash = blocks_global_slot_idx_state_hash_from_entry(&entry)?;
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
                        block: block.clone(),
                        snark: snark.into(),
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
                block,
                canonical,
                snark: snark.into(),
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

impl From<SnarkWorkSummary> for Snark {
    fn from(snark: SnarkWorkSummary) -> Self {
        Snark {
            fee: snark.fee,
            prover: snark.prover.0,
        }
    }
}

impl From<SnarkWorkSummaryWithStateHash> for Snark {
    fn from(snark: SnarkWorkSummaryWithStateHash) -> Self {
        Snark {
            fee: snark.fee,
            prover: snark.prover.0,
        }
    }
}

impl SnarkQueryInput {
    pub fn matches(&self, snark: &SnarkWithCanonicity) -> bool {
        let mut matches = true;
        let Self {
            state_hash,
            canonical,
            prover,
            and,
            or,
        } = self;

        if let Some(state_hash) = state_hash {
            matches &= snark.block.state_hash().0 == *state_hash;
        }
        if let Some(prover) = prover {
            matches &= snark
                .block
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
