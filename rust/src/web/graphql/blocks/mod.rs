use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    proof_systems::signer::pubkey::CompressedPubKey,
    store::IndexerStore,
};
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, InputObject, Object, Result, Schema, SimpleObject,
};
use chrono::{DateTime, SecondsFormat};
use std::sync::Arc;

#[derive(InputObject)]
pub struct BlockQueryInput {
    state_hash: String,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn block<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<Block>> {
        let db = ctx
            .data::<Arc<IndexerStore>>()
            .expect("db to be in context");
        // Choose best tip if query wasn't provided
        let state_hash_str = match query {
            Some(query) => query.state_hash,
            None => match db.get_best_block()? {
                Some(block) => block.state_hash,
                None => return Ok(None),
            },
        };
        let state_hash = &BlockHash::from(state_hash_str);
        let pcb = match db.get_block(state_hash)? {
            Some(pcb) => pcb,
            None => return Ok(None),
        };

        Ok(Some(Block::from(pcb)))
    }
}

/// Build the schema for the block endpoints
pub fn build_schema(
    store: Arc<IndexerStore>,
) -> Schema<QueryRoot, EmptyMutation, EmptySubscription> {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(store)
        .finish()
}

#[derive(SimpleObject)]
struct Block {
    /// Value state_hash
    state_hash: String,
    /// Value block_height
    block_height: u32,
    /// Value winning_account
    winner_account: WinnerAccount,
    /// Value date_time
    date_time: String,
    /// Value creator account
    creator_account: CreatorAccount,
}

#[derive(SimpleObject)]
struct WinnerAccount {
    /// The public_key for the WinnerAccount
    public_key: String,
}

#[derive(SimpleObject)]
struct CreatorAccount {
    /// The public_key for the WinnerAccount
    public_key: String,
}

/// convert epoch millis to an ISO 8601 formatted date
fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl From<PrecomputedBlock> for Block {
    fn from(block: PrecomputedBlock) -> Self {
        let winner_account = block.block_creator().0;
        let date_time = millis_to_date_string(block.timestamp().try_into().unwrap());
        let pk_creator = block.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();

        Block {
            state_hash: block.state_hash,
            block_height: block.blockchain_length,
            date_time,
            winner_account: WinnerAccount {
                public_key: winner_account,
            },
            creator_account: CreatorAccount {
                public_key: creator,
            },
        }
    }
}
