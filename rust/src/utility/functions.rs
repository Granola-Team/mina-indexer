use rust_decimal::Decimal;
use std::convert::TryInto;

/// Converts Nanomina to Mina, strips any trailing zeros, and converts -0 to 0.
/// This function takes a value in Nanomina (either u64 or i64), converts it to
/// Mina by adjusting the scale, normalizes the decimal representation to remove
/// trailing zeros, and converts any `-0` representation to `0`.
///
/// # Arguments
///
/// * `nanomina` - The amount in Nanomina to be converted.
///
/// # Returns
///
/// A `String` representing the value in Mina with trailing zeros removed.
pub fn nanomina_to_mina<T: TryInto<i64>>(nanomina: T) -> String {
    let value: i64 = nanomina.try_into().unwrap_or(0);
    let mut dec = Decimal::from(value);
    dec.set_scale(9).unwrap();
    dec.normalize().to_string()
}

#[cfg(test)]
mod utility_function_tests {

    use super::nanomina_to_mina;
    use std::{i64, u64};

    #[test]
    fn test_nanomina_to_mina_conversion_u64() {
        let actual: u64 = 1_000_000_001;
        let val = nanomina_to_mina(actual);
        assert_eq!("1.000000001", val);

        let actual: u64 = 1_000_000_000;
        let val = nanomina_to_mina(actual);
        assert_eq!("1", val);

        // Test i64::MAX as u64
        let max_u64_without_overflow = i64::MAX as u64;
        let val = nanomina_to_mina(max_u64_without_overflow);
        // Expected result for i64::MAX as u64
        let expected = "9223372036.854775807";
        assert_eq!(expected, val);
    }

    #[test]
    fn test_nanomina_to_mina_conversion_i64() {
        let actual: i64 = 1_000_000_001;
        let val = nanomina_to_mina(actual);
        assert_eq!("1.000000001", val);

        let actual: i64 = 1_000_000_000;
        let val = nanomina_to_mina(actual);
        assert_eq!("1", val);

        let negative_actual: i64 = -1_000_000_001;
        let negative_val = nanomina_to_mina(negative_actual);
        assert_eq!("-1.000000001", negative_val);

        // Test i64::MIN
        let min_i64: i64 = i64::MIN;
        let val = nanomina_to_mina(min_i64);
        // Expected result for i64::MIN
        let expected = "-9223372036.854775808"; // Based on i64::MIN scaled and normalized
        assert_eq!(expected, val);
    }
}
