//! GraphQL `getEvents` endpoint

use super::{date_time::DateTime, db};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

#[derive(InputObject, Debug)]
pub struct EventsQueryInput {
    pub address: Option<String>,
}

#[derive(Enum, Copy, Clone, Debug, Eq, PartialEq)]
pub enum EventsSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(SimpleObject, Debug)]
pub struct Event {
    pub block_info: BlockInfo,
    pub event_data: EventData,
    pub transaction_info: TxnInfo,
}

#[derive(SimpleObject, Debug)]
pub struct BlockInfo {
    pub state_hash: String,
    pub timestamp: DateTime,
    pub ledger_hash: String,
    pub height: u32,
    pub parent_hash: String,
    pub chain_status: String, // TODO
    pub distance_from_max_block_height: u32,
    pub global_slot_since_genesis: u32,
}

#[derive(SimpleObject, Debug)]
pub struct EventData {
    pub data: String, // TODO
}

#[derive(SimpleObject, Debug)]
pub struct TxnInfo {
    pub status: String,
    pub hash: String,
    pub memo: String,
}

#[derive(Default)]
pub struct EventsQueryRoot;

#[Object]
impl EventsQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn get_actions<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<EventsQueryInput>,
        sort_by: Option<EventsSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<Event>>> {
        let db = db(ctx);

        todo!(
            "getEvents:\n  db:      {:?}\n  query:   {:?}\n  sort_by: {:?}\n  limit:   {:?}",
            db,
            query,
            sort_by,
            limit
        )
    }
}
