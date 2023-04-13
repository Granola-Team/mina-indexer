use std::collections::HashMap;

use mina_serialization_types::{signatures::PublicKeyJson, v1::PublicKeyV1};
use serde::{Deserialize, Serialize};

use super::{account::Account, Ledger};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pk: PublicKeyJson,
    sk: Option<PublicKeyJson>,
    balance: String,
    delegate: Option<PublicKeyJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisLedger {
    name: String,
    num_accounts: u64,
    accounts: Vec<GenesisAccount>,
}

impl Into<Ledger> for GenesisLedger {
    fn into(self) -> Ledger {
        let mut accounts = HashMap::new();
        for genesis_account in self.accounts {
            let balance = match str::parse::<f32>(&genesis_account.balance) {
                Ok(float) => (float * 1e9) as u64,
                Err(_) => 0,
            };

            accounts.insert(
                PublicKeyV1::from(genesis_account.pk.clone()).into(),
                Account {
                    public_key: PublicKeyV1::from(genesis_account.pk).into(),
                    delegate: genesis_account
                        .delegate
                        .clone()
                        .map(|delegate| PublicKeyV1::from(delegate).into()),
                    balance,
                },
            );

            if let Some(delegate) = genesis_account.delegate {
                let delegate_public_key = PublicKeyV1::from(delegate);
                if let None = accounts.get(&delegate_public_key.clone().into()) {
                    let account = Account::empty(delegate_public_key.clone());
                    accounts.insert(delegate_public_key.into(), account);
                }
            }
        }

        Ledger { accounts }
    }
}

#[cfg(test)]
pub mod tests {
    use super::GenesisLedger;

    const GENESIS_LEDGER_JSON: &'static str = include_str!("./genesis_ledger.json");

    #[test]
    pub fn genesis_ledger_deserializes() {
        let _genesis_ledger: GenesisLedger = serde_json::from_str(GENESIS_LEDGER_JSON).unwrap();
    }
}
