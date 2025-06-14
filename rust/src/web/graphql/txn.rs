//! Transaction info for actions/events

use async_graphql::SimpleObject;

#[derive(SimpleObject, Debug)]
pub struct TxnInfo {
    /// Value txn status - applied/failed
    pub status: String,

    /// Value txn hash
    pub txn_hash: String,

    /// Value txn memo
    pub memo: String,
}
