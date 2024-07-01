pub mod parser;

use crate::{
    block::BlockHash,
    chain::Network,
    constants::*,
    ledger::{
        account::{Permissions, ReceiptChainHash, Timing, TokenPermissions},
        public_key::PublicKey,
        LedgerHash,
    },
    mina_blocks::v2::ZkappAccount,
};
use log::trace;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingLedger {
    pub epoch: u32,
    pub network: Network,
    pub ledger_hash: LedgerHash,
    pub total_currency: u64,
    pub genesis_state_hash: BlockHash,
    pub staking_ledger: HashMap<PublicKey, StakingAccount>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccount {
    pub pk: PublicKey,
    pub balance: u64,
    pub delegate: PublicKey,
    pub token: Option<u64>,
    pub token_permissions: TokenPermissions,
    pub receipt_chain_hash: ReceiptChainHash,
    pub voting_for: BlockHash,
    pub permissions: Permissions,
    pub nonce: Option<u32>,
    pub timing: Option<Timing>,
    pub zkapp: Option<ZkappAccount>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccountJson {
    pub pk: PublicKey,
    pub balance: String,
    pub delegate: PublicKey,
    pub token: String,
    pub token_permissions: TokenPermissions,
    pub receipt_chain_hash: ReceiptChainHash,
    pub voting_for: BlockHash,
    pub permissions: Permissions,
    pub nonce: Option<String>,
    pub timing: Option<TimingJson>,
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
    pub genesis_state_hash: BlockHash,
    pub delegations: HashMap<PublicKey, EpochStakeDelegation>,
    pub total_delegations: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochStakeDelegation {
    pub pk: PublicKey,
    pub count_delegates: Option<u32>,
    pub total_delegated: Option<u64>,
    pub delegates: Vec<PublicKey>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatedEpochStakeDelegation {
    pub pk: PublicKey,
    pub epoch: u32,
    pub network: Network,
    pub total_stake: u64,
    pub count_delegates: Option<u32>,
    pub total_delegated: Option<u64>,
    pub delegates: Vec<PublicKey>,
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

impl StakingAccount {
    pub fn chain_id() -> String {
        crate::chain::chain_id(
            MAINNET_GENESIS_HASH,
            MAINNET_PROTOCOL_CONSTANTS,
            MAINNET_CONSTRAINT_SYSTEM_DIGESTS,
        )
        .0[..6]
            .to_string()
    }
}

impl StakingLedger {
    pub async fn parse_file(
        path: &Path,
        genesis_state_hash: BlockHash,
    ) -> anyhow::Result<StakingLedger> {
        trace!("Parsing staking ledger");

        let mut file = File::open(path).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;

        let staking_ledger: Vec<StakingAccountJson> = serde_json::from_slice(&bytes)?;
        let staking_ledger: HashMap<PublicKey, StakingAccount> = staking_ledger
            .into_iter()
            .map(|acct| (acct.pk.clone(), acct.into()))
            .collect();

        let (network, epoch, ledger_hash) = split_ledger_path(path);
        let total_currency: u64 = staking_ledger.values().map(|account| account.balance).sum();
        Ok(Self {
            epoch,
            network,
            total_currency,
            ledger_hash,
            staking_ledger,
            genesis_state_hash,
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
                        delegates: vec![pk.clone()],
                    }),
                ) {
                    None => (), // first delegation
                    Some(None) => {
                        // delegated to another account
                        delegations.insert(delegate.clone(), None);
                    }
                    Some(Some(EpochStakeDelegation {
                        pk: delegate,
                        total_delegated,
                        count_delegates,
                        mut delegates,
                    })) => {
                        // accumulate delegation
                        delegates.push(pk.clone());
                        delegations.insert(
                            delegate.clone(),
                            Some(EpochStakeDelegation {
                                pk: delegate,
                                total_delegated: total_delegated.map(|acc| acc + balance),
                                count_delegates: count_delegates.map(|acc| acc + 1),
                                delegates,
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
                    ..Default::default()
                });
            }
        });

        let delegations: HashMap<PublicKey, EpochStakeDelegation> = delegations
            .into_iter()
            .map(|(pk, del)| (pk, del.unwrap_or_default()))
            .collect();
        Ok(AggregatedEpochStakeDelegations {
            delegations,
            total_delegations,
            epoch: self.epoch,
            network: self.network.clone(),
            ledger_hash: self.ledger_hash.clone(),
            genesis_state_hash: self.genesis_state_hash.clone(),
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
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::StakingLedger;
    use crate::{
        chain::Network, constants::MAINNET_GENESIS_HASH,
        ledger::staking::AggregatedEpochStakeDelegations,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn parse_file() -> anyhow::Result<()> {
        let path: PathBuf = "../tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path, MAINNET_GENESIS_HASH.into()).await?;

        assert_eq!(staking_ledger.epoch, 0);
        assert_eq!(staking_ledger.network, Network::Mainnet);
        assert_eq!(
            staking_ledger.ledger_hash.0,
            "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn calculate_delegations() -> anyhow::Result<()> {
        use crate::ledger::public_key::PublicKey;

        let path: PathBuf = "../tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path, MAINNET_GENESIS_HASH.into()).await?;
        let AggregatedEpochStakeDelegations {
            epoch,
            network,
            ledger_hash,
            genesis_state_hash,
            delegations,
            total_delegations,
        } = staking_ledger.aggregate_delegations()?;

        assert_eq!(epoch, 0);
        assert_eq!(network, Network::Mainnet);
        assert_eq!(
            ledger_hash.0,
            "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".to_string()
        );

        let pk = PublicKey::from("B62qrecVjpoZ4Re3a5arN6gXZ6orhmj1enUtA887XdG5mtZfdUbBUh4");
        let mut pk_delegations = delegations.get(&pk).cloned().unwrap();
        pk_delegations.delegates.sort();

        assert_eq!(pk_delegations.pk, pk);
        assert_eq!(pk_delegations.count_delegates, Some(25));
        assert_eq!(pk_delegations.total_delegated, Some(13277838425206999));
        assert_eq!(total_delegations, 794268782956784283);
        assert_eq!(genesis_state_hash.0, MAINNET_GENESIS_HASH.to_string());

        let mut expected_delegates: Vec<PublicKey> = [
            "B62qmCwouxG2UzH6zEYGFWFFzUuSv9sbLnr96VJWDX3paSSucX7jAJN",
            "B62qpz34iGX2eaRDyHmHbq3v1SnUgzounhudGZRfNUDh79JuTstPNy1",
            "B62qjUut7tByYkosrfLDC5aKLSLQ2JxTbkBcfF3em3HwiyNkEsQmwfM",
            "B62qmqMrgPshhHKLJ7DqWn1KeizEgga5MuGmWb2bXajUnyivfeMW6JE",
            "B62qq4qohsvmTAJmvJ5wSepyNHKsh1wMPf1UjoHLKEuLmgH2RdAa4zt",
            "B62qrecVjpoZ4Re3a5arN6gXZ6orhmj1enUtA887XdG5mtZfdUbBUh4",
            "B62qiigQwvLyyqUsAL3SmtjP43iGnUB1s1mUYrhvyZCc8PVFaW9ZQvd",
            "B62qqhTWoZineudzfT9o4YTdruHTps1yANJz9z1Zw3YnFAMgCky8LFS",
            "B62qrQW1u4635tmjLjkz7pdUrwE9QhmYP8rPb13SpaNBeHa4pGidstk",
            "B62qnbZcyj5U8N4nqGyt8gf67qsGitf3LFfjRsNZuXV6c3XA84V7p1v",
            "B62qppJosj13spPS9ZvkhqUfqkTRH9LHYHcUZR3Wivayjrs1tZcZxXq",
            "B62qkK29ScnXfTzrDkkfASepKoTE57CT8SA4r43EQCCwwJXAsP5TGGN",
            "B62qqKV2KTVR8Sic9Yq9P7Z1sb819smRBaCqWi7UuzHgiagLrSRmi6P",
            "B62qkpKnTJ1uAR6ZQ7Z7DW9UwjDuzSZJkTPDDKUGHgBdi5bQAUJ1gG2",
            "B62qizxV2Z1Lbf8TFb4Jzf3uJTd8CDBSuJ5ypkJdw9pZNKzW1Rzxrwh",
            "B62qnipPgHt7ajPdMko2STLDAxWW1M5q6sZ8V578khR2KMQUbhxtTPN",
            "B62qmtcbMEVVKN2guoyVEqmiPZtbvNhz4VUvUYvxesya3WHPkLiBvdK",
            "B62qmru4aEDszwLFvH59BtZnU4QLC52nrSRBJW1EfdAp8cD5drg8QFM",
            "B62qm3hoUHCPWGdKfrSK5Ek9STvGfwjf6L1uewvgFQHCVY4Y48DT4Qr",
            "B62qoA478cjzLTGH3JqDrNXGjQNGQJeKesjnS6o9aVv875epCMtrrsD",
            "B62qrYufgatwTD8UkM1tLnW5tfnSeWYSPtbBYyYvdQ4dvoA9KBeWfcH",
            "B62qmEo9HSSLLM3DtUJwcNdeqqQ6zoNUMqXauu9ySWKdpo8W7T9bjR3",
            "B62qknoGUuTS2MmtZMrJLX6SumUP5BjKJVhyPTKjSBH5xyenxZ8dTWV",
            "B62qrPH1SqnVrh92QART2N8sjmjRqnidtp4my5SAxstpurqQheNAR9u",
            "B62qpk2BRXnuxofXRU1z2y1LWRagabiSLoBuCJjDSv9ebVkzZE2zXnp",
        ]
        .into_iter()
        .map(PublicKey::from)
        .collect();
        expected_delegates.sort();
        assert_eq!(pk_delegations.delegates, expected_delegates);

        Ok(())
    }
}
