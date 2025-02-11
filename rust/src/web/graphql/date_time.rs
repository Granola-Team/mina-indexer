//! GraphQL date time representation

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DateTime(pub String);

impl DateTime {
    pub fn timestamp_millis(&self) -> i64 {
        let date_time = chrono::DateTime::parse_from_rfc3339(&self.0).expect("RFC3339 date time");
        date_time.timestamp_millis()
    }
}

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(DateTime(s)),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::DateTime;
    use crate::constants::*;

    #[test]
    fn date_time_millis() {
        assert_eq!(
            DateTime("1970-01-01T00:00:00.000Z".into()).timestamp_millis(),
            0
        );
        assert_eq!(
            DateTime("2021-03-17T00:00:00.000Z".into()).timestamp_millis(),
            1615939200000
        );
        assert_eq!(
            DateTime("2024-06-02T00:00:00.000Z".into()).timestamp_millis(),
            1717286400000
        );
        assert_eq!(
            DateTime("2024-06-03T00:00:00.000Z".into()).timestamp_millis(),
            1717372800000
        );
        assert_eq!(
            DateTime("2024-06-05T00:00:00.000Z".into()).timestamp_millis(),
            1717545600000
        );
    }

    #[test]
    fn date_time_to_global_slot() {
        assert_eq!(millis_to_global_slot(MAINNET_GENESIS_TIMESTAMP as i64), 0);
        assert_eq!(
            millis_to_global_slot(HARDFORK_GENESIS_TIMESTAMP as i64),
            564480
        );

        let dt_millis = DateTime("2024-06-02T00:00:00.000Z".into()).timestamp_millis();
        assert_eq!(millis_to_global_slot(dt_millis), 563040);

        let dt_millis = DateTime("2024-06-03T00:00:00.000Z".into()).timestamp_millis();
        assert_eq!(millis_to_global_slot(dt_millis), 563520);
    }
}
