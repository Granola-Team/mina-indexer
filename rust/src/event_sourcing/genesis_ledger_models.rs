use super::payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination};
use crate::constants::MINA_TOKEN_ID;
use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct Genesis {
    pub genesis_state_timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Timing {
    pub initial_minimum_balance: String,
    pub cliff_time: String,
    pub cliff_amount: String,
    pub vesting_period: String,
    pub vesting_increment: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    pub pk: String,
    pub balance: String,
    pub delegate: Option<String>, // Optional field
    pub timing: Option<Timing>,   // Optional field
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ledger {
    pub name: String,
    pub accounts: Vec<Account>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenesisLedger {
    pub genesis: Genesis,
    pub ledger: Ledger,
}

impl GenesisLedger {
    pub fn get_accounting_double_entries(&self) -> Vec<DoubleEntryRecordPayload> {
        self.ledger
            .accounts
            .iter()
            .map(|account| {
                let balance = BigDecimal::from_str(&account.balance).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                DoubleEntryRecordPayload {
                    height: 0,
                    state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
                    ledger_destination: LedgerDestination::BlockchainLedger,
                    lhs: vec![AccountingEntry {
                        counterparty: account.pk.to_string(),
                        transfer_type: "InitialDistribution".to_string(),
                        entry_type: AccountingEntryType::Debit,
                        account: format!("MinaGenesisLedger#{}", account.pk),
                        account_type: AccountingEntryAccountType::VirtualAddess,
                        amount_nanomina: balance.to_u64().expect("Number too large for u64"),
                        timestamp: 1615939200,
                        token_id: MINA_TOKEN_ID.to_string(),
                    }],
                    rhs: vec![AccountingEntry {
                        counterparty: format!("MinaGenesisLedger#{}", account.pk),
                        transfer_type: "InitialDistribution".to_string(),
                        entry_type: AccountingEntryType::Credit,
                        account: account.pk.to_string(),
                        account_type: AccountingEntryAccountType::BlockchainAddress,
                        amount_nanomina: balance.to_u64().expect("Number too large for u64"),
                        timestamp: 1615939200,
                        token_id: MINA_TOKEN_ID.to_string(),
                    }],
                    accessed_accounts: None,
                }
            })
            .collect::<Vec<DoubleEntryRecordPayload>>()
    }

    pub fn get_accounts(&self) -> Vec<String> {
        self.ledger.accounts.iter().map(|a| a.pk.to_string()).collect()
    }
}

#[cfg(test)]
mod gensis_ledger_tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_load_json_from_file() {
        // Provide the path to the JSON file
        let file_path = PathBuf::from("./src/data/genesis_ledger.json");

        // Ensure the file exists before testing
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read genesis_ledger.json file");

        // Test the load_json_from_file function
        let parsed_data: GenesisLedger = sonic_rs::from_str(&file_content).unwrap();

        // Assertions
        assert_eq!(parsed_data.genesis.genesis_state_timestamp, "2021-03-17T00:00:00Z");
        assert_eq!(parsed_data.ledger.name, "mainnet");

        let accounts = &parsed_data.ledger.accounts;
        assert_eq!(accounts.len(), 1675);

        let first_account = &accounts[0];
        assert_eq!(first_account.pk, "B62qmqMrgPshhHKLJ7DqWn1KeizEgga5MuGmWb2bXajUnyivfeMW6JE");
        assert_eq!(first_account.balance, "372093");
        assert_eq!(
            first_account.delegate.as_ref().unwrap(),
            "B62qrecVjpoZ4Re3a5arN6gXZ6orhmj1enUtA887XdG5mtZfdUbBUh4"
        );
        assert!(first_account.timing.is_some());

        let second_account = &accounts[1];
        assert_eq!(second_account.pk, "B62qmVHmj3mNhouDf1hyQFCSt3ATuttrxozMunxYMLctMvnk5y7nas1");
        assert_eq!(second_account.balance, "230400");
        assert!(second_account.delegate.is_some());
        assert!(second_account.timing.is_some());
    }
}
