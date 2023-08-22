use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use reqwest;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "gql/query/transactions.graphql",
    schema_path = "gql/query/schema.json",
    response_derives = "Debug",
)]

// Import the generated module
use generated::Transactions;

async fn perform_my_query(variables: union_query::Variables) -> Result<(), Box<dyn Error>> {
    let variables = Variables {
        // Create and set variable values here
    };

    // This is the important line = Build the query
    let request_body = Transactions::build_query(variables);

    // Send the request
    let client = reqwest::Client::new();
    let mut res = client
        .post("/gql")
        .json(&request_body)
        .send()
        .await?;

    // Parse the response
    let response_body: Response<generated::Transactions::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);

    // Access the data
    let data = response_body.data.unwrap();

    // Use the data in your application
    Ok(())
}
