pub mod parser;

use crate::{
    block::{precomputed::PcbVersion, BlockHash},
    chain::Network,
    ledger::{
        account::{Permissions, ReceiptChainHash, Timing, TokenPermissions},
        public_key::PublicKey,
        LedgerHash,
    },
};
use log::trace;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingLedger {
    pub epoch: u32,
    pub network: Network,
    pub ledger_hash: LedgerHash,
    pub staking_ledger: HashMap<PublicKey, StakingAccount>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccount {
    pub pk: PublicKey,
    pub balance: u64,
    pub delegate: PublicKey,
    pub token: Option<u32>,
    pub token_permissions: TokenPermissions,
    pub receipt_chain_hash: ReceiptChainHash,
    pub voting_for: BlockHash,
    pub permissions: Permissions,
    pub nonce: Option<u32>,
    pub timing: Option<Timing>,
    pub zkapp: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccountJson {
    pk: PublicKey,
    balance: String,
    delegate: PublicKey,
    token: String,
    token_permissions: TokenPermissions,
    receipt_chain_hash: ReceiptChainHash,
    voting_for: BlockHash,
    permissions: Permissions,
    nonce: Option<String>,
    timing: Option<TimingJson>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingJson {
    pub initial_minimum_balance: String,
    pub cliff_time: String,
    pub cliff_amount: String,
    pub vesting_period: String,
    pub vesting_increment: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatedEpochStakeDelegations {
    pub epoch: u32,
    pub network: Network,
    pub ledger_hash: LedgerHash,
    pub delegations: HashMap<PublicKey, EpochStakeDelegation>,
    pub total_delegations: u64,
}

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochStakeDelegation {
    pub pk: PublicKey,
    pub count_delegates: Option<u32>,
    pub total_delegated: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatedEpochStakeDelegation {
    pub pk: PublicKey,
    pub epoch: u32,
    pub network: Network,
    pub total_stake: u64,
    pub count_delegates: Option<u32>,
    pub total_delegated: Option<u64>,
}

impl From<StakingAccountJson> for StakingAccount {
    fn from(value: StakingAccountJson) -> Self {
        let token = Some(value.token.parse().expect("token is u32"));
        let nonce = value
            .nonce
            .map(|nonce| nonce.parse().expect("nonce is u32"));
        let balance = match value.balance.parse::<Decimal>() {
            Ok(amt) => (amt * dec!(1_000_000_000))
                .to_u64()
                .expect("staking account balance"),
            Err(e) => panic!("Unable to parse staking account balance: {e}"),
        };
        let timing = value.timing.map(|timing| Timing {
            cliff_time: timing.cliff_time.parse().expect("cliff_time is u64"),
            vesting_period: timing
                .vesting_period
                .parse()
                .expect("vesting_period is u64"),
            initial_minimum_balance: match timing.initial_minimum_balance.parse::<Decimal>() {
                Ok(amt) => (amt * dec!(1_000_000_000)).to_u64().unwrap(),
                Err(e) => panic!("Unable to parse initial_minimum_balance: {e}"),
            },
            cliff_amount: match timing.cliff_amount.parse::<Decimal>() {
                Ok(amt) => (amt * dec!(1_000_000_000)).to_u64().unwrap(),
                Err(e) => panic!("Unable to parse cliff_amount: {e}"),
            },
            vesting_increment: match timing.vesting_increment.parse::<Decimal>() {
                Ok(amt) => (amt * dec!(1_000_000_000)).to_u64().unwrap(),
                Err(e) => panic!("Unable to parse vesting_increment: {e}"),
            },
        });
        Self {
            nonce,
            token,
            timing,
            balance,
            pk: value.pk,
            delegate: value.delegate,
            voting_for: value.voting_for,
            permissions: value.permissions,
            token_permissions: value.token_permissions,
            receipt_chain_hash: value.receipt_chain_hash,
            zkapp: None,
        }
    }
}

pub fn is_valid_ledger_file(path: &Path) -> bool {
    crate::block::is_valid_file_name(path, &super::is_valid_ledger_hash)
}

pub fn split_ledger_path(path: &Path) -> (Network, u32, LedgerHash) {
    let parts: Vec<&str> = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .split('-')
        .collect();
    (
        parts[0].into(),
        parts[1].parse().unwrap(),
        LedgerHash(parts[2].into()),
    )
}

impl StakingLedger {
    pub fn parse_file(path: &Path, version: PcbVersion) -> anyhow::Result<StakingLedger> {
        trace!("Parsing {} staking ledger", version);

        let bytes = std::fs::read(path)?;
        let staking_ledger: Vec<StakingAccountJson> = serde_json::from_slice(&bytes)?;
        let staking_ledger = staking_ledger
            .into_iter()
            .map(|acct| (acct.pk.clone(), acct.into()))
            .collect();
        let (network, epoch, ledger_hash) = split_ledger_path(path);

        Ok(Self {
            epoch,
            network,
            ledger_hash,
            staking_ledger,
        })
    }

    /// Aggregate each public key's staking delegations and total delegations
    /// If the public key has delegated, they cannot be delegated to
    pub fn aggregate_delegations(&self) -> anyhow::Result<AggregatedEpochStakeDelegations> {
        let mut delegations = HashMap::new();
        self.staking_ledger
            .iter()
            .for_each(|(pk, staking_account)| {
                let balance = staking_account.balance;
                let delegate = staking_account.delegate.clone();

                if *pk != delegate {
                    delegations.insert(pk.clone(), None);
                }

                match delegations.insert(
                    delegate.clone(),
                    Some(EpochStakeDelegation {
                        pk: delegate.clone(),
                        total_delegated: Some(balance),
                        count_delegates: Some(1),
                    }),
                ) {
                    None => (), // first delegation
                    Some(None) => {
                        // pk delegated to another pk
                        delegations.insert(delegate.clone(), None);
                    }
                    Some(Some(EpochStakeDelegation {
                        pk,
                        total_delegated,
                        count_delegates,
                    })) => {
                        // accumulate delegation
                        delegations.insert(
                            delegate,
                            Some(EpochStakeDelegation {
                                pk,
                                total_delegated: total_delegated.map(|acc| acc + balance),
                                count_delegates: count_delegates.map(|acc| acc + 1),
                            }),
                        );
                    }
                }
            });

        let total_delegations = delegations.values().fold(0, |acc, x| {
            acc + x
                .as_ref()
                .map(|x| x.total_delegated.unwrap_or_default())
                .unwrap_or_default()
        });
        delegations.iter_mut().for_each(|(pk, delegation)| {
            if delegation.is_none() {
                *delegation = Some(EpochStakeDelegation {
                    pk: pk.clone(),
                    count_delegates: None,
                    total_delegated: None,
                });
            }
        });
        let delegations = delegations
            .into_iter()
            .map(|(pk, del)| (pk, del.unwrap_or_default()))
            .collect();
        Ok(AggregatedEpochStakeDelegations {
            delegations,
            total_delegations,
            epoch: self.epoch,
            network: self.network.clone(),
            ledger_hash: self.ledger_hash.clone(),
        })
    }

    pub fn summary(&self) -> String {
        format!("(epoch {}): {}", self.epoch, self.ledger_hash.0)
    }
}

impl From<String> for LedgerHash {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::str::FromStr for ReceiptChainHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO validate receipt chain hash?
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{EpochStakeDelegation, StakingLedger};
    use crate::{
        block::precomputed::PcbVersion, chain::Network,
        ledger::staking::AggregatedEpochStakeDelegations,
    };
    use std::path::PathBuf;

    #[test]
    fn parse_file() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path, PcbVersion::V1)?;

        assert_eq!(staking_ledger.epoch, 0);
        assert_eq!(staking_ledger.network, Network::Mainnet);
        assert_eq!(
            staking_ledger.ledger_hash.0,
            "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".to_string()
        );
        Ok(())
    }

    #[test]
    fn calculate_delegations() -> anyhow::Result<()> {
        use crate::ledger::public_key::PublicKey;

        let path: PathBuf = "./tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path, PcbVersion::V1)?;
        let AggregatedEpochStakeDelegations {
            epoch,
            network,
            ledger_hash,
            delegations,
            total_delegations,
        } = staking_ledger.aggregate_delegations()?;
        let pk: PublicKey = "B62qrecVjpoZ4Re3a5arN6gXZ6orhmj1enUtA887XdG5mtZfdUbBUh4".into();

        assert_eq!(epoch, 0);
        assert_eq!(network, Network::Mainnet);
        assert_eq!(
            ledger_hash.0,
            "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".to_string()
        );
        assert_eq!(
            delegations.get(&pk),
            Some(&EpochStakeDelegation {
                pk,
                count_delegates: Some(25),
                total_delegated: Some(13277838425206999)
            })
        );
        assert_eq!(total_delegations, 794268782956784283);
        Ok(())
    }
}
