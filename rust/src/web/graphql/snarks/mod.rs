use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    snark_work::{store::SnarkStore, SnarkWorkSummary},
    store::{blocks_global_slot_idx_iterator, IndexerStore},
    web::{graphql::db, millis_to_iso_date_string},
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
        self.block.state_hash.clone()
    }
    /// Value block height
    async fn block_height(&self) -> u32 {
        self.block.blockchain_length
    }
    /// Value date time
    async fn date_time(&self) -> String {
        millis_to_iso_date_string(self.block.timestamp().try_into().unwrap())
    }
}

#[derive(InputObject, Clone)]
pub struct SnarkQueryInput {
    state_hash: Option<String>,
    canonical: Option<bool>,
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
        limit: Option<usize>,
    ) -> Result<Option<Vec<SnarkWithCanonicity>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);

        let mut snarks = Vec::with_capacity(limit);

        let mode: speedb::IteratorMode = match sort_by {
            Some(SnarkSortByInput::BlockHeightAsc) => speedb::IteratorMode::Start,
            Some(SnarkSortByInput::BlockHeightDesc) => speedb::IteratorMode::End,
            None => speedb::IteratorMode::End,
        };
        let mut limit_reached = false;
        let iter = blocks_global_slot_idx_iterator(db, mode);
        for entry in iter {
            if limit_reached {
                break;
            }
            let (_, value) = entry?;
            let state_hash = String::from_utf8(value.into_vec()).expect("state hash");
            let block = db
                .get_block(&BlockHash::from(state_hash.clone()))?
                .expect("block to be returned");
            let canonical = get_block_canonicity(db, &state_hash)?;

            let snark_work = db.get_snark_work_in_block(&BlockHash::from(state_hash))?;
            let snarks_with_canonicity = snark_work.map_or(Vec::new(), |summaries| {
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
                    limit_reached = true;
                    break;
                }
            }
        }
        if let Some(sort_by) = sort_by {
            match sort_by {
                SnarkSortByInput::BlockHeightAsc => {
                    snarks
                        .sort_by(|a, b| a.block.blockchain_length.cmp(&b.block.blockchain_length));
                }
                SnarkSortByInput::BlockHeightDesc => {
                    snarks
                        .sort_by(|a, b| b.block.blockchain_length.cmp(&a.block.blockchain_length));
                }
            }
        }

        Ok(Some(snarks))
    }
}

fn get_block_canonicity(db: &Arc<IndexerStore>, state_hash: &str) -> Result<bool> {
    let get_block_canonicity = db.get_block_canonicity(&BlockHash::from(state_hash.to_owned()));
    let canonicity = get_block_canonicity?
        .map(|status| matches!(status, Canonicity::Canonical))
        .unwrap_or(false);
    Ok(canonicity)
}

impl From<SnarkWorkSummary> for Snark {
    fn from(snark: SnarkWorkSummary) -> Self {
        Snark {
            fee: snark.fee,
            prover: snark.prover.0,
        }
    }
}

impl SnarkQueryInput {
    pub fn matches(&self, snark: &SnarkWithCanonicity) -> bool {
        let mut matches = true;

        if let Some(state_hash) = &self.state_hash {
            matches = matches && &snark.block.state_hash == state_hash;
        }

        if let Some(canonical) = &self.canonical {
            matches = matches && &snark.canonical == canonical;
        }

        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(snark));
        }

        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(snark));
            }
        }
        matches
    }
}
