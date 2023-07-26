use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};

use crate::gql::root::Context;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DelegationTotals {
    pub total_delegated: i32,
    pub count_delegates: i32,
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Delegation Totals")]
impl DelegationTotals {
    #[graphql(description = "Total Delegated")]
    fn totalDelegated(&self) -> &i32 {
        &self.total_delegated
    }

    #[graphql(description = "Count Delegates")]
    fn countDelegates(&self) -> &i32 {
        &self.count_delegates
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Delegation Totals query input")]
pub struct DelegationTotalsQueryInput {
    pub total_delegated: Option<i32>,
    pub count_delegates: Option<i32>,
}

pub async fn get_delegation_totals_from_ctx(
    ctx: &Context,
    public_key: &str,
    epoch: i32,
) -> Result<Option<DelegationTotals>, rocksdb::Error> {
    // placeholder to get delegations totals from ctx.delegation_totals_store
    unimplemented!()
}
