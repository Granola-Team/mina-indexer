//! GraphQL `actions` endpoint

use super::{block::BlockInfo, db, txn::TxnInfo};
use crate::{
    base::public_key::PublicKey,
    block::store::BlockStore,
    canonicity::store::CanonicityStore,
    command::store::UserCommandStore,
    ledger::token::TokenAddress,
    mina_blocks::v2::zkapp::action_state::ActionStateWithMeta,
    store::{zkapp::actions::ZkappActionStore, IndexerStore},
    utility::store::zkapp::actions::zkapp_action_index,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject, Debug)]
pub struct ActionsQueryInput {
    /// Input public key
    pub public_key: String,

    /// Input token address
    pub token: Option<String>,

    /// Input start block height
    pub start_block_height: Option<u32>,

    /// Input end block height
    pub end_block_height: Option<u32>,

    /// Input start action index
    pub start_action_index: Option<u32>,

    /// Input end action index
    pub end_action_index: Option<u32>,
}

#[derive(Default, Enum, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ActionsSortByInput {
    #[default]
    BlockHeightDesc,
    BlockHeightAsc,
}

#[derive(SimpleObject, Debug)]
pub struct Action {
    /// Value action data
    pub action: String,

    /// Value action txn
    pub txn: TxnInfo,

    /// Value action block
    pub block: BlockInfo,
}

#[derive(Default)]
pub struct ActionsQueryRoot;

#[Object]
impl ActionsQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn actions(
        &self,
        ctx: &Context<'_>,
        query: ActionsQueryInput,
        sort_by: Option<ActionsSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<Action>>> {
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
            ActionsSortByInput::BlockHeightAsc => Direction::Forward,
            ActionsSortByInput::BlockHeightDesc => Direction::Reverse,
        };
        let index = match direction {
            Direction::Forward => query.start_action_index,
            Direction::Reverse => query.end_action_index,
        };

        let mut actions = Vec::with_capacity(limit);
        for (key, value) in db
            .actions_iterator(&public_key, &token, index, direction)
            .flatten()
        {
            if actions.len() >= limit {
                break;
            }

            let index = zkapp_action_index(&key);
            let action: ActionStateWithMeta = serde_json::from_slice(&value)?;

            if query.matches(&action, index) {
                actions.push(Action::new(db, action)?);
            }
        }

        Ok(Some(actions))
    }
}

impl Action {
    fn new(db: &IndexerStore, action: ActionStateWithMeta) -> async_graphql::Result<Self> {
        let canonicity = db
            .get_block_canonicity(&action.state_hash)?
            .unwrap()
            .to_string();
        let global_slot = db.get_block_global_slot(&action.state_hash)?.unwrap();

        let cmd = db
            .get_user_command_state_hash(&action.txn_hash, &action.state_hash)?
            .unwrap();
        let memo = cmd.command.memo();
        let status = format!("{:?}", cmd.status);

        Ok(Self {
            action: action.action.0,
            txn: TxnInfo {
                memo,
                status,
                txn_hash: action.txn_hash.to_string(),
            },
            block: BlockInfo {
                canonicity,
                global_slot,
                state_hash: action.state_hash.0,
                height: action.block_height,
            },
        })
    }
}

impl ActionsQueryInput {
    fn matches(&self, action: &ActionStateWithMeta, index: u32) -> bool {
        let Self {
            public_key: _,
            token: _,
            start_block_height,
            end_block_height,
            start_action_index,
            end_action_index,
        } = self;

        // block height
        if let Some(start_block_height) = start_block_height {
            if action.block_height < *start_block_height {
                return false;
            }
        }

        if let Some(end_block_height) = end_block_height {
            if action.block_height >= *end_block_height {
                return false;
            }
        }

        // index
        if let Some(start_action_index) = start_action_index {
            if index < *start_action_index {
                return false;
            }
        }

        if let Some(end_action_index) = end_action_index {
            if index >= *end_action_index {
                return false;
            }
        }

        true
    }
}
