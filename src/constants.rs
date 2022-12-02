/// Current GQL endpoint for mina. It may not work with WS. Subscriptions don't work over HTTP (mina's problem)
pub const GRAPHQL_URL: &'static str = "wss://minagraph.com/graphql";
pub const SUBSCRIPTION_QUERY: &'static str =
    include_str!("../graphql/subscriptions/NewBlockSubscription.graphql");
