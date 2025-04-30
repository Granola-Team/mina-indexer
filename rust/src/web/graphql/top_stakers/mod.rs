//! GraphQL `topStakers` endpoint

use super::db;
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    ledger::{account, store::best::BestLedgerStore, token::TokenAddress, username::Username},
    store::username::UsernameStore,
    utility::store::common::{from_be_bytes, U32_LEN},
};
use anyhow::Context as aContext;
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject)]
pub struct TopStakersQueryInput {
    epoch: u32,

    #[graphql(name = "genesis_state_hash")]
    genesis_state_hash: Option<String>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum TopStakersSortByInput {
    #[default]
    NumCanonicalBlocksProducedDesc,
    NumCanonicalBlocksProducedAsc,
}

#[derive(Default)]
pub struct TopStakersQueryRoot;

#[derive(SimpleObject)]
pub struct TopStakerAccount {
    username: String,

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
    async fn top_stakers(
        &self,
        ctx: &Context<'_>,
        query: Option<TopStakersQueryInput>,
        sort_by: Option<TopStakersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopStakerAccount>> {
        let db = db(ctx);
        let epoch = query.as_ref().map_or_else(
            || db.get_current_epoch().expect("current epoch"),
            |q| q.epoch,
        );

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
            Err(e) => return Err(async_graphql::Error::new(e.to_string())),
        };

        let mut accounts = Vec::new();
        let direction = match sort_by.unwrap_or_default() {
            TopStakersSortByInput::NumCanonicalBlocksProducedAsc => Direction::Forward,
            TopStakersSortByInput::NumCanonicalBlocksProducedDesc => Direction::Reverse,
        };

        for (key, _) in db
            .canonical_epoch_blocks_produced_iterator(
                Some(genesis_state_hash.clone()),
                Some(epoch),
                direction,
            )
            .flatten()
        {
            if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                || accounts.len() >= limit
            {
                // gone beyond desired region or limit
                break;
            }

            let num = from_be_bytes(key[StateHash::LEN..][U32_LEN..][..U32_LEN].to_vec());
            let pk = PublicKey::from_bytes(&key[StateHash::LEN..][U32_LEN..][U32_LEN..])?;
            let account = db
                .get_best_account(&pk, &TokenAddress::default())? // always MINA
                .with_context(|| format!("Account missing {pk}"))
                .unwrap()
                .deduct_mina_account_creation_fee();

            let account = TopStakerAccount::from((
                account.clone(),
                db.get_block_production_pk_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash.clone()),
                )?,
                num,
                db.get_block_production_pk_supercharged_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash.clone()),
                )?,
                db.get_pk_epoch_slots_produced_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash.clone()),
                )?,
                db.get_username(&pk)?,
            ));

            accounts.push(account);
        }

        Ok(accounts)
    }
}

impl From<(account::Account, u32, u32, u32, u32, Option<Username>)> for TopStakerAccount {
    fn from(value: (account::Account, u32, u32, u32, u32, Option<Username>)) -> Self {
        Self {
            public_key: value.0.public_key.0,
            username: value.5.unwrap_or_default().0,
            num_blocks_produced: value.1,
            num_canonical_blocks_produced: value.2,
            num_supercharged_blocks_produced: value.3,
            num_slots_produced: value.4,
        }
    }
}
