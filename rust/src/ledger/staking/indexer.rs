//! Internal indexer staking ledger

use crate::{
    base::{amount::Amount, nonce::Nonce, public_key::PublicKey},
    chain::Network,
    constants::MINA_TOKEN_ADDRESS,
    ledger::{account::Account, hash::LedgerHash, token::TokenAddress, Ledger},
    store::{username::UsernameStore, IndexerStore},
};
use blake2::{digest::VariableOutput, Blake2bVar};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingAccount {
    pub pk: PublicKey,
    pub balance: Amount,
    pub delegate: Option<PublicKey>,
    pub token: TokenAddress,
    pub nonce: Option<Nonce>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingLedger(pub Vec<StakingAccount>);

impl StakingLedger {
    /// Staking ledger accounts sorted by:
    /// - token
    /// - balance
    /// - nonce
    /// - public key
    pub fn from_ledger(ledger: &Ledger) -> Self {
        let mut staking_accounts = Vec::with_capacity(ledger.len());

        for token_ledger in ledger.tokens.values() {
            for account in token_ledger.accounts.values() {
                staking_accounts.push(account.clone().deduct_mina_account_creation_fee().into());
            }
        }

        staking_accounts.sort();
        Self(staking_accounts)
    }

    pub fn ledger_hash(&self) -> LedgerHash {
        let mut hasher = Blake2bVar::new(35).expect("hasher");
        let bytes = serde_json::to_vec(self).expect("serialize staking ledger");
        hasher.write_all(bytes.as_slice()).expect("hash bytes");

        let hash = hasher.finalize_boxed().to_vec();
        LedgerHash("jxx".to_string() + &bs58::encode(hash).into_string())
    }
}

use std::{cmp::Ordering, collections::HashMap};

impl Ord for StakingAccount {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.token.0 as &str, &other.token.0 as &str) {
            // both MINA accounts
            (MINA_TOKEN_ADDRESS, token) if token != MINA_TOKEN_ADDRESS => Ordering::Greater,
            (token, MINA_TOKEN_ADDRESS) if token != MINA_TOKEN_ADDRESS => Ordering::Less,
            _ => {
                let token_cmp = self.token.cmp(&other.token);
                if token_cmp == Ordering::Equal {
                    let balance_cmp = self.balance.cmp(&other.balance);
                    if balance_cmp == Ordering::Equal {
                        let nonce_cmp = self.nonce.cmp(&other.nonce);
                        if nonce_cmp == Ordering::Equal {
                            return self.pk.cmp(&other.pk);
                        }

                        return nonce_cmp;
                    }

                    return balance_cmp;
                }

                token_cmp
            }
        }
    }
}

impl PartialOrd for StakingAccount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/////////////////
// conversions //
/////////////////

impl From<super::StakingLedger> for StakingLedger {
    fn from(value: super::StakingLedger) -> Self {
        let mut ledger = Self(Vec::with_capacity(value.staking_ledger.len()));
        for account in value.staking_ledger.into_values() {
            ledger.0.push(account.into());
        }

        ledger.0.sort();
        ledger
    }
}

impl From<(StakingLedger, u32, Network, &IndexerStore)> for super::StakingLedger {
    fn from(value: (StakingLedger, u32, Network, &IndexerStore)) -> Self {
        let ledger_hash = value.0.ledger_hash();
        let is_pre_hardfork = Self::V1_STAKING_LEDGER_HASHES.contains(&(&ledger_hash.0 as &str));
        let staking_ledger: HashMap<PublicKey, super::StakingAccount> = value
            .0
             .0
            .into_iter()
            .filter_map(|acct| {
                if is_pre_hardfork || acct.token.0 == MINA_TOKEN_ADDRESS {
                    Some((acct.pk.clone(), (acct, value.3).into()))
                } else {
                    None
                }
            })
            .collect();

        let total_currency: u64 = staking_ledger.values().map(|account| account.balance).sum();
        let genesis_state_hash = Self::genesis_state_hash(&ledger_hash);

        Self {
            epoch: value.1,
            network: value.2,
            total_currency,
            ledger_hash,
            staking_ledger,
            genesis_state_hash,
        }
    }
}

impl From<Account> for StakingAccount {
    fn from(value: Account) -> Self {
        Self {
            pk: value.public_key,
            balance: value.balance,
            delegate: Some(value.delegate),
            token: value.token.unwrap_or_default(),
            nonce: value.nonce,
        }
    }
}

impl From<super::StakingAccount> for StakingAccount {
    fn from(value: super::StakingAccount) -> Self {
        Self {
            pk: value.pk,
            balance: value.balance.into(),
            delegate: Some(value.delegate),
            token: value.token.unwrap_or_default(),
            nonce: value.nonce,
        }
    }
}

impl From<super::StakingAccountJson> for StakingAccount {
    fn from(value: super::StakingAccountJson) -> Self {
        let account: super::StakingAccount = value.into();
        account.into()
    }
}

impl From<(StakingAccount, &IndexerStore)> for super::StakingAccount {
    fn from(value: (StakingAccount, &IndexerStore)) -> Self {
        Self {
            delegate: value.0.delegate.unwrap_or_else(|| value.0.pk.clone()),
            username: value
                .1
                .get_username(&value.0.pk)
                .expect("username")
                .map(|u| u.0),
            pk: value.0.pk,
            balance: value.0.balance.0,
            token: Some(value.0.token),
            nonce: value.0.nonce,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StakingLedger;
    use crate::ledger::{genesis::GenesisLedger, staking, Ledger};
    use std::path::PathBuf;

    #[test]
    fn check_ledger_hash() -> anyhow::Result<()> {
        let path = PathBuf::from("../tests/data/staking_ledgers/mainnet-42-jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH.json");
        let staking_ledger = staking::StakingLedger::parse_file(&path)?;
        let staking_ledger: StakingLedger = staking_ledger.into();

        assert_eq!(
            staking_ledger.ledger_hash().0,
            "jxxKfx81sRrNBmZfXyMVVYYNQFHEvrKiX6RiSJUchfzNDnxjAY5"
        );

        Ok(())
    }

    #[test]
    fn check_staking_ledger() -> anyhow::Result<()> {
        let path = PathBuf::from("../tests/data/staking_ledgers/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json");
        let epoch_0_staking_ledger = staking::StakingLedger::parse_file(&path)?;
        let epoch_0_staking_ledger: StakingLedger = epoch_0_staking_ledger.into();

        let genesis_ledger = GenesisLedger::new_v1()?;
        let genesis_ledger: Ledger = genesis_ledger.into();
        let genesis_staking_ledger = StakingLedger::from_ledger(&genesis_ledger);

        assert_eq!(epoch_0_staking_ledger, genesis_staking_ledger);
        Ok(())
    }
}
