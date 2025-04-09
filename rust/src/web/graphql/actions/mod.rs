//! GraphQL `getActions` endpoint

use super::{date_time::DateTime, db};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

#[derive(InputObject, Debug)]
pub struct ActionsQueryInput {
    pub address: Option<String>,
}

#[derive(Default, Enum, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ActionsSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[default]
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(SimpleObject, Debug)]
pub struct Action {
    pub block_info: BlockInfo,
    pub action_state: ActionState,
    pub action_data: ActionData,
}

#[derive(SimpleObject, Debug)]
pub struct BlockInfo {
    pub state_hash: String,
    pub timestamp: DateTime,
    pub ledger_hash: String,
    pub height: u32,
    pub parent_hash: String,
    pub chain_status: String,
    pub distance_from_max_block_height: u32,
    pub global_slot_since_genesis: u32,
}

#[derive(SimpleObject, Debug)]
pub struct ActionState {
    pub action_state_one: String,
    pub action_state_two: String,
    pub action_state_three: String,
    pub action_state_four: String,
    pub action_state_five: String,
}

#[derive(SimpleObject, Debug)]
pub struct ActionData {
    pub data: String,
    pub transaction_info: TxnInfo,
}

#[derive(SimpleObject, Debug)]
pub struct TxnInfo {
    pub status: String,
    pub hash: String,
    pub memo: String,
}

#[derive(Default)]
pub struct ActionsQueryRoot;

#[Object]
impl ActionsQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn get_actions(
        &self,
        ctx: &Context<'_>,
        query: Option<ActionsQueryInput>,
        sort_by: Option<ActionsSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<Action>>> {
        let db = db(ctx);

        todo!(
            "getActions:\n  db:      {:?}\n  query:   {:?}\n  sort_by: {:?}\n  limit:   {:?}",
            db,
            query,
            sort_by,
            limit
        )
    }
}
