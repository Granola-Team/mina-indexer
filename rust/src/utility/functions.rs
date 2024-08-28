use std::time::Duration;

use rust_decimal::Decimal;

/// Pretty print duration for use in logs.
///
/// Example: 1d 2h 3m 4s
pub fn pretty_print_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds == 0 {
        return "0 seconds".to_string();
    }

    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if secs > 0 {
        parts.push(format!("{}s", secs));
    }

    parts.join(" ")
}

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

    use super::*;

    #[test]
    fn test_nanomina_to_mina_conversion() {
        let actual = 1_000_000_001;
        let val = nanomina_to_mina(actual);
        assert_eq!("1.000000001", val);

        let actual = 1_000_000_000;
        let val = nanomina_to_mina(actual);
        assert_eq!("1", val);
    }

    #[test]
    fn test_pretty_print_duration() {
        assert_eq!(pretty_print_duration(Duration::from_secs(0)), "0 seconds");
        assert_eq!(pretty_print_duration(Duration::from_secs(1)), "1s");
        assert_eq!(pretty_print_duration(Duration::from_secs(60)), "1m");
        assert_eq!(pretty_print_duration(Duration::from_secs(3661)), "1h 1m 1s");
        assert_eq!(
            pretty_print_duration(Duration::from_secs(86400 + 3661)),
            "1d 1h 1m 1s"
        );
        assert_eq!(
            pretty_print_duration(Duration::from_secs(172800 + 7200 + 120 + 5)),
            "2d 2h 2m 5s"
        );
    }
}
