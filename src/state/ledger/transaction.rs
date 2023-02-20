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
            .filter_map(|command| {
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

                // parse better
                let amount = payload_body.get("amount")?.as_str()?.parse::<u64>().ok()?;

                Some(Transaction {
                    source: source_pk,
                    receiver: receiver_pk,
                    amount,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::Transaction;

    const COMMANDS_WITH_ONE_TRANSACTION_JSON: &'static str =
        include_str!("../../../tests/data/commands/commands_with_one_transaction.json");

    #[test]
    fn transaction_from_commands_deserializes() {
        let commands: Vec<Value> =
            serde_json::from_str::<Value>(COMMANDS_WITH_ONE_TRANSACTION_JSON)
                .expect("json is valid")
                .as_array()
                .expect("json is an array")
                .to_owned();
        let transactions = Transaction::from_commands(&commands);
        assert_eq!(transactions.len(), 1);
        let transaction = transactions.get(0).expect("transaction 0 exists");
        assert_eq!(
            transaction.source,
            "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".to_string()
        );
        assert_eq!(
            transaction.receiver,
            "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM".to_string()
        );
        assert_eq!(transaction.amount, 1000);
    }
}
