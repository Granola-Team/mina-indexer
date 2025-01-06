use super::payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination};
use crate::constants::MINA_TOKEN_ID;
use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

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

impl HardForkLedger {
    pub fn get_accounting_double_entries(&self) -> Vec<DoubleEntryRecordPayload> {
        self.accounts
            .iter()
            .map(|(pk, account)| {
                let balance = BigDecimal::from_str(&account.balance).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                DoubleEntryRecordPayload {
                    height: 359604,
                    state_hash: "n/a".to_string(),
                    ledger_destination: LedgerDestination::BlockchainLedger,
                    lhs: vec![AccountingEntry {
                        counterparty: pk.to_string(),
                        transfer_type: "HardForkLedger".to_string(),
                        entry_type: AccountingEntryType::Debit,
                        account: format!("HardForkLedger#{}", pk),
                        account_type: AccountingEntryAccountType::VirtualAddess,
                        amount_nanomina: balance.to_u64().expect("Number too large for u64"),
                        timestamp: 0,
                        token_id: MINA_TOKEN_ID.to_string(),
                    }],
                    rhs: vec![AccountingEntry {
                        counterparty: format!("HardForkLedger#{}", pk),
                        transfer_type: "HardForkLedger".to_string(),
                        entry_type: AccountingEntryType::Credit,
                        account: pk.to_string(),
                        account_type: AccountingEntryAccountType::BlockchainAddress,
                        amount_nanomina: balance.to_u64().expect("Number too large for u64"),
                        timestamp: 0,
                        token_id: MINA_TOKEN_ID.to_string(),
                    }],
                    expected_balances: None,
                }
            })
            .collect::<Vec<DoubleEntryRecordPayload>>()
    }
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
