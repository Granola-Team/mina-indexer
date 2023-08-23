use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use reqwest;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "gql/query/transactions.graphql",
    schema_path = "gql/query/schema.json",
    response_derives = "Debug",
)]

use generated::Transactions;

async fn perform_my_query(variables: transactions::Variables) -> Result<(), Box<dyn Error>> {
    let variables = Variables {
        // Create and set variable values here
    };

    // This is the important line = Build the query and send the request
    let request_body = Transactions::build_query(variables);

    let mina_explorer_gql_api_url = "https://graphql.minaexplorer.com";

    let client = reqwest::Client::new();
    let mut res = client
        .post(mina_explorer_gql_api_url)
        .json(&request_body)
        .send()
        .await?;

    // Parse the response and access the data
    let response_body: Response<generated::Transactions::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    let data = response_body.data.unwrap();

    Ok(())
}
