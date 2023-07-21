use juniper::GraphQLInputObject;

use crate::gql::root::Context;

#[allow(non_snake_case)]
pub struct Stakes {
    pub epochNumber: i32,
    pub ledgerHash: String,
}

impl Stakes {
    pub fn new(epoch_number: i32, ledger_hash: String) -> Self {
        Self {
            epochNumber: epoch_number,
            ledgerHash: ledger_hash,
        }
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Stakes query input")]
pub struct StakesQueryInput {
    pub epoch_number: Option<i32>,
    pub ledger_hash: Option<String>,
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Stakes")]
impl Stakes {
    #[graphql(description = "Epoch Number")]
    fn epochNumber(&self) -> &i32 {
        &self.epochNumber
    }

    #[graphql(description = "Ledger Hash")]
    fn ledgerHash(&self) -> &str {
        &self.ledgerHash
    }
}
