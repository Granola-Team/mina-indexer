use crate::{constants::MINA_SCALE, utility::functions::nanomina_to_mina};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct SignedAmount(pub i64);

impl SignedAmount {
    pub fn new(amount: i64) -> Self {
        Self(amount * MINA_SCALE as i64)
    }
}

impl ToString for SignedAmount {
    fn to_string(&self) -> String {
        nanomina_to_mina(self.0)
    }
}

impl Add<SignedAmount> for SignedAmount {
    type Output = SignedAmount;

    fn add(self, rhs: SignedAmount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<i64> for SignedAmount {
    type Output = SignedAmount;

    fn add(self, rhs: i64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<SignedAmount> for SignedAmount {
    type Output = SignedAmount;

    fn sub(self, rhs: SignedAmount) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Sub<i64> for SignedAmount {
    type Output = SignedAmount;

    fn sub(self, rhs: i64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl From<u64> for SignedAmount {
    fn from(value: u64) -> Self {
        SignedAmount(
            value
                .try_into()
                .expect("Expected value to be <= 'i64::MAX as u64'"),
        )
    }
}

#[cfg(test)]
mod signed_amount_tests {
    use super::SignedAmount;
    use std::{i64, u64};

    #[test]
    fn test_signed_amount_creation() {
        let amount = SignedAmount::new(1);
        assert_eq!(amount.0, 1_000_000_000); // Assuming MINA_SCALE is 1e9

        let amount = SignedAmount::new(-1);
        assert_eq!(amount.0, -1_000_000_000);

        let amount = SignedAmount::new(0);
        assert_eq!(amount.0, 0);
    }

    #[test]
    fn test_signed_amount_to_string() {
        let amount = SignedAmount::new(1);
        assert_eq!(amount.to_string(), "1");

        let amount = SignedAmount::new(-1);
        assert_eq!(amount.to_string(), "-1");

        let amount = SignedAmount::new(0);
        assert_eq!(amount.to_string(), "0");
    }

    #[test]
    fn test_signed_amount_addition() {
        let amount1 = SignedAmount::new(1);
        let amount2 = SignedAmount::new(2);
        let result = amount1 + amount2;
        assert_eq!(result.0, 3_000_000_000); // 1e9 + 2e9 = 3e9

        let amount3 = SignedAmount::new(-1);
        let result = amount1 + amount3;
        assert_eq!(result.0, 0); // 1e9 + (-1e9) = 0
    }

    #[test]
    fn test_signed_amount_subtraction() {
        let amount1 = SignedAmount::new(3);
        let amount2 = SignedAmount::new(1);
        let result = amount1 - amount2;
        assert_eq!(result.0, 2_000_000_000); // 3e9 - 1e9 = 2e9

        let amount3 = SignedAmount::new(-1);
        let result = amount1 - amount3;
        assert_eq!(result.0, 4_000_000_000); // 3e9 - (-1e9) = 4e9
    }

    #[test]
    fn test_signed_amount_addition_with_i64() {
        let amount1 = SignedAmount::new(1);
        let result = amount1 + 2_000_000_000_i64;
        assert_eq!(result.0, 3_000_000_000); // 1e9 + 2e9 = 3e9
    }

    #[test]
    fn test_signed_amount_subtraction_with_i64() {
        let amount1 = SignedAmount::new(3);
        let result = amount1 - 1_000_000_000_i64;
        assert_eq!(result.0, 2_000_000_000); // 3e9 - 1e9 = 2e9
    }

    #[test]
    fn test_signed_amount_from_u64() {
        let amount: SignedAmount = SignedAmount::from(1_000_000_000_u64);
        assert_eq!(amount.0, 1_000_000_000); // Direct conversion from u64 to i64

        // Test maximum convertible u64 value
        let max_u64 = i64::MAX as u64;
        let amount: SignedAmount = SignedAmount::from(max_u64);
        assert_eq!(amount.0, max_u64 as i64);

        // Test overflow scenario
        let overflow_u64 = u64::MAX;
        let result = std::panic::catch_unwind(|| SignedAmount::from(overflow_u64));
        assert!(result.is_err()); // Should panic because of overflow
    }
}
