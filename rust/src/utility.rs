use anyhow::anyhow;
use sonic_rs::Value;
use std::{fs::File, io::Read, path::Path};

pub fn extract_height_and_hash(path: &Path) -> (u32, &str) {
    let filename = path.file_stem().and_then(|x| x.to_str()).expect("Failed to extract filename from path");

    let mut parts = filename.split('-');

    match (parts.next(), parts.next(), parts.next()) {
        (Some(_), Some(height_str), Some(hash_part)) => {
            let block_height = height_str.parse::<u32>().expect("Failed to parse block height");
            let hash = hash_part.split('.').next().expect("Failed to parse the hash");
            (block_height, hash)
        }
        _ => panic!("Filename format is invalid {}", filename),
    }
}

#[cfg(test)]
mod extract_height_and_hash_tests {
    use super::extract_height_and_hash;
    use std::path::Path;

    #[test]
    fn test_extract_height_and_hash_valid_format() {
        let path = Path::new("prefix-1234-hashpart.extension");
        let (height, hash) = extract_height_and_hash(path);
        assert_eq!(height, 1234);
        assert_eq!(hash, "hashpart");
    }

    #[test]
    #[should_panic(expected = "Filename format is invalid")]
    fn test_extract_height_and_hash_invalid_format() {
        let path = Path::new("invalid-format");
        let _ = extract_height_and_hash(path); // This should panic
    }

    #[test]
    #[should_panic(expected = "Failed to parse block height")]
    fn test_extract_height_and_hash_non_numeric_height() {
        let path = Path::new("prefix-notanumber-hash.extension");
        let _ = extract_height_and_hash(path); // This should panic due to non-numeric height
    }

    #[test]
    #[should_panic(expected = "Failed to extract filename from path")]
    fn test_extract_height_and_hash_empty_path() {
        let path = Path::new("");
        let _ = extract_height_and_hash(path); // This should panic due to missing filename
    }
}

pub fn get_top_level_keys_from_json_file(file: &str) -> anyhow::Result<Vec<String>> {
    let mut file = File::open(file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the JSON file using sonic-rs
    let json_value: Value = sonic_rs::from_str(&contents)?;

    // Check if the top-level element is an object, then get keys
    if let Some(obj) = json_value.into_object() {
        let keys = obj.into_iter().map(|(key, _)| key.to_string()).collect::<Vec<_>>();
        Ok(keys)
    } else {
        Err(anyhow!("Top-level JSON structure is not an object"))
    }
}

#[cfg(test)]
mod get_top_level_keys_from_json_file_tests {
    use super::*;
    use anyhow::Result;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_top_level_keys_from_json_file_with_object() -> Result<()> {
        // Create a temporary JSON file with known top-level keys
        let mut temp_file = NamedTempFile::new()?;
        writeln!(
            temp_file,
            r#"{{
                "name": "Alice",
                "age": 30,
                "city": "Wonderland"
            }}"#
        )?;

        // Call the function and check the result
        let keys = get_top_level_keys_from_json_file(temp_file.path().to_str().unwrap())?;
        let expected_keys = vec!["name".to_string(), "age".to_string(), "city".to_string()];
        assert_eq!(keys, expected_keys);

        Ok(())
    }

    #[test]
    fn test_get_top_level_keys_from_json_file_with_non_object() -> Result<()> {
        // Create a temporary JSON file with a non-object top-level structure
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, r#"[1, 2, 3]"#)?;

        // Call the function and expect an error
        let result = get_top_level_keys_from_json_file(temp_file.path().to_str().unwrap());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_get_top_level_keys_from_json_file_with_empty_object() -> Result<()> {
        // Create a temporary JSON file with an empty object
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, r#"{{}}"#)?;

        // Call the function and expect an empty vector of keys
        let keys = get_top_level_keys_from_json_file(temp_file.path().to_str().unwrap())?;
        let expected_keys: Vec<String> = vec![];
        assert_eq!(keys, expected_keys);

        Ok(())
    }
}

pub struct Throttler {
    count: usize,
    interval: usize,
}

impl Throttler {
    pub fn new(interval: usize) -> Self {
        Throttler { count: 0, interval }
    }

    pub fn should_invoke(&mut self) -> bool {
        self.count += 1;
        if self.count % self.interval == 0 {
            self.count = 0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod throttler_tests {
    use super::Throttler;

    #[test]
    fn test_throttler_initial_state() {
        let mut throttler = Throttler::new(3);

        // Throttler should not invoke on the first call
        assert!(!throttler.should_invoke(), "Throttler should not invoke on first call");
    }

    #[test]
    fn test_throttler_invocation() {
        let mut throttler = Throttler::new(3);

        // Call should not invoke until the third one
        assert!(!throttler.should_invoke(), "First call should not invoke");
        assert!(!throttler.should_invoke(), "Second call should not invoke");
        assert!(throttler.should_invoke(), "Third call should invoke");
    }

    #[test]
    fn test_throttler_resets_after_invocation() {
        let mut throttler = Throttler::new(3);

        // Invoke the throttler until it resets
        assert!(!throttler.should_invoke(), "First call should not invoke");
        assert!(!throttler.should_invoke(), "Second call should not invoke");
        assert!(throttler.should_invoke(), "Third call should invoke");

        // Ensure it resets after the interval
        assert!(!throttler.should_invoke(), "Fourth call should not invoke");
        assert!(!throttler.should_invoke(), "Fifth call should not invoke");
        assert!(throttler.should_invoke(), "Sixth call should invoke");
    }

    #[test]
    fn test_throttler_handles_large_intervals() {
        let mut throttler = Throttler::new(10);

        for i in 1..10 {
            assert!(!throttler.should_invoke(), "Call {} should not invoke for interval 10", i);
        }

        assert!(throttler.should_invoke(), "Tenth call should invoke for interval 10");
    }

    #[test]
    fn test_throttler_interval_of_one() {
        let mut throttler = Throttler::new(1);

        // Every call should invoke for an interval of 1
        assert!(throttler.should_invoke(), "First call should invoke for interval 1");
        assert!(throttler.should_invoke(), "Second call should invoke for interval 1");
        assert!(throttler.should_invoke(), "Third call should invoke for interval 1");
    }

    #[test]
    fn test_throttler_multiple_invocations() {
        let mut throttler = Throttler::new(3);

        // First cycle
        assert!(!throttler.should_invoke(), "First call should not invoke in first cycle");
        assert!(!throttler.should_invoke(), "Second call should not invoke in first cycle");
        assert!(throttler.should_invoke(), "Third call should invoke in first cycle");

        // Second cycle
        assert!(!throttler.should_invoke(), "First call should not invoke in second cycle");
        assert!(!throttler.should_invoke(), "Second call should not invoke in second cycle");
        assert!(throttler.should_invoke(), "Third call should invoke in second cycle");

        // Third cycle
        assert!(!throttler.should_invoke(), "First call should not invoke in third cycle");
        assert!(!throttler.should_invoke(), "Second call should not invoke in third cycle");
        assert!(throttler.should_invoke(), "Third call should invoke in third cycle");
    }
}
