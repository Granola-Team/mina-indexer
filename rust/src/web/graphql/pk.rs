//! GQL public key/username pairs

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

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct PK_ {
    #[graphql(name = "public_key")]
    pub public_key: String,
    pub username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct DelegatePK {
    pub delegate: String,
    pub delegate_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct CreatorPK {
    pub creator: String,
    pub creator_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct CoinbaseReceiverPK {
    pub coinbase_receiver: String,
    pub coinbase_receiver_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct ProverPK {
    pub prover: String,
    pub prover_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct RecipientPK {
    pub recipient: String,
    pub recipient_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct SenderPK {
    pub sender: String,
    pub sender_username: String,
}

#[derive(Default, Clone, Debug, PartialEq, SimpleObject, Serialize)]
pub struct WinnerPK {
    pub winner: String,
    pub winner_username: String,
}

///////////
// impls //
///////////

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

impl PK_ {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl DelegatePK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl CreatorPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl CoinbaseReceiverPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl ProverPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl RecipientPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl SenderPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

impl WinnerPK {
    pub fn new(db: &std::sync::Arc<IndexerStore>, pk: PublicKey) -> Self {
        PK::new(db, pk).into()
    }
}

/////////////////
// conversions //
/////////////////

impl From<PK> for PK_ {
    fn from(value: PK) -> Self {
        Self {
            public_key: value.public_key,
            username: value.username,
        }
    }
}

impl From<PK> for DelegatePK {
    fn from(value: PK) -> Self {
        Self {
            delegate: value.public_key,
            delegate_username: value.username,
        }
    }
}

impl From<PK> for CoinbaseReceiverPK {
    fn from(value: PK) -> Self {
        Self {
            coinbase_receiver: value.public_key,
            coinbase_receiver_username: value.username,
        }
    }
}

impl From<PK> for CreatorPK {
    fn from(value: PK) -> Self {
        Self {
            creator: value.public_key,
            creator_username: value.username,
        }
    }
}

impl From<PK> for ProverPK {
    fn from(value: PK) -> Self {
        Self {
            prover: value.public_key,
            prover_username: value.username,
        }
    }
}

impl From<PK> for RecipientPK {
    fn from(value: PK) -> Self {
        Self {
            recipient: value.public_key,
            recipient_username: value.username,
        }
    }
}

impl From<PK> for SenderPK {
    fn from(value: PK) -> Self {
        Self {
            sender: value.public_key,
            sender_username: value.username,
        }
    }
}

impl From<PK> for WinnerPK {
    fn from(value: PK) -> Self {
        Self {
            winner: value.public_key,
            winner_username: value.username,
        }
    }
}
