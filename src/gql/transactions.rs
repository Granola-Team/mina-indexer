#![allow(clippy::all, warnings)]
pub struct Transactions;
pub mod transactions {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Transactions";
    pub const QUERY : & str = "query Transactions {\n  transactions(limit: 10000, query: {canonical: true, kind: \"PAYMENT\", OR: [{memo: \"bUeKKPB3UehQ\"}, {memo: \"E4Mw6WPPoZpTVmbnH6uTsPy9QGX6wD6AYt\"}, {memo: \"E4YbUmaZZqAoURRyPFxvfHRXHC1k7nufTxYGAc8cA\"}], dateTime_gte: \"2023-05-20T06:00:00Z\", dateTime_lte: \"2023-05-28T06:00:00Z\"}, sortBy: NONCE_DESC) {\n    memo\n    canonical\n    from\n    hash\n    blockHeight\n    dateTime\n  }\n}\n\n" ;
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
    pub struct Variables;
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
