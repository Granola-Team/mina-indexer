pub mod parser;

use crate::{block::BlockHash, ledger::public_key::PublicKey};
use anyhow::bail;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::BufReader, path::Path};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingLedger {
    pub epoch: u32,
    pub network: String,
    pub ledger_hash: LedgerHash,
    pub staking_ledger: HashMap<PublicKey, StakingAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerHash(pub String);

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccount {
    pk: PublicKey,
    balance: u64,
    delegate: PublicKey,
    token: u32,
    token_permissions: TokenPermissions,
    receipt_chain_hash: ReceiptChainHash,
    voting_for: BlockHash,
    permissions: Permissions,
    nonce: Option<u32>,
    timing: Option<Timing>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    stake: bool,
    edit_state: Permission,
    send: Permission,
    set_delegate: Permission,
    set_permissions: Permission,
    set_verification_key: Permission,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "signature")]
    Signature,
    #[serde(rename = "proof")]
    Proof,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub initial_minimum_balance: u64,
    pub cliff_time: u64,
    pub cliff_amount: u64,
    pub vesting_period: u64,
    pub vesting_increment: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPermissions {}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChainHash(String);

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

impl From<StakingAccountJson> for StakingAccount {
    fn from(value: StakingAccountJson) -> Self {
        let token = value.token.parse().expect("token is u32");
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
        }
    }
}

pub fn is_valid_ledger_file(path: &Path) -> bool {
    crate::block::is_valid_file_name(path, &super::is_valid_ledger_hash)
}

pub fn split_ledger_path(path: &Path) -> (String, u32, LedgerHash) {
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
    pub fn parse_file(path: &Path) -> anyhow::Result<StakingLedger> {
        if !is_valid_ledger_file(path) {
            bail!("Invalid ledger file: {}", path.display())
        } else {
            let file = std::fs::File::open(path)?;
            let size = file.metadata()?.len() as usize;
            let reader = BufReader::with_capacity(size, file);
            let staking_ledger: Vec<StakingAccountJson> = serde_json::from_reader(reader)?;
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

#[cfg(test)]
mod tests {
    use super::StakingLedger;
    use std::path::PathBuf;

    #[test]
    fn parse_file() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/staking_ledgers/mainnet-42-jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH.json".into();
        let staking_ledger = StakingLedger::parse_file(&path)?;

        assert_eq!(staking_ledger.epoch, 42);
        assert_eq!(
            staking_ledger.ledger_hash.0,
            "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH".to_string()
        );

        Ok(())
    }
}
