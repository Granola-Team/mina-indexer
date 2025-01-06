use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct HardForkLedgerAccount {
    pub nonce: String,
    pub balance: String,
    pub delegate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HardForkLedger {
    #[serde(flatten)]
    pub accounts: HashMap<String, HardForkLedgerAccount>,
}

#[cfg(test)]
mod hard_fork_ledger_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_hard_fork_ledger() {
        // Provide the path to the JSON file
        let file_path = PathBuf::from("./src/data/ledger_359604.json");

        // Ensure the file exists before testing
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read genesis_ledger.json file");

        let ledger: HardForkLedger = sonic_rs::from_str(&file_content).expect("Failed to parse");
        assert_eq!(ledger.accounts.len(), 228174);

        let acct = ledger.accounts.get("B62qiTKyEZ4Lts4DesZZwKYkZKPGD3FBPkMEpfGWC8KuhenMNyts1nd").unwrap();
        assert_eq!(acct.nonce, "3");
        assert_eq!(acct.balance, "0");
        assert_eq!(acct.delegate.as_ref().unwrap(), "B62qiTKyEZ4Lts4DesZZwKYkZKPGD3FBPkMEpfGWC8KuhenMNyts1nd");
    }
}
