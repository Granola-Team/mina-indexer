use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sonic_rs::{JsonValueMutTrait, Value};
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

pub fn get_cleaned_pcb(file: &str) -> Result<String> {
    let mut file = File::open(file)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    unsafe {
        match sonic_rs::from_slice_unchecked::<Value>(&contents) {
            Ok(mut json_value) => {
                remove_non_utf8_keys(&mut json_value);

                // Serialize back to JSON
                let cleaned_json = sonic_rs::to_string(&json_value).expect("Serialization failed");
                Ok(cleaned_json)
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }
}

/// Recursively removes all "proofs" keys from a `sonic_rs::Value`.
fn remove_non_utf8_keys(value: &mut Value) {
    if let Some(map) = value.as_object_mut() {
        let proofs = "proofs".to_string();
        let sok_digest = "sok_digest".to_string();
        map.remove(&proofs);
        map.remove(&sok_digest);

        for (_, v) in map.iter_mut() {
            remove_non_utf8_keys(v);
        }
    } else if let Some(array) = value.as_array_mut() {
        for v in array.iter_mut() {
            remove_non_utf8_keys(v);
        }
    }
}
#[cfg(test)]
mod remove_proofs_invalid_utf8_tests {
    use super::*;
    use sonic_rs::{json, to_value};

    #[test]
    fn test_invalid_utf8_proofs_key() {
        // Simulate a JSON structure containing invalid UTF-8 bytes
        let raw_json_bytes = b"{
            \"proofs\": \"\xFF\xFE\xFD\",
            \"sok_digest\": \"\xFF\xFE\xFD\",
            \"key1\": \"value2\"
        }";

        // Parse the JSON bytes into a `Value`
        unsafe {
            let mut value = sonic_rs::from_slice_unchecked(raw_json_bytes).expect("Failed to parse JSON");

            // Remove "proofs" key
            remove_non_utf8_keys(&mut value);

            // Construct the expected value
            let expected = to_value(&json!({
                "key1": "value2",
            }))
            .unwrap();

            assert_eq!(value, expected);
        }
    }

    #[test]
    fn test_invalid_utf8_nested_proofs() {
        // Simulate a JSON structure with invalid UTF-8 in a nested object
        let raw_json_bytes = b"{
            \"key1\": {
                \"proofs\": \"\xFF\",
                \"nested\": {
                    \"proofs\": \"\xFF\",
                    \"sok_digest\": \"\xFF\"
                }
            }
        }";

        // Parse the JSON bytes into a `Value`
        unsafe {
            let mut value = sonic_rs::from_slice_unchecked(raw_json_bytes).expect("Failed to parse JSON");

            // Remove "proofs" keys
            remove_non_utf8_keys(&mut value);

            // Construct the expected value
            let expected = to_value(&json!({
                "key1": {
                    "nested": {}
                }
            }))
            .unwrap();

            assert_eq!(value, expected);
        }
    }

    #[test]
    fn test_get_cleaned_pcb_with_invalid_utf8() {
        // Path to the test file with invalid UTF-8
        let test_file = "./src/event_sourcing/test_data/misc_blocks/mainnet-397612-3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK.json";

        // Ensure the file exists
        assert!(Path::new(test_file).exists(), "Test file does not exist");

        // Call the function to clean the JSON
        match get_cleaned_pcb(test_file) {
            Ok(cleaned_json) => {
                println!("Cleaned JSON:\n{}", cleaned_json);

                assert!(!cleaned_json.contains("\"proofs\""), "JSON still contains 'proofs'");
            }
            Err(e) => panic!("Failed to process file: {}", e),
        }
    }
}

pub fn get_top_level_keys_from_json_file(file: &str) -> anyhow::Result<Vec<String>> {
    let contents = get_cleaned_pcb(file)?;

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

fn clean_memo(decoded: &str) -> String {
    // Remove null bytes and leading control characters
    decoded.trim_matches(|c: char| c == '\0' || c.is_control()).to_string()
}

pub fn decode_base58check_to_string(input: &str) -> Result<String> {
    let decoded_bytes = bs58::decode(input)
        .with_check(None) // Verifies the checksum
        .into_vec()
        .map_err(|e| anyhow!("Decoding error: {e}"))?;

    String::from_utf8(decoded_bytes)
        .map(|m| clean_memo(&m))
        .map_err(|e| anyhow!("Invalid UTF-8 sequence {e}"))
}

#[cfg(test)]
mod decode_base58check_to_string_tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn base58check_encode(input: &str) -> String {
        // Convert the input string to bytes
        let input_bytes = input.as_bytes();

        // First SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(input_bytes);
        let first_hash = hasher.finalize();

        // Second SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(first_hash);
        let double_hash = hasher.finalize();

        // Take the first 4 bytes of the second hash as the checksum
        let checksum_bytes = &double_hash[0..4];

        // Append the checksum to the original input
        let mut input_with_checksum = input_bytes.to_vec();
        input_with_checksum.extend_from_slice(checksum_bytes);

        // Encode the result in Base58
        bs58::encode(input_with_checksum).into_string()
    }

    #[test]
    fn test_valid_base58check_string() -> Result<()> {
        let encode = base58check_encode("hello");
        let result = decode_base58check_to_string(&encode).unwrap();
        assert_eq!(result, "hello");
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct TreeNode<T> {
    pub value: T,
    pub children: Vec<TreeNode<T>>,
}

impl<T> TreeNode<T> {
    pub fn new(value: T) -> Self {
        Self { value, children: Vec::new() }
    }

    pub fn add_child(&mut self, child: TreeNode<T>) {
        self.children.push(child);
    }

    pub fn size(&self) -> usize {
        let mut total = 1; // Count the current node
        for child in &self.children {
            total += child.size(); // Recursively count children
        }
        total
    }
}

#[cfg(test)]
mod tree_node_tests {
    use super::*;

    #[test]
    fn test_tree_node_creation() {
        let root = TreeNode::new("root");
        assert_eq!(root.value, "root");
        assert!(root.children.is_empty());
    }

    #[test]
    fn test_tree_node_add_child() {
        let mut root = TreeNode::new("root");
        let child1 = TreeNode::new("child1");
        let child2 = TreeNode::new("child2");

        root.add_child(child1.clone());
        root.add_child(child2.clone());

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].value, "child1");
        assert_eq!(root.children[1].value, "child2");
    }

    #[test]
    fn test_tree_node_size_single_node() {
        let root = TreeNode::new("root");
        assert_eq!(root.size(), 1);
    }

    #[test]
    fn test_tree_node_size_with_children() {
        let mut root = TreeNode::new("root");
        let mut child1 = TreeNode::new("child1");
        let child2 = TreeNode::new("child2");
        let grandchild = TreeNode::new("grandchild");

        child1.add_child(grandchild);
        root.add_child(child1);
        root.add_child(child2);

        // The structure is:
        // root
        // ├── child1
        // │   └── grandchild
        // └── child2

        assert_eq!(root.size(), 4);
    }

    #[test]
    fn test_tree_node_with_different_types() {
        let mut root = TreeNode::new(1);
        let child = TreeNode::new(2);
        root.add_child(child);

        assert_eq!(root.value, 1);
        assert_eq!(root.children[0].value, 2);
    }

    #[test]
    fn test_tree_node_deep_hierarchy() {
        let mut root = TreeNode::new("root");
        let mut level1 = TreeNode::new("level1");
        let mut level2 = TreeNode::new("level2");
        let level3 = TreeNode::new("level3");

        level2.add_child(level3);
        level1.add_child(level2);
        root.add_child(level1);

        // The structure is:
        // root
        // └── level1
        //     └── level2
        //         └── level3

        assert_eq!(root.size(), 4);
        assert_eq!(root.children[0].children[0].children[0].value, "level3");
    }
}
