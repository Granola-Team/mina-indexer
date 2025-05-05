//! GQL pk

use crate::{
    base::public_key::PublicKey,
    store::{username::UsernameStore, IndexerStore},
};
use async_graphql::SimpleObject;
use serde::Serialize;

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
#[graphql(name = "PublicKey")]
pub struct PK {
    pub public_key: String,
    pub username: String,
}

impl PK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        Self {
            username: db
                .get_username(&pk)
                .expect("username")
                .unwrap_or_default()
                .0,
            public_key: pk.0,
        }
    }
}
