//! GraphQL long representation

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};

#[derive(Debug, Clone)]
pub struct Long(pub String);

#[Scalar]
impl ScalarType for Long {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(Long(s)),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}
