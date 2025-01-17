use super::{
    account::{Account, Permissions, ReceiptChainHash, Timing},
    amount::Amount,
    nonce::Nonce,
    public_key::PublicKey,
    token::TokenAddress,
    Ledger, TokenLedger,
};
use crate::{block::genesis::GenesisBlock, constants::*, mina_blocks::v2::ZkappAccount};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisTimestamp {
    pub genesis_state_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisRoot {
    pub genesis: GenesisTimestamp,
    pub ledger: GenesisAccounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub pk: String,
    pub balance: String,
    pub nonce: Option<Nonce>,
    pub delegate: Option<String>,
    pub token: Option<u64>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<String>,
    pub permissions: Option<Permissions>,
    pub timing: Option<GenesisAccountTiming>,
    pub zkapp: Option<ZkappAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccountTiming {
    pub initial_minimum_balance: String,
    pub cliff_time: String,
    pub cliff_amount: String,
    pub vesting_period: String,
    pub vesting_increment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccounts {
    pub name: String,
    pub accounts: Vec<GenesisAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisLedger {
    ledger: TokenLedger,
}

impl std::str::FromStr for GenesisRoot {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(|e| anyhow!("Error parsing genesis root: {e}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisConstants {
    pub k: Option<u32>,
    pub slots_per_epoch: Option<u32>,
    pub slots_per_sub_window: Option<u32>,
    pub delta: Option<u32>,
    pub txpool_max_size: Option<u32>,
}

impl GenesisConstants {
    pub fn override_with(&mut self, constants: Self) {
        let Self {
            delta,
            k,
            slots_per_epoch,
            slots_per_sub_window,
            txpool_max_size,
        } = constants;

        if delta.is_some() {
            self.delta = delta;
        }
        if k.is_some() {
            self.k = k;
        }
        if slots_per_epoch.is_some() {
            self.slots_per_epoch = slots_per_epoch;
        }
        if slots_per_sub_window.is_some() {
            self.slots_per_sub_window = slots_per_sub_window;
        }
        if txpool_max_size.is_some() {
            self.txpool_max_size = txpool_max_size;
        }
    }
}

impl std::default::Default for GenesisConstants {
    fn default() -> Self {
        Self {
            delta: Some(MAINNET_DELTA),
            k: Some(MAINNET_TRANSITION_FRONTIER_K),
            txpool_max_size: Some(MAINNET_TXPOOL_MAX_SIZE),
            slots_per_epoch: Some(MAINNET_EPOCH_SLOT_COUNT),
            slots_per_sub_window: Some(MAINNET_SLOTS_PER_SUB_WINDOW),
        }
    }
}

impl From<GenesisRoot> for GenesisLedger {
    fn from(value: GenesisRoot) -> Self {
        Self::new(value.ledger)
    }
}

impl From<GenesisLedger> for Ledger {
    fn from(value: GenesisLedger) -> Self {
        let token_ledger: TokenLedger = value.into();

        Ledger::from_mina_ledger(token_ledger)
    }
}

impl From<GenesisLedger> for TokenLedger {
    fn from(value: GenesisLedger) -> Self {
        Self {
            accounts: value
                .ledger
                .accounts
                .into_iter()
                .map(|(pk, acct)| {
                    (
                        pk,
                        Account {
                            // add display fee
                            balance: acct.balance + MAINNET_ACCOUNT_CREATION_FEE,
                            ..acct
                        },
                    )
                })
                .collect(),
        }
    }
}

impl GenesisLedger {
    pub const MAINNET_V1_GENESIS_LEDGER_CONTENTS: &'static str =
        include_str!("../../data/genesis_ledgers/mainnet.json");

    /// This is the only way to construct a genesis ledger
    pub fn new(genesis: GenesisAccounts) -> GenesisLedger {
        let mut accounts = HashMap::new();

        // Add genesis block winner
        let block_creator = Account::from(GenesisBlock::new_v1().unwrap());
        accounts.insert(block_creator.public_key.clone(), block_creator);

        for account in genesis.accounts {
            let balance = account
                .balance
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse Genesis Balance"));

            let public_key = PublicKey::from(account.pk);
            accounts.insert(
                public_key.clone(),
                Account {
                    balance,
                    username: None,
                    nonce: None,
                    public_key: public_key.clone(),
                    // If delegate is None, delegate to yourself
                    delegate: account.delegate.map_or(public_key, PublicKey),
                    token: account.token.map(TokenAddress::from),
                    token_symbol: None,
                    receipt_chain_hash: account.receipt_chain_hash,
                    voting_for: account.voting_for.map(Into::into),
                    permissions: account.permissions,
                    timing: account.timing.map(|t| t.into()),
                    zkapp: account.zkapp,
                    genesis_account: true,
                },
            );
        }

        Self {
            ledger: TokenLedger { accounts },
        }
    }
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<GenesisRoot> {
    let data = std::fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

impl From<GenesisAccountTiming> for Timing {
    fn from(value: GenesisAccountTiming) -> Self {
        Self {
            initial_minimum_balance: value
                .initial_minimum_balance
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis initial minimum balance"))
                .0,
            cliff_time: value.cliff_time.parse().expect("cliff time is u64"),
            cliff_amount: value
                .cliff_amount
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis cliff amount"))
                .0,
            vesting_period: value.vesting_period.parse().expect("vesting period is u64"),
            vesting_increment: value
                .vesting_increment
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis vesting increment"))
                .0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GenesisConstants, GenesisLedger, GenesisRoot};
    use crate::ledger::public_key::PublicKey;
    use std::path::PathBuf;

    #[test]
    fn test_genesis_ledger_default_delegation_test() -> anyhow::Result<()> {
        let ledger_json = r#"{
            "genesis": {
                "genesis_state_timestamp": "2021-03-17T00:00:00Z"
            },
            "ledger": {
                "name": "mainnet",
                "accounts": [
                    {"pk": "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1","balance":"0"}
                ]
            }
        }"#;

        let root: GenesisRoot = serde_json::from_str(ledger_json)?;

        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            root.ledger.accounts.first().unwrap().pk
        );
        assert_eq!(None, root.ledger.accounts.first().unwrap().delegate);

        let ledger = GenesisLedger::new(root.ledger);
        let map = ledger.ledger.accounts;
        let value = map
            .get(&PublicKey::from(
                "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            ))
            .unwrap();

        // The delete should be the same as the public key
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            value.public_key.0
        );
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            value.delegate.0
        );

        Ok(())
    }

    #[test]
    fn override_genesis_constants() -> anyhow::Result<()> {
        let mut none_constants = GenesisConstants::default();
        let none_path: PathBuf = "./tests/data/genesis_constants/none.json".into();
        none_constants.override_with(serde_json::from_slice::<GenesisConstants>(&std::fs::read(
            none_path,
        )?)?);
        assert_eq!(none_constants, GenesisConstants::default());

        let mut some_constants = GenesisConstants::default();
        let some_path: PathBuf = "./tests/data/genesis_constants/some.json".into();
        let some_constants_file =
            serde_json::from_slice::<GenesisConstants>(&std::fs::read(some_path)?)?;

        some_constants.override_with(some_constants_file);
        assert_eq!(
            some_constants,
            GenesisConstants {
                delta: Some(1),
                txpool_max_size: Some(1000),
                ..GenesisConstants::default()
            }
        );

        let mut all_constants = GenesisConstants::default();
        let all_path: PathBuf = "./tests/data/genesis_constants/all.json".into();
        let all_constants_file =
            serde_json::from_slice::<GenesisConstants>(&std::fs::read(all_path)?)?;

        all_constants.override_with(all_constants_file.clone());
        assert_eq!(all_constants, all_constants_file);

        Ok(())
    }
}
