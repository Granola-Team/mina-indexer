use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

#[derive(Serialize, Deserialize)]
pub struct StakingEntry {
    pub pk: String,
    pub balance: String,
    pub delegate: String,
}

#[derive(Serialize, Deserialize)]
pub struct StakingLedger {
    entries: Vec<StakingEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StakeSummary {
    pub delegate: String,
    pub stake: u64,
    pub total_staked: u64,
    pub delegators: HashSet<String>,
}

impl StakeSummary {
    pub fn get_stake_percentage(&self) -> f32 {
        let percentage = self.stake.to_f32().unwrap() / self.total_staked.to_f32().unwrap() * 100_f32;
        format!("{:.2}", percentage).parse::<f32>().unwrap()
    }
}

impl StakingLedger {
    pub fn new(entries: Vec<StakingEntry>) -> Self {
        Self { entries }
    }

    pub fn get_total_staked(&self) -> u64 {
        self.entries.iter().fold(0_u64, |total_stake, staking_entry| {
            let balance_nanomina = BigDecimal::from_str(&staking_entry.balance).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
            total_stake + balance_nanomina.to_u64().unwrap()
        })
    }

    pub fn get_stakes(&self, total_staked: u64) -> HashMap<String, StakeSummary> {
        let mut stakes: HashMap<String, StakeSummary> = HashMap::new();
        for staking_entry in self.entries.iter() {
            let key = staking_entry.delegate.to_string();
            let balance_nanomina = BigDecimal::from_str(&staking_entry.balance).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
            if !stakes.contains_key(&key) {
                stakes.insert(
                    key.to_string(),
                    StakeSummary {
                        delegate: key.to_string(),
                        stake: 0,
                        total_staked,
                        delegators: HashSet::new(),
                    },
                );
            }
            if let Some(entry) = stakes.get_mut(&key) {
                entry.delegators.insert(staking_entry.pk.to_string());
                entry.stake += balance_nanomina.to_u64().unwrap()
            };
        }
        stakes
    }
}

#[cfg(test)]
mod staking_ledger_parsing_tests {
    use super::{StakeSummary, StakingEntry, StakingLedger};
    use bigdecimal::{BigDecimal, ToPrimitive};
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, str::FromStr};

    #[derive(Serialize, Deserialize, Debug)]
    struct StakingEntryFixture {
        pub stake: String,
        pub total_stake: String,
        pub delegators: String,
    }

    impl StakingEntryFixture {
        pub fn get_stake(&self) -> u64 {
            let stake = self.stake.replace(",", "");
            let stake = BigDecimal::from_str(&stake).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
            stake.to_u64().unwrap()
        }

        pub fn get_total_stake_percentage(&self) -> f32 {
            self.total_stake.parse::<f32>().unwrap()
        }

        pub fn get_delegators(&self) -> u64 {
            self.delegators.parse::<u64>().unwrap()
        }
    }

    #[test]
    fn test_parsing_staking_ledger() {
        const STAKING_LEDGER_PATH: &str = "./src/event_sourcing/test_data/staking_ledgers/mainnet-9-jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk.json";
        const EXPECTED_DATA_PATH: &str = "./src/event_sourcing/test_data/staking_ledgers/mainnet-9-staking-data.json";

        let staking_entries: Vec<StakingEntry> = {
            let file_content = std::fs::read_to_string(STAKING_LEDGER_PATH).expect("Failed to read staking ledger file");
            sonic_rs::from_str(&file_content).expect("Failed to parse staking ledger JSON")
        };

        let expected_staking_entries: HashMap<String, StakingEntryFixture> = {
            let file_content = std::fs::read_to_string(EXPECTED_DATA_PATH).expect("Failed to read expected data file");
            sonic_rs::from_str(&file_content).expect("Failed to parse expected data JSON")
        };

        assert_eq!(staking_entries.len(), 25_524);

        let staking_ledger = StakingLedger::new(staking_entries);
        let staking_summary = staking_ledger.get_stakes(staking_ledger.get_total_staked());

        for (key, expected_staking_entry) in expected_staking_entries.iter() {
            let entry = staking_summary
                .get(key)
                .unwrap_or_else(|| panic!("Missing staking summary entry for key: {}", key));
            assert_stake_summary_matches(entry, expected_staking_entry);
        }
    }

    fn assert_stake_summary_matches(actual: &StakeSummary, expected: &StakingEntryFixture) {
        assert_eq!(actual.get_stake_percentage(), expected.get_total_stake_percentage());
        assert_eq!(actual.stake, expected.get_stake());
        assert_eq!(actual.delegators.len() as u64, expected.get_delegators());
    }
}
