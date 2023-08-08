use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};

use crate::{delegation_totals_store::get_delegation_totals_from_db, gql::root::Context};

use juniper::GraphQLScalarValue;

// f64 type does not implement the Eq and Hash traits, which are required by the juniper crate for GraphQL objects
// and i64 doesn't implement the required traits for being used directly as a GraphQL scalar value in the Juniper schema
// so we can create a custom scalar type for representing the total_delegated field, and then convert the value between i64 and f64 when needed
#[derive(Debug, Clone, PartialEq, GraphQLScalarValue, Serialize, Deserialize)]
pub struct TotalDelegated(pub f64);

// Implement traits (conversion and others) for TotalDelegated
impl From<TotalDelegated> for f64 {
    fn from(val: TotalDelegated) -> Self {
        val.0
    }
}

impl From<f64> for TotalDelegated {
    fn from(value: f64) -> Self {
        TotalDelegated(value)
    }
}

impl Eq for TotalDelegated {}

impl Ord for TotalDelegated {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for TotalDelegated {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Hash for TotalDelegated {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DelegationTotals {
    pub total_delegated: TotalDelegated,
    pub count_delegates: i32,
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Delegation Totals")]
impl DelegationTotals {
    #[graphql(description = "Total Delegated")]
    fn totalDelegated(&self) -> &TotalDelegated {
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
    pub total_delegated: Option<TotalDelegated>,
    pub count_delegates: Option<i32>,
}

pub async fn get_delegation_totals_from_ctx(
    ctx: &Context,
    public_key: &str,
    epoch: i32,
) -> Option<DelegationTotals> {
    match get_delegation_totals_from_db(&ctx.delegation_totals_db, public_key, epoch) {
        Ok(Some(delegation_totals)) => Some(delegation_totals),
        _ => None,
    }
}
