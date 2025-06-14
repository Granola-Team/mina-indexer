//! GraphQL `getEvents` endpoint

use super::{block::BlockInfo, db, txn::TxnInfo};
use crate::{
    base::public_key::PublicKey,
    block::store::BlockStore,
    canonicity::store::CanonicityStore,
    command::store::UserCommandStore,
    ledger::token::TokenAddress,
    mina_blocks::v2::zkapp::event::ZkappEventWithMeta,
    store::{zkapp::events::ZkappEventStore, IndexerStore},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject, Debug)]
pub struct EventsQueryInput {
    /// Input public key
    pub public_key: String,

    /// Input token address
    pub token: Option<String>,

    /// Input start block height
    pub start_block_height: Option<u32>,

    /// Input end block height
    pub end_block_height: Option<u32>,
}

#[derive(Default, Enum, Copy, Clone, Debug, Eq, PartialEq)]
pub enum EventsSortByInput {
    #[default]
    BlockHeightDesc,
    BlockHeightAsc,
}

/// Value event
#[derive(SimpleObject, Debug)]
pub struct Event {
    /// Value event data
    pub event: String,

    /// Value event txn
    pub txn: TxnInfo,

    /// Value event block
    pub block: BlockInfo,
}

#[derive(Default)]
pub struct EventsQueryRoot;

#[Object]
impl EventsQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn get_actions(
        &self,
        ctx: &Context<'_>,
        query: EventsQueryInput,
        sort_by: Option<EventsSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<Event>>> {
        let db = db(ctx);

        let public_key = match PublicKey::new(&query.public_key) {
            Ok(public_key) => public_key,
            Err(_) => {
                return Err(async_graphql::Error::new(format!(
                    "Invalid public key: {}",
                    &query.public_key
                )))
            }
        };
        let token = match query.token.as_ref() {
            Some(token) => match TokenAddress::new(token) {
                Some(token) => token,
                None => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid token: {}",
                        token
                    )))
                }
            },
            None => TokenAddress::default(),
        };
        let direction = match sort_by.unwrap_or_default() {
            EventsSortByInput::BlockHeightAsc => Direction::Forward,
            EventsSortByInput::BlockHeightDesc => Direction::Reverse,
        };

        let mut events = Vec::with_capacity(limit);
        for (_, value) in db.events_iterator(&public_key, &token, direction).flatten() {
            if events.len() >= limit {
                break;
            }

            let event: ZkappEventWithMeta = serde_json::from_slice(&value)?;
            if query.matches(&event) {
                events.push(Event::new(db, event)?);
            }
        }

        Ok(Some(events))
    }
}

impl Event {
    fn new(db: &IndexerStore, event: ZkappEventWithMeta) -> async_graphql::Result<Self> {
        let canonicity = db
            .get_block_canonicity(&event.state_hash)?
            .unwrap()
            .to_string();
        let global_slot = db.get_block_global_slot(&event.state_hash)?.unwrap();

        let cmd = db
            .get_user_command_state_hash(&event.txn_hash, &event.state_hash)?
            .unwrap();
        let memo = cmd.command.memo();
        let status = format!("{:?}", cmd.status);

        Ok(Self {
            event: event.event.0,
            txn: TxnInfo {
                memo,
                status,
                txn_hash: event.txn_hash.to_string(),
            },
            block: BlockInfo {
                canonicity,
                global_slot,
                state_hash: event.state_hash.0,
                height: event.block_height,
            },
        })
    }
}

impl EventsQueryInput {
    fn matches(&self, event: &ZkappEventWithMeta) -> bool {
        let Self {
            public_key: _,
            token: _,
            start_block_height,
            end_block_height,
        } = self;

        if let Some(start_block_height) = start_block_height {
            if event.block_height < *start_block_height {
                return false;
            }
        }

        if let Some(end_block_height) = end_block_height {
            if event.block_height >= *end_block_height {
                return false;
            }
        }

        true
    }
}
