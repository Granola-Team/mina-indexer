#![allow(clippy::all, warnings)]
pub struct Transactions;
pub mod transactions {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Transactions";
    pub const QUERY : & str = "query Transactions(\n  $memos: [String!]!,\n  $dateTime_gte: String!,\n  $dateTime_lte: String!,\n  $limit: Float! = 1000\n) {\n  transactions(\n    limit: $limit,\n    query: {\n      canonical: true,\n      kind: \"PAYMENT\",\n      OR: $memos,\n      dateTime_gte: $dateTime_gte,\n      dateTime_lte: $dateTime_lte,\n      sortBy: NONCE_DESC\n    }\n  ) {\n    memo\n    canonical\n    from\n    hash\n    blockHeight\n    dateTime\n  }\n}\n" ;
    use super::*;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    type DateTimeUtc = super::DateTimeUtc;
    #[derive(Serialize)]
    pub struct Variables {
        pub memos: Vec<String>,
        #[serde(rename = "dateTime_gte")]
        pub date_time_gte: String,
        #[serde(rename = "dateTime_lte")]
        pub date_time_lte: String,
        pub limit: Float,
    }
    impl Variables {
        pub fn default_limit() -> Float {
            1000i64
        }
    }
    #[derive(Deserialize)]
    pub struct ResponseData {
        pub transactions: Vec<TransactionsTransactions>,
    }
    #[derive(Deserialize)]
    pub struct TransactionsTransactions {
        pub memo: String,
        pub canonical: Boolean,
        pub from: String,
        pub hash: String,
        #[serde(rename = "blockHeight")]
        pub block_height: Int,
        #[serde(rename = "dateTime")]
        pub date_time: DateTimeUtc,
    }
}
impl graphql_client::GraphQLQuery for Transactions {
    type Variables = transactions::Variables;
    type ResponseData = transactions::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: transactions::QUERY,
            operation_name: transactions::OPERATION_NAME,
        }
    }
}
