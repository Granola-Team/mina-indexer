use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;
use juniper::GraphQLInputObject;
use mina_serialization_types::json::UserCommandWithStatusJson;
use mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBodyJson;
use mina_serialization_types::staged_ledger_diff::StakeDelegationJson;
use mina_serialization_types::staged_ledger_diff::UserCommandJson;

use crate::gql::root::Context;

pub struct Transaction {
    pub from: String,
    pub to: String,
    pub memo: String,
    pub height: i32,
    pub timestamp: DateTime<Utc>,
}

impl Transaction {
    pub fn from_cmd(cmd: UserCommandWithStatusJson, height: i32, timestamp: u64) -> Self {
        match cmd.data {
            UserCommandJson::SignedCommand(signed_cmd) => {
                let payload = signed_cmd.payload;
                let (sender, receiver) = {
                    match payload.body {
                        SignedCommandPayloadBodyJson::PaymentPayload(payload) => {
                            (payload.source_pk, payload.receiver_pk)
                        }
                        SignedCommandPayloadBodyJson::StakeDelegation(payload) => {
                            let StakeDelegationJson::SetDelegate {
                                delegator,
                                new_delegate,
                            } = payload;

                            (delegator, new_delegate)
                        }
                    }
                };

                Self {
                    from: sanitize_json(sender),
                    to: sanitize_json(receiver),
                    memo: sanitize_json(payload.common.memo),
                    height,
                    timestamp: timestamp_to_datetime(timestamp),
                }
            }
        }
    }
}

// JSON utility
fn sanitize_json<T: serde::Serialize>(s: T) -> String {
    serde_json::to_string(&s).unwrap().replace('\"', "")
}

// Conversion utility
fn timestamp_to_datetime(ts: u64) -> DateTime<Utc> {
    let ts_sec = ts / 1_000; // convert milliseconds to seconds
    let naive_datetime = NaiveDateTime::from_timestamp_opt(ts_sec as i64, 0).unwrap();
    DateTime::<Utc>::from_utc(naive_datetime, Utc)
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Transaction query input")]
pub struct TransactionQueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub memo: Option<String>,
    pub date_time_gte: Option<DateTime<Utc>>,
    pub date_time_lte: Option<DateTime<Utc>>,
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Transaction")]
impl Transaction {
    #[graphql(description = "From")]
    fn from(&self) -> &str {
        &self.from
    }

    #[graphql(description = "To")]
    fn to(&self) -> &str {
        &self.to
    }

    #[graphql(description = "Memo")]
    fn memo(&self) -> &str {
        &self.memo
    }

    #[graphql(description = "Block height")]
    fn height(&self) -> i32 {
        self.height
    }

    #[graphql(description = "Timestamp")]
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
