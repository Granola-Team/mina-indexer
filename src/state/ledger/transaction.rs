use serde_json::Value;

use super::PublicKey;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Transaction {
    pub source: PublicKey,
    pub receiver: PublicKey,
    pub amount: u64,
}

impl Transaction {
    pub fn from_commands(commands: &[Value]) -> Vec<Self> {
        commands
            .iter()
            .map(|command| {
                let payload_body = command
                    .as_object()?
                    .get("data")?
                    .as_array()?
                    .get(1)?
                    .as_object()?
                    .get("payload")?
                    .as_object()?
                    .get("body")?
                    .as_array()?
                    .get(1)?
                    .as_object()?;

                let source_pk = payload_body.get("source_pk")?.as_str()?.to_string();

                let receiver_pk = payload_body.get("receiver_pk")?.as_str()?.to_string();

                let amount = payload_body.get("amount")?.as_u64()?;

                Some(Transaction {
                    source: source_pk,
                    receiver: receiver_pk,
                    amount,
                })
            })
            .flatten()
            .collect()
    }
}
