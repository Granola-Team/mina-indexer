//! GraphQL `topSnarkers` endpoint

use super::{db, pk::PK_};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash, username::Username},
    block::store::BlockStore,
    snark_work::store::SnarkStore,
    store::IndexerStore,
    utility::store::common::{u64_from_be_bytes, U32_LEN, U64_LEN},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::{DBIterator, Direction, IteratorMode};
use std::sync::Arc;

#[derive(InputObject)]
pub struct TopSnarkersQueryInput {
    /// Input epoch
    epoch: Option<u32>,

    /// Input genesis state hash
    #[graphql(name = "genesis_state_hash")]
    genesis_state_hash: Option<String>,

    /// Input SNARK prover public key
    #[graphql(name = "public_key")]
    public_key: Option<String>,

    /// Input SNARK prover username
    username: Option<String>,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq)]
pub enum TopSnarkersSortByInput {
    #[default]
    /// Sort by epoch total fees descending
    TotalFeesDesc,
    /// Sort by epoch total fees ascending
    TotalFeesAsc,

    /// Sort by epoch max fee descending
    MaxFeeDesc,
    /// Sort by epoch max fee ascending
    MaxFeeAsc,

    /// Sort by all-time total fees descending
    AllTimeTotalFeesDesc,
    /// Sort by all-time total fees ascending
    AllTimeTotalFeesAsc,

    /// Sort by all-time max fee descending
    AllTimeMaxFeeDesc,
    /// Sort by all-time max fee ascending
    AllTimeMaxFeeAsc,
}

#[derive(Default)]
pub struct TopSnarkersQueryRoot;

#[derive(SimpleObject)]
pub struct TopSnarker {
    /// Value SNARK prover public key
    #[graphql(flatten)]
    public_key: PK_,

    /// Value all-time total SNARK fees
    #[graphql(name = "total_fees")]
    total_fees: u64,

    /// Value epoch total SNARK fees
    #[graphql(name = "epoch_fees")]
    epoch_fees: u64,

    /// Value all-time min SNARK fee
    #[graphql(name = "min_fee")]
    min_fee: u64,

    /// Value epoch min SNARK fee
    #[graphql(name = "epoch_min_fee")]
    epoch_min_fee: u64,

    /// Value all-time max SNARK fee
    #[graphql(name = "max_fee")]
    max_fee: u64,

    /// Value epoch max SNARK fee
    #[graphql(name = "epoch_max_fee")]
    epoch_max_fee: u64,

    /// Value all-time SNARKs sold
    #[graphql(name = "snarks_sold")]
    snarks_sold: u32,

    /// Value epoch SNARKs sold
    #[graphql(name = "epoch_snarks_sold")]
    epoch_snarks_sold: u32,
}

///////////
// impls //
///////////

#[Object]
impl TopSnarkersQueryRoot {
    async fn top_snarkers(
        &self,
        ctx: &Context<'_>,
        query: Option<TopSnarkersQueryInput>,
        sort_by: Option<TopSnarkersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopSnarker>> {
        let db = db(ctx);
        let epoch = query
            .as_ref()
            .and_then(|q| q.epoch)
            .unwrap_or_else(|| db.get_current_epoch().expect("current epoch"));

        let genesis_state_hash = query
            .as_ref()
            .and_then(|q| q.genesis_state_hash.clone())
            .or_else(|| {
                db.get_best_block_genesis_hash()
                    .expect("best block genesis state hash")
                    .map(|g| g.0)
            });
        let genesis_state_hash = match StateHash::new(genesis_state_hash.unwrap()) {
            Ok(genesis_state_hash) => genesis_state_hash,
            Err(e) => return Err(async_graphql::Error::from(e)),
        };

        TopSnarkersQueryInput::verify_inputs(query.as_ref())?;
        TopSnarkersQueryInput::handler(
            db,
            query.as_ref(),
            epoch,
            &genesis_state_hash,
            sort_by.unwrap_or_default(),
            limit,
        )
    }
}

impl TopSnarkersQueryInput {
    fn handler(
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        epoch: u32,
        genesis_state_hash: &StateHash,
        sort_by: TopSnarkersSortByInput,
        limit: usize,
    ) -> Result<Vec<TopSnarker>> {
        use TopSnarkersSortByInput::*;

        let mut snarkers = vec![];
        let iter = make_iterator(db, epoch, genesis_state_hash, sort_by);

        for (key, _) in iter.flatten() {
            if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                || snarkers.len() >= limit
            {
                // gone beyond the desired epoch or limit
                break;
            }

            let pk = PublicKey::from_bytes(&key[StateHash::LEN..][U32_LEN..][U64_LEN..])?;
            let fee = u64_from_be_bytes(&key[StateHash::LEN..][U32_LEN..][..U64_LEN])?;
            let epoch_fees = match sort_by {
                TotalFeesAsc | TotalFeesDesc => fee,
                _ => db
                    .get_snark_prover_epoch_fees(&pk, Some(epoch), Some(genesis_state_hash), None)?
                    .expect("epoch fees"),
            };
            let epoch_max_fee = match sort_by {
                MaxFeeAsc | MaxFeeDesc => fee,
                _ => db
                    .get_snark_prover_epoch_max_fee(
                        &pk,
                        Some(epoch),
                        Some(genesis_state_hash),
                        None,
                    )?
                    .expect("epoch max fee"),
            };
            let total_fees = match sort_by {
                AllTimeTotalFeesAsc | AllTimeTotalFeesDesc => fee,
                _ => db
                    .get_snark_prover_total_fees(&pk, None)?
                    .expect("total fees"),
            };
            let max_fee = match sort_by {
                AllTimeMaxFeeAsc | AllTimeMaxFeeDesc => fee,
                _ => db.get_snark_prover_max_fee(&pk, None)?.expect("max fee"),
            };

            let top_snarker = TopSnarker {
                epoch_fees,
                epoch_max_fee,
                epoch_min_fee: db
                    .get_snark_prover_epoch_min_fee(
                        &pk,
                        Some(epoch),
                        Some(genesis_state_hash),
                        None,
                    )?
                    .expect("epoch min fee"),
                epoch_snarks_sold: db.get_snarks_pk_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?,
                total_fees,
                max_fee,
                min_fee: db.get_snark_prover_min_fee(&pk, None)?.expect("min fee"),
                snarks_sold: db.get_snarks_pk_total_count(&pk)?,
                public_key: PK_::new(db, pk),
            };

            if TopSnarkersQueryInput::matches(query, &top_snarker) {
                snarkers.push(top_snarker);
            }
        }

        Ok(snarkers)
    }

    fn verify_inputs(query: Option<&Self>) -> Result<()> {
        if let Some(public_key) = query.and_then(|q| q.public_key.as_ref()) {
            if !PublicKey::is_valid(public_key as &str) {
                return Err(async_graphql::Error::new(format!(
                    "Invalid public key: {}",
                    public_key
                )));
            }
        }

        if let Some(username) = query.and_then(|q| q.username.as_ref()) {
            if !Username::is_valid(username as &str) {
                return Err(async_graphql::Error::new(format!(
                    "Invalid username: {}",
                    username
                )));
            }
        }

        Ok(())
    }

    fn matches(query: Option<&Self>, top_snarker: &TopSnarker) -> bool {
        if let Some(Self {
            epoch: _,
            genesis_state_hash: _,
            public_key,
            username,
        }) = query
        {
            if let Some(public_key) = public_key {
                if top_snarker.public_key.public_key != *public_key {
                    return false;
                }
            }

            if let Some(username) = username {
                if top_snarker.public_key.username != *username {
                    return false;
                }
            }
        }

        true
    }
}

/////////////
// helpers //
/////////////

fn make_iterator<'a>(
    db: &'a Arc<IndexerStore>,
    epoch: u32,
    genesis_state_hash: &StateHash,
    sort_by: TopSnarkersSortByInput,
) -> DBIterator<'a> {
    match sort_by {
        TopSnarkersSortByInput::TotalFeesAsc => {
            db.snark_prover_total_fees_epoch_iterator(epoch, genesis_state_hash, Direction::Forward)
        }
        TopSnarkersSortByInput::TotalFeesDesc => {
            db.snark_prover_total_fees_epoch_iterator(epoch, genesis_state_hash, Direction::Reverse)
        }
        TopSnarkersSortByInput::MaxFeeAsc => {
            db.snark_prover_max_fee_epoch_iterator(epoch, genesis_state_hash, Direction::Forward)
        }
        TopSnarkersSortByInput::MaxFeeDesc => {
            db.snark_prover_max_fee_epoch_iterator(epoch, genesis_state_hash, Direction::Reverse)
        }
        TopSnarkersSortByInput::AllTimeTotalFeesAsc => {
            db.snark_prover_total_fees_iterator(IteratorMode::Start)
        }
        TopSnarkersSortByInput::AllTimeTotalFeesDesc => {
            db.snark_prover_total_fees_iterator(IteratorMode::End)
        }
        TopSnarkersSortByInput::AllTimeMaxFeeAsc => {
            db.snark_prover_max_fee_iterator(IteratorMode::Start)
        }
        TopSnarkersSortByInput::AllTimeMaxFeeDesc => {
            db.snark_prover_max_fee_iterator(IteratorMode::End)
        }
    }
}
