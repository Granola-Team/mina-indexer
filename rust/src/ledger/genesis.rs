use super::{
    account::{Account, ReceiptChainHash, Timing, VotingFor},
    token::{TokenAddress, TokenId},
    Ledger, TokenLedger,
};
use crate::{
    base::{amount::Amount, nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    block::genesis::GenesisBlock,
    constants::*,
    utility::compression::decompress_gzip,
};
use anyhow::anyhow;
use log::error;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisLedger {
    ledger: TokenLedger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisRoot {
    pub genesis: Option<GenesisTimestamp>,
    pub proof: Option<GenesisProof>,
    pub ledger: GenesisAccounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisTimestamp {
    pub genesis_state_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisProof {
    pub fork: GenesisForkProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisForkProof {
    pub state_hash: StateHash,
    pub blockchain_length: u32,
    pub global_slot_since_genesis: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccounts {
    pub name: Option<String>,
    pub accounts: Vec<GenesisAccount>,
    pub seed: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub pk: String,
    pub balance: String,
    pub delegate: Option<String>,
    pub token_permissions: Option<TokenPermissions>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<VotingFor>,
    pub permissions: Option<Permissions>,
    pub timing: Option<GenesisAccountTiming>,

    #[serde(default)]
    pub nonce: Option<Nonce>,

    #[serde(default)]
    pub token: Option<TokenId>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TokenPermissions {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub stake: bool,
    pub edit_state: Permission,
    pub send: Permission,
    pub set_delegate: Permission,
    pub set_permissions: Permission,
    pub set_verification_key: Permission,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    #[default]
    Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccountTiming {
    pub initial_minimum_balance: String,
    pub cliff_time: String,
    pub cliff_amount: String,
    pub vesting_period: String,
    pub vesting_increment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolConstants {
    pub k: Option<u32>,
    pub slots_per_epoch: Option<u32>,
    pub slots_per_sub_window: Option<u32>,
    pub delta: Option<u32>,
    pub txpool_max_size: Option<u32>,
}

///////////
// impls //
///////////

impl ProtocolConstants {
    pub fn from_path<P>(path: Option<P>) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut constants = Self::default();
        if let Some(path) = path {
            if let Ok(ref contents) = std::fs::read(path) {
                if let Ok(override_constants) = serde_json::from_slice(contents) {
                    constants.override_with(override_constants);
                } else {
                    error!(
                        "Error parsing supplied protocol constants. Using default:\n{}",
                        serde_json::to_string_pretty(&constants)?
                    )
                }
            } else {
                error!(
                    "Error reading protocol constants file. Using default:\n{}",
                    serde_json::to_string_pretty(&constants)?
                )
            }
        }
        Ok(constants)
    }

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

impl GenesisLedger {
    /// Original mainnet genesis ledger
    pub fn new_v1() -> anyhow::Result<Self> {
        Self::from_str(include_str!("../../data/genesis_ledgers/mainnet.json"))
    }

    /// Hardfork genesis ledger
    pub fn new_v2() -> anyhow::Result<Self> {
        let bytes = include_bytes!("../../data/genesis_ledgers/hardfork.json.gz");
        if let Ok(root) = decompress_gzip(bytes) {
            if let Ok(root) = serde_json::from_slice::<GenesisRoot>(&root) {
                Ok(root.into())
            } else {
                Err(anyhow::anyhow!("Failed to deserialize genesis ledger"))
            }
        } else {
            Err(anyhow::anyhow!("Failed to decompress genesis ledger"))
        }
    }

    /// This is the only way to construct a genesis ledger
    pub fn new(genesis: GenesisAccounts) -> GenesisLedger {
        // Add genesis block winner
        let block_creator = Account::from_genesis(GenesisBlock::new_v1().unwrap());
        let mut accounts = HashMap::from([(block_creator.public_key.clone(), block_creator)]);

        // add genesis ledger accounts
        for account in genesis.accounts {
            let balance = account
                .balance
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse Genesis Balance"));

            let public_key = PublicKey::from(account.pk);
            let delegate = account
                .delegate
                .map_or_else(|| public_key.to_owned(), PublicKey);

            accounts.insert(
                public_key.clone(),
                Account {
                    public_key,
                    balance,
                    nonce: account.nonce,
                    delegate: delegate.into(),
                    token: account.token.map(TokenAddress::from),
                    receipt_chain_hash: account.receipt_chain_hash,
                    voting_for: account.voting_for,
                    timing: account.timing.map(Into::into),
                    genesis_account: Some(balance),
                    ..Default::default()
                },
            );
        }

        Self {
            ledger: TokenLedger { accounts },
        }
    }

    pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        GenesisRoot::parse_file(path).map(Into::into)
    }
}

impl GenesisRoot {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path.as_ref())?;

        // decompress if gzip'd
        if path.as_ref().extension().is_some_and(|ext| ext == "gz") {
            let bytes = decompress_gzip(&bytes[..])?;
            return Ok(serde_json::from_slice(&bytes)?);
        }

        Ok(serde_json::from_slice(&bytes)?)
    }
}

//////////////
// defaults //
//////////////

impl std::default::Default for ProtocolConstants {
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

/////////////////
// conversions //
/////////////////

impl FromStr for GenesisRoot {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(|e| anyhow!("Error parsing genesis root: {e}"))
    }
}

impl FromStr for GenesisLedger {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GenesisRoot::from_str(s).map(Into::into)
    }
}

impl From<GenesisRoot> for GenesisLedger {
    fn from(value: GenesisRoot) -> Self {
        Self::new(value.ledger)
    }
}

impl From<GenesisLedger> for Ledger {
    fn from(value: GenesisLedger) -> Self {
        Ledger::from_mina_ledger(value.into())
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
                            balance: if acct
                                .token
                                .as_ref()
                                .is_some_and(|t| t.0 != MINA_TOKEN_ADDRESS)
                            {
                                acct.balance
                            } else {
                                acct.balance + MAINNET_ACCOUNT_CREATION_FEE
                            },
                            ..acct
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<GenesisAccountTiming> for Timing {
    fn from(value: GenesisAccountTiming) -> Self {
        Self {
            initial_minimum_balance: value
                .initial_minimum_balance
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis initial minimum balance"))
                .0
                .into(),
            cliff_time: value.cliff_time.parse().expect("cliff time is u64"),
            cliff_amount: value
                .cliff_amount
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis cliff amount"))
                .0
                .into(),
            vesting_period: value.vesting_period.parse().expect("vesting period is u64"),
            vesting_increment: value
                .vesting_increment
                .parse::<Amount>()
                .unwrap_or_else(|_| panic!("Unable to parse genesis vesting increment"))
                .0
                .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_v1() -> anyhow::Result<()> {
        let v1 = GenesisLedger::new_v1()?;
        let v1: Ledger = v1.into();

        assert_eq!(v1.len(), 1676);
        Ok(())
    }

    #[test]
    fn parse_v2() -> anyhow::Result<()> {
        let v2 = GenesisLedger::new_v2()?;
        let v2: Ledger = v2.into();

        assert_eq!(v2.len(), 228174);
        Ok(())
    }

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

        // before turning into a [Ledger]
        let root: GenesisRoot = serde_json::from_str(ledger_json)?;
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            root.ledger.accounts.first().unwrap().pk
        );
        assert_eq!(None, root.ledger.accounts.first().unwrap().delegate);

        // after turning into a [Ledger]
        let ledger = GenesisLedger::new(root.ledger);
        let account = ledger
            .ledger
            .accounts
            .get(&"B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1".into())
            .unwrap();

        // The delete should be the same as the public key
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            account.public_key.0
        );
        assert_eq!(
            "B62qqdcf6K9HyBSaxqH5JVFJkc1SUEe1VzDc5kYZFQZXWSQyGHoino1",
            account.delegate.0 .0
        );

        Ok(())
    }

    #[test]
    fn override_genesis_constants() -> anyhow::Result<()> {
        // no override
        let mut none_constants = ProtocolConstants::default();
        let none_path: PathBuf = "./tests/data/genesis_constants/none.json".into();

        none_constants.override_with(serde_json::from_slice::<ProtocolConstants>(
            &std::fs::read(none_path)?,
        )?);
        assert_eq!(none_constants, ProtocolConstants::default());

        // override some
        let mut some_constants = ProtocolConstants::default();
        let some_path: PathBuf = "./tests/data/genesis_constants/some.json".into();
        let some_constants_file =
            serde_json::from_slice::<ProtocolConstants>(&std::fs::read(some_path)?)?;

        some_constants.override_with(some_constants_file);
        assert_eq!(
            some_constants,
            ProtocolConstants {
                delta: Some(1),
                txpool_max_size: Some(1000),
                ..ProtocolConstants::default()
            }
        );

        // override all
        let mut all_constants = ProtocolConstants::default();
        let all_path: PathBuf = "./tests/data/genesis_constants/all.json".into();
        let all_constants_file =
            serde_json::from_slice::<ProtocolConstants>(&std::fs::read(all_path)?)?;

        all_constants.override_with(all_constants_file.clone());
        assert_eq!(all_constants, all_constants_file);

        Ok(())
    }
}
