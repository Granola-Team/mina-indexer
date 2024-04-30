use super::{is_valid_ledger_file, StakingLedger};
use crate::block::precomputed::PcbVersion;
use glob::glob;
use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};

pub struct StakingLedgerParser {
    pub ledgers_dir: PathBuf,
    ledger_paths: IntoIter<PathBuf>,
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

    pub fn next_ledger(&mut self) -> anyhow::Result<Option<StakingLedger>> {
        if let Some(next_path) = self.ledger_paths.next() {
            return StakingLedger::parse_file(&next_path, PcbVersion::V1).map(Some);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::StakingLedgerParser;
    use std::path::PathBuf;

    #[test]
    fn parser() -> anyhow::Result<()> {
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

        while let Some(staking_ledger) = ledger_parser.next_ledger()? {
            assert_eq!(staking_ledger.epoch, expect[n].0);
            assert_eq!(staking_ledger.ledger_hash.0, expect[n].1);
            n += 1;
        }

        Ok(())
    }
}
