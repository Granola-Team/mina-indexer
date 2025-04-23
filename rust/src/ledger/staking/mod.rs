pub mod parser;
pub mod permissions;

use super::token::TokenAddress;
use crate::{
    base::{nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    block::extract_network,
    chain::Network,
    constants::{HARDFORK_GENESIS_HASH, MAINNET_GENESIS_HASH, MINA_SCALE_DEC},
    ledger::{
        account::{ReceiptChainHash, Timing},
        LedgerHash,
    },
    mina_blocks::v2::ZkappAccount,
    utility::{compression::decompress_gzip, functions::extract_height_and_hash},
};
use anyhow::Context;
use log::trace;
use permissions::StakingPermissions;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingLedger {
    pub epoch: u32,
    pub network: Network,
    pub ledger_hash: LedgerHash,
    pub total_currency: u64,
    pub genesis_state_hash: StateHash,
    pub staking_ledger: HashMap<PublicKey, StakingAccount>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccount {
    pub pk: PublicKey,
    pub balance: u64,
    pub delegate: PublicKey,
    pub username: Option<String>,
    pub token: Option<TokenAddress>,
    pub permissions: StakingPermissions,
    pub receipt_chain_hash: ReceiptChainHash,
    pub voting_for: StateHash,
    pub nonce: Option<Nonce>,
    pub timing: Option<Timing>,
    pub zkapp: Option<ZkappAccount>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccountJson {
    pub pk: PublicKey,
    pub balance: String,
    pub delegate: Option<PublicKey>,
    pub username: Option<String>,
    pub token: String,
    pub permissions: StakingPermissions,
    pub receipt_chain_hash: ReceiptChainHash,
    pub voting_for: StateHash,
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
    pub genesis_state_hash: StateHash,
    pub delegations: HashMap<PublicKey, EpochStakeDelegation>,
    pub total_delegations: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochStakeDelegation {
    pub pk: PublicKey,
    pub count_delegates: Option<u32>,
    pub total_delegated: Option<u64>,
    pub delegates: HashSet<PublicKey>,
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
        let token = if let Ok(token_id) = value.token.parse::<u64>() {
            assert_eq!(token_id, 1);

            Some(TokenAddress::default())
        } else if let Ok(token) = value
            .token
            .parse::<TokenAddress>()
            .with_context(|| value.token)
        {
            Some(token)
        } else {
            panic!("Invalid staking account token");
        };

        let nonce = value.nonce.map(Into::into);
        let balance = match value.balance.parse::<Decimal>() {
            Ok(amt) => (amt * MINA_SCALE_DEC)
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
                Ok(amt) => (amt * MINA_SCALE_DEC).to_u64().unwrap().into(),
                Err(e) => panic!("Unable to parse initial_minimum_balance: {e}"),
            },
            cliff_amount: match timing.cliff_amount.parse::<Decimal>() {
                Ok(amt) => (amt * MINA_SCALE_DEC).to_u64().unwrap().into(),
                Err(e) => panic!("Unable to parse cliff_amount: {e}"),
            },
            vesting_increment: match timing.vesting_increment.parse::<Decimal>() {
                Ok(amt) => (amt * MINA_SCALE_DEC).to_u64().unwrap().into(),
                Err(e) => panic!("Unable to parse vesting_increment: {e}"),
            },
        });

        Self {
            nonce,
            token,
            timing,
            balance,
            delegate: value.delegate.unwrap_or_else(|| value.pk.to_owned()),
            pk: value.pk,
            username: value.username,
            voting_for: value.voting_for,
            permissions: value.permissions,
            receipt_chain_hash: value.receipt_chain_hash,
            zkapp: None,
        }
    }
}

impl StakingLedger {
    const V1_STAKING_LEDGER_HASHES: [&str; 79] = [
        "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
        "jwAAZcXndLYxb8w4LTU2d4K1qT3dL8Ck2jKzVEf9t9GAyXweQRG",
        "jxRySSfk8kJZVj46zveaToDUJUC2GtprmeK7poqWymEzB6d2Tun",
        "jwPwVsSPZ2tmmGbp8UrWGmDgFDrrzTPpYcjpWosckmcVZV2kcW7",
        "jxVF5YbC3B5Rk6ibfsL97WaqojfxrgWtEqMJST9pb4X8s3kRD2T",
        "jwJXdYzAikMHnTsw2kYyS1WQxJrGQsy1FKT5c18eHP2wGANafKf",
        "jxQgtuyHp8nA2P6F9CSRrLcVeHi8Ap7wVHeNnH2UbSX15izcSHK",
        "jxct9rteQ7wjhQVf7h4mGQmGZprJMkjbzEWgU7VvV6HEq2DN5yA",
        "jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk",
        "jxhjiLBeMR7pgtV8ogcJvqXdr6asoNrC3g6hoUzEDLBSnZoxUDJ",
        "jx2XUFjvsvtTKB4HPAzih5boAtuoR34kxjEoU1RUhfXTATyx8tw",
        "jx4itrnmDkG3ptAiwhitJHt9K8stgFFoenrkZrm2prbtaS54xQU",
        "jwq7sAxDuN9MrdLjAQULoyrY5hWa6g52SVq8EmajBeBY38zamgz",
        "jxPj7F7aRew1zvpW9JaGSgt9xmJitenrRSM6YGKnuhe5HXqyZtZ",
        "jxn15ATGoe4WGgYpbssxJH9XW8NXRDy22WvSsBqvMqcnLPgPAwN",
        "jwAXd4GZgxE3YCwqs99g4MpLNiEV2ZfZPstyah4jxo753AVgL6R",
        "jwe63YTTUcc2b4sFdP54ehCZ3Dp9sZKshwCmtoVP3bidzfPfcxw",
        "jx5PU6GmyUqNuCnHNRF3pjHp7CTXXiCog4zJ1WcwHdyF3EJJ1Px",
        "jxos2foijoacWtcKdjzqwv2PrU7je8XFDnsSVNGgrgJaJLeA8VE",
        "jxBBSjakhQRKLbUM7z99KXNnMke2GbdcJyqpD9gyRoJJybsMRqh",
        "jxix1ap5gwXmiiwRqjijDv5KbHmnjAfj19CDywRLT1J8yTADcsT",
        "jwV7BsK9rBf5uRWqMZmWKVAUcEcd7pDAo9NCFTrvSvXRjHCwypF",
        "jwb5g4nyyMFvrXqN9wZLLb2TUx3Ux4gJ5F1k8Rt5nT9Eyaw9mZK",
        "jwHGGFvSf4BVMuQs65gXb384cGzdkbQDyr9rVUPnGDXa1kKJNne",
        "jx3Z9VyiCTMdif3cHZQVs1zfLKmkE8Z6N2CzTfDFi3gM6XyJaRa",
        "jxAqNQwwU31ez8JPg6aJxugdX4uYnKFwbWGjqRtAxkfBBsLf2gf",
        "jxsdc9d3AkKmVSWZQExucepfuLwfzQHtZpiCFArGqtfVe5jveiZ",
        "jx29wpTRDF8tuMFXgqT8inkJhb5chPjtZiwgTHzs6GxsvAy5KiH",
        "jxSi26fHMFyv8kxj4nBDDwi5FBt4oJummDjnfPodDbsNBzyjQdU",
        "jxDP6iJZGfNixGBiVasAnYYm1Fk29qWP2MecJ4mAg676DK7sQCM",
        "jwcWudRBTNZuMd1Tcyjzpr71buwc9RNmT2Jip1efA9eWvXcZiKL",
        "jwVvWi6GLeL6mz9jVFtD1HA7GNVuqe8tjFedisASfk8WVmdcfKE",
        "jw9ZJUdzn6NYSinWYuSEV27qKE2ZFXyvpzgxD5ZzsbyWYpeqnR8",
        "jxHoMZnbhR25patdD3SeNQPe3U9MPctcRSRvPw7p7rpTKcZLB6t",
        "jx1t9ivUkPJq9QfewYxFEc9GGLQVRZupDa9LRYFQeqpr9JPb1jj",
        "jwJLfz7Pqfr3eRDFAMDw5TJ4Q3aD7ZZpP8YWKdWWU2iHU137NUE",
        "jwpXcZgEcdvSswWPkaKYBcq1jfydzqitb87psbroyW6FSmjiSL8",
        "jwHyH1qgW4iBRHEJEDo4yaxMW82VgNCLmQwHNzVKSxTapisydbo",
        "jw9FBsiQK5uJVGd8nr333vvctg3hPKf5kZUHf7f5bnUojWyNt3Z",
        "jxxaCK9mMbnpCR3D5TS55Tit8y36E9jN8ER1P6Xry8TyHPYp1CY",
        "jwPQHxrJ94osTLCAiHYBuA6L4KGjkDV9t1A4mhdUoVEmbt2gxha",
        "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH",
        "jxRhDLj6Q62jjRDNS2yYtDu6yHziPx6yLNXvPdgMfZaF3NFvJic",
        "jxdhN2AXg5v3c6KbGdmNW58hvdiTVULhXF3yztD8CdKNnGdf3jp",
        "jxWMPncjMY9VhwehhVKHhobvJuAhZcdx5kfUtX4V3dy9Rw9aMZA",
        "jxQXzUkst2L9Ma9g9YQ3kfpgB5v5Znr1vrYb1mupakc5y7T89H8",
        "jwfyNt9AX6zRoWf67EcAzSQSDdLsS7Y8gZQPKmceCKo9C4hyKyX",
        "jxZGkwwaAEXdKaFB12jdxfedApFQ4gDJ58aiSjNw9VUffBgAmdg",
        "jwe5YREbjxzPCKe3eK7KfW5yXMdh71ca9mnMFfx9dBBQnRB6Rkb",
        "jxaswvEn5WF82AHLwbzMJN5Ty6RNAH9azqMV2R9q4sJStCpMp3w",
        "jwuGkeeB2rxs2Cr679nZMVZpWms6QoEkcgt82Z2jsjB9X1MuJwW",
        "jxWkqFVYsmQrXQZ2kkujynVj3TfbLfhVSgrY73CSVDpc17Bp3L6",
        "jxyqGrB5cSEavMbcMyNXhFMLcWpvbLR9a73GLqbTyPKVREkDjDM",
        "jx6taGcqX3HpWcz558wWNnJcne99jiQQiR7AnE7Ny8cQB1ASDVK",
        "jw8dXuUqXVgd6NvmpryGmFLnRv1176oozHAro8gMFwj8yuvhBeS",
        "jxXZTgUtCJmJnuwURmNMhoJWQ44X1kRLaKXtuYRFxnT9GFGSnnj",
        "jwgDB316LgQr15vmZYC5G9gjcizT5MPssZbQkvtBLtqpi93mbMw",
        "jwUe5igYAtQWZpcVYxt6xPJywnCZqDiNng9xZQLSZfZKpLZ61Hp",
        "jxffUAqcai9KoheQDcG46CCczjMRzFk61oXSokjaKvphicMpPj5",
        "jxKpSD4zcfKCSQQd3CG3yBqiesbUqm7eucRqLSvi9T1gUXtUPR5",
        "jxwahv5MsbGaUwSdAhyQA7Gr7atsyQbcju289PkoAnS4UgHGdce",
        "jy1jMBD7atiiheMxufJQDDfBuC2BjSXGj2HC5uSjXXsjAsGZt71",
        "jwbeXmeEZ2aYSyXnBbsMqWUxVDouwYZdzCqBejTaoecCkHoinPy",
        "jx4MPGB51t9MjrUh7NSsU6dLaouAb9bE2xu8b79kzmkEtKezwfw",
        "jxAzD4eVVmY4bFF9QnMrEmjG8rEXEgVCFbD4H85LVZu4c4Zmi9D",
        "jwvsYHPfACRUFYLL5NknBJc7zEY1q8t9rQfF8ek2pk2dUuKCz5J",
        "jxKCrryFrvzBE4iUURcS9zNTKcRdejiE9K28Bqcu7Us7RQqNfdL",
        "jxJbw37Kd7KxNvy5yd322NFwYZUdsXCeeEfjqGJ3cY9ukMmxBiW",
        "jxQwGGbtjRnhT1k7CqyASPKihyjFdtYSnJMANxdyWbHvGUofn8t",
        "jxw6YYsPFbC7bPqCcc6pVShATXbebaX1cxFqeV7Kyo1Pa5L3TU4",
        "jxiXyAr4NX6Ne1jxMU4WsiYc6SeBajSQZgmro9b63yDfQEeunD3",
        "jx4YTukDZVaFoiwYpKzzPmoCNzZgyXG1nHQkN7mwoJoB8aXMAmt",
        "jwyody4XQNTnGxkXQEKf87AN27wXadAjYgnGLAtvHahDkn2uWDU",
        "jxvumaCvujr7UzW1qCB87YR2RWu8CqvkwrCmHY8kkwpvN4WbTJn",
        "jx25quMPEvvipny2VxwDys5yCHaUL8oCMapfLv4eoRrsxEKm4pD",
        "jwqkCmcBJDi7XVRuW3dJpTGJ8ZbFeWo1iuzbQPmt536GeC5YChN",
        "jwqNEHtM8gmFAThkBWzU2DQiUuK1rW52Z8zsHyxMtwxCMovLu5K",
        "jxXwNfemxGwZcxKGhfrwzfE4QfxxGm5mkYieHQCafFkb6QBf9Xo",
        "jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD",
    ];

    pub fn is_valid<P>(path: P) -> bool
    where
        P: AsRef<Path>,
        P: Into<PathBuf>,
    {
        crate::utility::functions::is_valid_file_name(path, &LedgerHash::is_valid)
    }

    pub fn split_ledger_path(path: &Path) -> (Network, u32, LedgerHash) {
        let (height, hash) = extract_height_and_hash(path);
        let network = extract_network(path);
        (network, height, LedgerHash::new_or_panic(hash.to_string()))
    }

    /// Parse a valid (compressed) ledger file
    pub async fn parse_file(path: &Path) -> anyhow::Result<Self> {
        let mut bytes = std::fs::read(path)?;
        let is_compressed = path.extension().is_some_and(|ext| ext == "gz");

        // decompress if needed
        if is_compressed {
            bytes = decompress_gzip(&bytes[..])?;
        }

        let file_name = path.file_stem().unwrap().to_str().map(|stem| {
            let stem = PathBuf::from(stem);

            if is_compressed {
                return stem
                    .file_stem()
                    .expect("valid ledger file")
                    .to_str()
                    .unwrap()
                    .into();
            }

            stem
        });
        trace!("Parsing staking ledger {:?}", file_name);

        let staking_ledger: Vec<StakingAccountJson> = serde_json::from_slice(&bytes)
            .with_context(|| format!("Failed reading staking ledger {}", path.display()))?;

        let staking_ledger: HashMap<PublicKey, StakingAccount> = staking_ledger
            .into_iter()
            .map(|acct| (acct.pk.clone(), acct.into()))
            .collect();

        let (network, epoch, ledger_hash) = Self::split_ledger_path(path);
        let total_currency: u64 = staking_ledger.values().map(|account| account.balance).sum();
        let genesis_state_hash = Self::genesis_state_hash(&ledger_hash);

        Ok(Self {
            epoch,
            network,
            total_currency,
            ledger_hash,
            staking_ledger,
            genesis_state_hash,
        })
    }

    pub fn genesis_state_hash(ledger_hash: &LedgerHash) -> StateHash {
        if Self::V1_STAKING_LEDGER_HASHES.contains(&(&ledger_hash.0 as &str)) {
            MAINNET_GENESIS_HASH.into()
        } else {
            HARDFORK_GENESIS_HASH.into()
        }
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
                        delegates: HashSet::from([pk.clone(); 1]),
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
                        delegates.insert(pk.clone());
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
        format!("{}-{}-{}", self.network, self.epoch, self.ledger_hash)
    }
}

/////////////////
// conversions //
/////////////////

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
    use std::{collections::HashSet, path::PathBuf};

    #[tokio::test]
    async fn parse_file() -> anyhow::Result<()> {
        let path: PathBuf = "../tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path).await?;

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
        use crate::base::public_key::PublicKey;

        let path: PathBuf = "../tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json".into();
        let staking_ledger = StakingLedger::parse_file(&path).await?;
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
        let pk_delegations = delegations.get(&pk).cloned().unwrap();
        assert_eq!(pk_delegations.pk, pk);
        assert_eq!(pk_delegations.count_delegates, Some(25));
        assert_eq!(pk_delegations.total_delegated, Some(13277838425206999));
        assert_eq!(total_delegations, 794268782956784283);
        assert_eq!(genesis_state_hash.0, MAINNET_GENESIS_HASH.to_string());

        let expected_delegates: HashSet<PublicKey> = [
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
        assert_eq!(pk_delegations.delegates, expected_delegates);

        Ok(())
    }
}
