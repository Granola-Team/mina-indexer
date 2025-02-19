// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(test, feature = "browser"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
mod tests {
    use mina_indexer::protocol::serialization_types::common::CharJson;
    use wasm_bindgen_test::*;

    #[allow(dead_code)]
    #[wasm_bindgen_test]
    fn char_json_wasm() {
        char_json().unwrap()
    }

    #[test]
    fn char_json() -> anyhow::Result<()> {
        let json_str = r#""\u0001""#;
        let char: CharJson = serde_json::from_str(json_str)?;
        assert_eq!(char.0, 1);
        let json_str_from_char = serde_json::to_string(&char)?;
        assert_eq!(json_str, json_str_from_char.as_str());
        Ok(())
    }
}
