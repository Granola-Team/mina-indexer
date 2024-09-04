use rust_decimal::Decimal;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

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

/// Calculate the total size of the file paths
pub fn calculate_total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().fold(0, |acc, p| {
        match p.metadata() {
            Ok(metadata) => acc + metadata.len(),
            Err(_) => acc, // Skip files that can't be read
        }
    })
}

pub fn is_valid_file_name(path: &Path, hash_validator: &dyn Fn(&str) -> bool) -> bool {
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        if ext != "json" {
            return false;
        }
    } else {
        return false;
    }

    if let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) {
        let parts: Vec<&str> = file_stem.split('-').collect();

        match parts.as_slice() {
            // mainnet-<hash>.json
            [_, hash] => hash_validator(hash),

            // mainnet-<number>-<hash>.json
            [_, epoch_str, hash] => epoch_str.parse::<u32>().is_ok() && hash_validator(hash),

            _ => false,
        }
    } else {
        false
    }
}

#[cfg(test)]
mod utility_function_tests {
    use super::*;
    use crate::block::is_valid_state_hash;
    use std::{fs::File, io::Write, path::Path};
    use tempfile::TempDir;

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

    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_empty_vector() {
        let paths: Vec<PathBuf> = vec![];
        assert_eq!(calculate_total_size(&paths), 0);
    }

    #[test]
    fn test_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "test1.txt", "Hello, World!");
        let paths = vec![file_path];
        assert_eq!(calculate_total_size(&paths), 13); // "Hello, World!" is 13
                                                      // bytes
    }

    #[test]
    fn test_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = create_temp_file(&temp_dir, "test1.txt", "Hello");
        let file2 = create_temp_file(&temp_dir, "test2.txt", "World");
        let file3 = create_temp_file(&temp_dir, "test3.txt", "Rust");
        let paths = vec![file1, file2, file3];
        assert_eq!(calculate_total_size(&paths), 14); // "Hello" + "World" +
                                                      // "Rust" = 5 + 5 + 4 = 14
                                                      // bytes
    }

    #[test]
    fn test_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let existing_file = create_temp_file(&temp_dir, "existing.txt", "I exist");
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");
        let paths = vec![existing_file, nonexistent_file];
        assert_eq!(calculate_total_size(&paths), 7); // Only counts "I exist" (7
                                                     // bytes)
    }

    #[test]
    fn test_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = create_temp_file(&temp_dir, "empty.txt", "");
        let paths = vec![empty_file];
        assert_eq!(calculate_total_size(&paths), 0);
    }

    #[test]
    fn test_is_valid_file_name() {
        // Valid cases
        assert!(is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234.json"),
            &is_valid_state_hash
        ));

        assert!(is_valid_file_name(
            Path::new("mainnet-3Nabcdef12345678901234567890123456789012345678901234.json"),
            &is_valid_state_hash
        ));

        // Invalid cases
        assert!(!is_valid_file_name(
            Path::new("mainnet-42-abcdef1234567890123456789012345678901234567890123456.json"), /* Invalid hash (does not start with 3N) */
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef1234.json"), // Hash too short
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42.json"), // Missing hash part
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234.txt"), /* Invalid extension */
            &is_valid_state_hash
        ));

        assert!(!is_valid_file_name(
            Path::new("mainnet-42-3Nabcdef12345678901234567890123456789012345678901234-123.json"), /* Too many parts */
            &is_valid_state_hash
        ));
    }
}
