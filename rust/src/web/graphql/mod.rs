use crate::{proof_systems::signer::pubkey::CompressedPubKey, store::IndexerStore};
use crate::block::store::BlockStore;
use actix_web::{HttpResponse, Result};
use async_graphql::{
    http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject
};
use chrono::DateTime;
use std::sync::Arc;

//type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send +
// Sync>>;

pub struct QueryRoot;

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

/// RFC 2822 date format
fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}


#[Object]
impl QueryRoot {
    async fn block<'ctx>(&self, ctx: &Context<'ctx>) -> Block {
        let db = ctx.data::<Arc<IndexerStore>>().expect("db should be there");
        let best_tip = db.get_best_block().expect("asdf").unwrap();
        let winner_account = best_tip.block_creator().0;
        let date_time = millis_to_date_string(best_tip.timestamp().try_into().unwrap());
        let pk_creator = best_tip.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();
        Block {
            state_hash: best_tip.state_hash,
            block_height: best_tip.blockchain_length,
            date_time,
            winner_account: WinnerAccount {
                public_key: winner_account,
            },
            creator_account: CreatorAccount {
                public_key: creator,
            }
        }
    }
}

pub fn build_schema(
    store: Arc<IndexerStore>,
) -> Schema<QueryRoot, EmptyMutation, EmptySubscription> {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(store)
        .finish()
}

pub async fn index_graphiql() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/graphql").finish()))
}
