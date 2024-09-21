use super::db;
use crate::{
    block::store::BlockStore,
    ledger::public_key::PublicKey,
    snark_work::store::SnarkStore,
    store::username::UsernameStore,
    utility::store::{u32_from_be_bytes, U32_LEN, U64_LEN},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject)]
pub struct TopSnarkersQueryInput {
    epoch: u32,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq)]
pub enum TopSnarkersSortByInput {
    MaxFeeAsc,
    MaxFeeDesc,
    TotalFeesAsc,
    #[default]
    TotalFeesDesc,
}

#[derive(Default)]
pub struct TopSnarkersQueryRoot;

#[derive(SimpleObject)]
pub struct TopSnarker {
    username: Option<String>,

    #[graphql(name = "public_key")]
    public_key: String,

    #[graphql(name = "total_fees")]
    total_fees: u64,

    #[graphql(name = "min_fee")]
    min_fee: u64,

    #[graphql(name = "max_fee")]
    max_fee: u64,

    #[graphql(name = "snarks_sold")]
    snarks_sold: u32,
}

#[Object]
impl TopSnarkersQueryRoot {
    async fn top_snarkers<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<TopSnarkersQueryInput>,
        sort_by: Option<TopSnarkersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopSnarker>> {
        use TopSnarkersSortByInput::*;
        let db = db(ctx);
        let epoch = query
            .as_ref()
            .map_or(db.get_current_epoch().expect("current epoch"), |q| q.epoch);
        let iter = match sort_by {
            Some(MaxFeeAsc) => db.snark_prover_max_fee_epoch_iterator(epoch, Direction::Forward),
            Some(MaxFeeDesc) => db.snark_prover_max_fee_epoch_iterator(epoch, Direction::Reverse),
            Some(TotalFeesAsc) => {
                db.snark_prover_total_fees_epoch_iterator(epoch, Direction::Forward)
            }
            Some(TotalFeesDesc) | None => {
                db.snark_prover_total_fees_epoch_iterator(epoch, Direction::Reverse)
            }
        };
        let mut snarkers = vec![];

        for (key, _) in iter.flatten() {
            let key_epoch = u32_from_be_bytes(&key[..U32_LEN])?;
            if key_epoch != epoch {
                // we've gone beyond the desired epoch
                break;
            }

            let pk = PublicKey::from_bytes(&key[U32_LEN..][U64_LEN..])?;
            let username = db.get_username(&pk).ok().flatten().map(|u| u.0);
            let total_fees = db
                .get_snark_prover_epoch_fees(&pk, Some(epoch))?
                .expect("total fees");
            let min_fee = db
                .get_snark_prover_epoch_min_fee(&pk, Some(epoch))?
                .expect("min fee");
            let max_fee = db
                .get_snark_prover_epoch_max_fee(&pk, Some(epoch))?
                .expect("max fee");
            let snarks_sold = db.get_snarks_pk_epoch_count(&pk, Some(epoch))?;

            snarkers.push(TopSnarker {
                username,
                public_key: pk.0,
                total_fees,
                min_fee,
                max_fee,
                snarks_sold,
            });

            if snarkers.len() >= limit {
                break;
            }
        }
        Ok(snarkers)
    }
}
