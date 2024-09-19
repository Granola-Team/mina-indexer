use super::db;
use crate::{
    block::store::BlockStore,
    ledger::{account, public_key::PublicKey, store::best::BestLedgerStore},
    store::username::UsernameStore,
    utility::store::{from_be_bytes, U32_LEN},
};
use anyhow::Context as aContext;
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject)]
pub struct TopStakersQueryInput {
    epoch: u32,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum TopStakersSortByInput {
    NumCanonicalBlocksProducedAsc,
    NumCanonicalBlocksProducedDesc,
}

#[derive(Default)]
pub struct TopStakersQueryRoot;

#[derive(SimpleObject)]
pub struct TopStakerAccount {
    username: Option<String>,

    #[graphql(name = "public_key")]
    public_key: String,

    #[graphql(name = "num_blocks_produced")]
    num_blocks_produced: u32,

    #[graphql(name = "num_canonical_blocks_produced")]
    num_canonical_blocks_produced: u32,

    #[graphql(name = "num_supercharged_blocks_produced")]
    num_supercharged_blocks_produced: u32,

    #[graphql(name = "num_slots_produced")]
    num_slots_produced: u32,
}

#[Object]
impl TopStakersQueryRoot {
    async fn top_stakers<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<TopStakersQueryInput>,
        sort_by: Option<TopStakersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopStakerAccount>> {
        let db = db(ctx);
        let epoch = query
            .as_ref()
            .map_or(db.get_current_epoch().expect("current epoch"), |q| q.epoch);
        let direction = match sort_by {
            Some(TopStakersSortByInput::NumCanonicalBlocksProducedAsc) => Direction::Forward,
            Some(TopStakersSortByInput::NumCanonicalBlocksProducedDesc) | None => {
                Direction::Reverse
            }
        };
        let mut accounts = Vec::new();

        for (key, _) in db
            .canonical_epoch_blocks_produced_iterator(Some(epoch), direction)
            .flatten()
        {
            let key_epoch = from_be_bytes(key[..U32_LEN].to_vec());
            if key_epoch != epoch {
                break;
            }

            let num = from_be_bytes(key[U32_LEN..][..U32_LEN].to_vec());
            let pk = PublicKey::from_bytes(&key[U32_LEN..][U32_LEN..])?;
            let account = db
                .get_best_account(&pk)?
                .with_context(|| format!("Account missing {pk}"))
                .unwrap()
                .display();
            let username = match db.get_username(&pk) {
                Ok(None) | Err(_) => None,
                Ok(Some(username)) => Some(username.0),
            };
            let account = TopStakerAccount::from((
                account.clone(),
                db.get_block_production_pk_epoch_count(&pk, Some(epoch))?,
                num,
                db.get_block_production_pk_supercharged_epoch_count(&pk, Some(epoch))?,
                db.get_pk_epoch_slots_produced_count(&pk, Some(epoch))?,
                username,
            ));

            accounts.push(account);
            if accounts.len() >= limit {
                break;
            }
        }
        Ok(accounts)
    }
}

impl From<(account::Account, u32, u32, u32, u32, Option<String>)> for TopStakerAccount {
    fn from(value: (account::Account, u32, u32, u32, u32, Option<String>)) -> Self {
        Self {
            public_key: value.0.public_key.0,
            username: value.5.or(Some("Unknown".to_string())),
            num_blocks_produced: value.1,
            num_canonical_blocks_produced: value.2,
            num_supercharged_blocks_produced: value.3,
            num_slots_produced: value.4,
        }
    }
}
