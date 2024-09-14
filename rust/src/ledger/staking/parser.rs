use super::{is_valid_ledger_file, StakingLedger};
use crate::{
    constants::MAINNET_GENESIS_HASH,
    ledger::{store::staking::StakingLedgerStore, LedgerHash},
    store::IndexerStore,
};
use glob::glob;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    vec::IntoIter,
};

pub struct StakingLedgerParser {
    pub ledgers_dir: PathBuf,
    pub ledger_paths: IntoIter<PathBuf>,
}

/// Staking ledgers have this format:
///  <network_name>-<epoch_number>-<ledger_hash>.json

impl StakingLedgerParser {
    pub fn new(ledgers_dir: &Path) -> anyhow::Result<Self> {
        let ledger_paths: Vec<PathBuf> = glob(&format!("{}/*-*-*.json", ledgers_dir.display()))?
            .filter_map(|path| path.ok())
            .filter(|path| is_valid_ledger_file(path))
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
                if let Some((epoch, hash)) = extract_epoch_hash(&next_path) {
                    if store.get_staking_ledger_hash_by_epoch(epoch, None)? != Some(hash) {
                        // add the missing staking ledger
                        return StakingLedger::parse_file(&next_path, MAINNET_GENESIS_HASH.into())
                            .await
                            .map(Some);
                    } else {
                        continue;
                    }
                }
            }

            // parse all staking ledgers if no store
            return StakingLedger::parse_file(&next_path, MAINNET_GENESIS_HASH.into())
                .await
                .map(Some);
        }
        Ok(None)
    }
}

pub fn extract_epoch_hash(path: &Path) -> Option<(u32, LedgerHash)> {
    let filename = path.file_stem().and_then(|x| x.to_str()).unwrap();
    let first_dash = filename.find('-');
    let second_dash =
        first_dash.and_then(|index| filename[index + 1..].find('-').map(|i| i + index + 1));
    if let (Some(first_dash_pos), Some(second_dash_pos)) = (first_dash, second_dash) {
        let potential_epoch = &filename[first_dash_pos + 1..second_dash_pos];
        let potential_hash = &filename[second_dash_pos + 1..];
        return potential_epoch.parse().ok().and_then(|e| {
            match LedgerHash::from_str(potential_hash) {
                Ok(ledger_hash) => Some((e, ledger_hash)),
                Err(_) => None,
            }
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::StakingLedgerParser;
    use std::path::PathBuf;

    #[tokio::test]
    async fn parser() -> anyhow::Result<()> {
        let ledgers_dir: PathBuf = "./tests/data/staking_ledgers".into();
        let mut n = 0;
        let mut ledger_parser = StakingLedgerParser::new(&ledgers_dir)?;
        let expect = vec![
            (
                0,
                "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee".to_string(),
            ),
            (
                42,
                "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH".to_string(),
            ),
        ];

        while let Some(staking_ledger) = ledger_parser.next_ledger(None).await? {
            assert_eq!(staking_ledger.epoch, expect[n].0);
            assert_eq!(staking_ledger.ledger_hash.0, expect[n].1);
            n += 1;
        }
        Ok(())
    }
}
