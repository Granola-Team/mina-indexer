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
    pub block_height: i32,
    pub date_time: DateTime<Utc>,
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

                let naive_dt = NaiveDateTime::from_timestamp_millis(timestamp as i64).unwrap();
                let datetime = DateTime::<Utc>::from_utc(naive_dt, Utc);

                Self {
                    from: sanitize_json(sender),
                    to: sanitize_json(receiver),
                    memo: sanitize_json(payload.common.memo),
                    block_height: height,
                    date_time: datetime,
                }
            }
        }
    }
}

// JSON utility
fn sanitize_json<T: serde::Serialize>(s: T) -> String {
    serde_json::to_string(&s).unwrap().replace('\"', "")
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Transaction query input")]
pub struct TransactionQueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub memos: Option<Vec<String>>,
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
    fn block_height(&self) -> i32 {
        self.block_height
    }

    #[graphql(description = "Datetime")]
    fn date_time(&self) -> DateTime<Utc> {
        self.date_time
    }
}
