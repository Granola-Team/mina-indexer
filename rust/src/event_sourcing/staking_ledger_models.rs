use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct StakingEntry {
    pub pk: String,
    pub balance: String,
    pub delegate: String,
}

#[cfg(test)]
mod staking_ledger_parsing_tests {
    use super::StakingEntry;
    use std::path::Path;

    #[test]
    fn test_parsing_staking_ledger() {
        let path = Path::new("./src/event_sourcing/test_data/staking_ledgers/mainnet-9-jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        let staking_entries: Vec<StakingEntry> = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(staking_entries.len(), 25_524);
    }
}
