use super::StakingLedger;
use crate::{
    block::extract_height_and_hash,
    ledger::{store::staking::StakingLedgerStore, LedgerHash},
    store::IndexerStore,
};
use glob::glob;
use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};

pub struct StakingLedgerParser {
    pub ledgers_dir: PathBuf,
    pub ledger_paths: IntoIter<PathBuf>,
}

/// Staking ledgers have this format:
///  <network_name>-<epoch_number>-<ledger_hash>.json
/// or
///  <network_name>-<epoch_number>-<ledger_hash>.json.gz

impl StakingLedgerParser {
    pub fn new(ledgers_dir: &Path) -> anyhow::Result<Self> {
        let gzipped_paths = glob(&format!("{}/*-*-*.json.gz", ledgers_dir.display()))?
            .filter_map(|path| path.ok())
            .filter(|path| StakingLedger::is_valid(path));
        let ledger_paths: Vec<PathBuf> = gzipped_paths
            .chain(
                glob(&format!("{}/*-*-*.json", ledgers_dir.display()))?
                    .filter_map(|path| path.ok())
                    .filter(|path| StakingLedger::is_valid(path)),
            )
            .collect();

        Ok(Self {
            ledgers_dir: ledgers_dir.to_path_buf(),
            ledger_paths: ledger_paths.into_iter(),
        })
    }

    /// Only parse the staking ledger if it's not already in the db
    pub async fn next_ledger(
        &mut self,
        store: Option<&std::sync::Arc<IndexerStore>>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        for next_path in self.ledger_paths.by_ref() {
            if let Some(store) = store {
                // extract epoch and ledger hash to check if it's in the db
                let (epoch, hash) = extract_epoch_hash(&next_path);

                if store.get_staking_ledger_hash_by_epoch(epoch, None)? != Some(hash) {
                    // add the missing staking ledger
                    return StakingLedger::parse_file(&next_path).await.map(Some);
                } else {
                    continue;
                }
            }

            // parse all staking ledgers if no store
            return StakingLedger::parse_file(&next_path).await.map(Some);
        }

        Ok(None)
    }
}

pub fn extract_epoch_hash(path: &Path) -> (u32, LedgerHash) {
    let (epoch, hash) = extract_height_and_hash(path);
    (epoch, LedgerHash::new_or_panic(hash.to_string()))
}

#[cfg(test)]
mod tests {
    use super::StakingLedgerParser;
    use crate::{
        base::state_hash::StateHash,
        constants::{HARDFORK_GENESIS_HASH, MAINNET_GENESIS_HASH},
        ledger::hash::LedgerHash,
    };
    use std::{collections::HashSet, path::PathBuf, str::FromStr};

    #[tokio::test]
    async fn parser() -> anyhow::Result<()> {
        let ledgers_dir: PathBuf = "../tests/data/staking_ledgers".into();
        let mut ledger_parser = StakingLedgerParser::new(&ledgers_dir)?;

        #[derive(Debug, PartialEq, Eq, Hash)]
        struct StakingAccountInfo {
            epoch: u32,
            ledger_hash: LedgerHash,
            genesis_state_hash: StateHash,
            num_accounts: usize,
        }

        let expect = HashSet::from([
            // pre-hardfork
            StakingAccountInfo {
                epoch: 0,
                ledger_hash: LedgerHash::from_str(
                    "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
                )?,
                genesis_state_hash: MAINNET_GENESIS_HASH.into(),
                num_accounts: 1676,
            },
            StakingAccountInfo {
                epoch: 1,
                ledger_hash: LedgerHash::from_str(
                    "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
                )?,
                genesis_state_hash: MAINNET_GENESIS_HASH.into(),
                num_accounts: 1676,
            },
            StakingAccountInfo {
                epoch: 42,
                ledger_hash: LedgerHash::from_str(
                    "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH",
                )?,
                genesis_state_hash: MAINNET_GENESIS_HASH.into(),
                num_accounts: 130791,
            },
            // post-hardfork
            StakingAccountInfo {
                epoch: 0,
                ledger_hash: LedgerHash::from_str(
                    "jxsAidvKvEQJMC7Z2wkLrFGzCqUxpFMRhAj4K5o49eiFLhKSyXL",
                )?,
                genesis_state_hash: HARDFORK_GENESIS_HASH.into(),
                num_accounts: 226659,
            },
        ]);

        let mut res = HashSet::new();

        while let Some(staking_ledger) = ledger_parser.next_ledger(None).await? {
            res.insert(StakingAccountInfo {
                epoch: staking_ledger.epoch,
                ledger_hash: staking_ledger.ledger_hash.to_owned(),
                genesis_state_hash: staking_ledger.genesis_state_hash.to_owned(),
                num_accounts: staking_ledger.staking_ledger.len(),
            });
        }

        assert_eq!(res, expect);
        Ok(())
    }
}
