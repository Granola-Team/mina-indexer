use rust_decimal::Decimal;

/// Converts Nanomina to Mina, strips any trailing zeros, and converts -0 to 0.
/// This function takes a value in Nanomina, converts it to Mina by adjusting
/// the scale, normalizes the decimal representation to remove trailing zeros,
/// and converts any `-0` representation to `0`.
///
/// # Arguments
///
/// * `nanomina` - The amount in Nanomina to be converted.
///
/// # Returns
///
/// A `String` representing the value in Mina with trailing zeros removed.
pub fn nanomina_to_mina(nanomina: u64) -> String {
    let mut dec = Decimal::from(nanomina);
    dec.set_scale(9).unwrap();
    dec.normalize().to_string()
}

#[cfg(test)]
mod utility_function_tests {

    use super::nanomina_to_mina;

    #[test]
    fn test_nanomina_to_mina_conversion() {
        let actual = 1_000_000_001;
        let val = nanomina_to_mina(actual);
        assert_eq!("1.000000001", val);

        let actual = 1_000_000_000;
        let val = nanomina_to_mina(actual);
        assert_eq!("1", val);
    }
}
