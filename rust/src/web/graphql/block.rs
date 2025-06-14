use async_graphql::SimpleObject;

#[derive(SimpleObject, Debug)]
pub struct BlockInfo {
    /// Value block height
    pub height: u32,

    /// Value block global slot since genesis
    pub global_slot: u32,

    /// Value block state hash
    pub state_hash: String,

    /// Value block canonicity - canonical/orphaned
    pub canonicity: String,
}
