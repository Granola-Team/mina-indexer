use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    snark_work::{store::SnarkStore, SnarkWorkSummary},
    store::{blocks_global_slot_idx_iterator, blocks_global_slot_idx_state_hash_from_entry},
    web::graphql::{db, get_block_canonicity},
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};

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
        let mode: speedb::IteratorMode = if let Some(SnarkSortByInput::BlockHeightAsc) = sort_by {
            speedb::IteratorMode::Start
        } else {
            speedb::IteratorMode::End
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
            matches = matches && &snark.block.state_hash().0 == state_hash;
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
