use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;
use juniper::GraphQLInputObject;
use mina_serialization_types::json::UserCommandWithStatusJson;
use mina_serialization_types::staged_ledger_diff::SignedCommandPayloadBodyJson;
use mina_serialization_types::staged_ledger_diff::StakeDelegationJson;
use mina_serialization_types::staged_ledger_diff::UserCommandJson;
use mina_serialization_types::v1::UserCommandWithStatusV1;

use crate::gql::root::Context;
use crate::store::TransactionKey;
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub memo: String,
    pub block_height: i32,
    pub date_time: DateTime<Utc>,
    pub canonical: bool,
    pub kind: String,
}

impl Transaction {
    pub fn from_cmd(cmd: UserCommandWithStatusJson, height: i32, timestamp: u64) -> Self {
        match cmd.data {
            UserCommandJson::SignedCommand(signed_cmd) => {
                let payload = signed_cmd.payload;

                let (sender, receiver, kind) = {
                    match payload.body {
                        SignedCommandPayloadBodyJson::PaymentPayload(payload) => {
                            (payload.source_pk, payload.receiver_pk, "PAYMENT")
                        }
                        SignedCommandPayloadBodyJson::StakeDelegation(payload) => {
                            let StakeDelegationJson::SetDelegate {
                                delegator,
                                new_delegate,
                            } = payload;
                            (delegator, new_delegate, "STAKE_DELEGATION")
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
                    canonical: true,
                    kind: kind.to_owned(),
                }
            }
        }
    }
}

// JSON utility
fn sanitize_json<T: serde::Serialize>(s: T) -> String {
    serde_json::to_string(&s).unwrap().replace('\"', "")
}

#[allow(non_snake_case)]
#[derive(GraphQLInputObject)]
#[graphql(description = "Transaction query input")]
pub struct TransactionQueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub memos: Option<Vec<String>>,
    pub canonical: Option<bool>,
    pub kind: Option<String>,
    // Logical  operators
    pub OR: Option<Box<TransactionQueryInput>>,
    pub AND: Option<Box<TransactionQueryInput>>,
    // Comparison operators
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
    #[graphql(description = "Canonical")]
    fn canonical(&self) -> bool {
        self.canonical
    }
    #[graphql(description = "Kind")]
    fn kind(&self) -> &str {
        &self.kind
    }
}

pub fn get_transactions(
    ctx: &Context,
    query: Option<TransactionQueryInput>,
    limit: Option<i32>,
) -> Vec<Transaction> {
    let limit = limit.unwrap_or(100);
    let limit_idx = usize::try_from(limit).unwrap();

    let mut transactions: Vec<Transaction> = Vec::new();
    for entry in ctx.db.iter_prefix_cf("tx", b"T") {
        let (key, value) = entry.unwrap();

        let key = TransactionKey::from_slice(&key).unwrap();
        let cmd = bcs::from_bytes::<UserCommandWithStatusV1>(&value)
            .unwrap()
            .inner();

        let transaction = Transaction::from_cmd(
            UserCommandWithStatusJson::from(cmd),
            key.height() as i32,
            key.timestamp(),
        );

        // Only apply filters if a query is provided
        // TODO: Generalize filtering
        if let Some(ref query_input) = query {
            if let Some(ref kind) = query_input.kind {
                if transaction.kind != *kind {
                    continue;
                }
            }
            if let Some(canonical) = query_input.canonical {
                if transaction.canonical != canonical {
                    continue;
                }
            }
            if let Some(ref from) = query_input.from {
                if transaction.from != *from {
                    continue;
                }
            }

            if let Some(ref to) = query_input.to {
                if transaction.to != *to {
                    continue;
                }
            }

            if let Some(ref memos) = query_input.memos {
                if !memos.contains(&transaction.memo) {
                    continue;
                }
            }

            if let Some(ref timestamp_gte) = query_input.date_time_gte {
                if transaction.date_time < *timestamp_gte {
                    continue;
                }
            }

            if let Some(ref timestamp_lte) = query_input.date_time_lte {
                if transaction.date_time > *timestamp_lte {
                    continue;
                }
            }
        }
        transactions.push(transaction);
        // stop collecting when reaching limit
        if transactions.len() >= limit_idx {
            break;
        }
    }
    transactions
}
