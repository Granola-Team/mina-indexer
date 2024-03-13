use crate::{
    block::{extract_block_height, precomputed::PrecomputedBlock},
    canonicity::canonical_chain_discovery::discovery,
};
use anyhow::{anyhow, bail};
use glob::glob;
use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};
use tracing::info;

/// Splits block paths into three collections:
/// - _deep canonical_ (chain of canonical blocks with at least
///   `canonical_threshold` confirmations; blocks up to _canonical root_, which
///   becomes the root of the witness tree)
/// - _recent_ (descendents of the _canonical root_)
/// - _orphaned_ (blocks at or below the height of the _canonical root_)
///
/// Traverses deep canoncial, recent, then orphaned (orphaned paths bypass the
/// witness tree)
pub struct BlockParser {
    pub blocks_dir: PathBuf,
    pub blocks_processed: u32,
    pub total_num_blocks: u32,
    pub num_deep_canonical_blocks: u32,
    pub bytes_processed: u64,
    pub total_num_bytes: u64,
    pub deep_canonical_bytes: u64,
    canonical_paths: IntoIter<PathBuf>,
    recent_paths: IntoIter<PathBuf>,
    orphaned_paths: IntoIter<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockParserPaths {
    pub canonical_paths: Vec<PathBuf>,
    pub recent_paths: Vec<PathBuf>,
    pub orphaned_paths: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedBlock {
    Recent(PrecomputedBlock),
    DeepCanonical(PrecomputedBlock),
    Orphaned(PrecomputedBlock),
}

impl BlockParser {
    pub fn paths(&self) -> BlockParserPaths {
        BlockParserPaths {
            canonical_paths: self.canonical_paths.clone().collect(),
            recent_paths: self.recent_paths.clone().collect(),
            orphaned_paths: self.orphaned_paths.clone().collect(),
        }
    }

    /// Returns a new block parser which employs canonical chain discovery
    pub fn new_with_canonical_chain_discovery(
        blocks_dir: &Path,
        canonical_threshold: u32,
        reporting_freq: u32,
    ) -> anyhow::Result<Self> {
        Self::with_canonical_chain_discovery(
            blocks_dir,
            None,
            None,
            canonical_threshold,
            reporting_freq,
        )
    }

    /// Returns a new ccd block parser with paths filtered by max length
    pub fn new_with_canonical_chain_discovery_filtered(
        blocks_dir: &Path,
        block_length: u32,
        canonical_threshold: u32,
        reporting_freq: u32,
    ) -> anyhow::Result<Self> {
        Self::with_canonical_chain_discovery(
            blocks_dir,
            None,
            Some(block_length),
            canonical_threshold,
            reporting_freq,
        )
    }

    /// Returns a new length-sorted block parser with paths filtered by a min
    /// length
    pub fn new_length_sorted_min_filtered(
        blocks_dir: &Path,
        min_length_filter: Option<u32>,
    ) -> anyhow::Result<Self> {
        Self::new_length_sorted_filtered(blocks_dir, min_length_filter, None)
    }

    /// Returns a new length-sorted block parser with paths filtered by min or
    /// max length
    pub fn new_length_sorted_filtered(
        blocks_dir: &Path,
        min_length: Option<u32>,
        max_length: Option<u32>,
    ) -> anyhow::Result<Self> {
        if blocks_dir.exists() {
            let pattern = format!("{}/*-*-*.json", blocks_dir.display());
            let blocks_dir = blocks_dir.to_owned();
            let mut paths: Vec<PathBuf> = glob(&pattern)?.filter_map(|x| x.ok()).collect();
            let total_num_bytes = paths
                .iter()
                .fold(0, |acc, p| acc + p.metadata().unwrap().len());

            if min_length.is_some() {
                paths.retain(|p| extract_block_height(p) > min_length)
            }

            if max_length.is_some() {
                paths.retain(|p| extract_block_height(p) < max_length)
            }

            paths.sort_by_cached_key(|path| extract_block_height(path));
            Ok(Self {
                blocks_dir,
                total_num_bytes,
                bytes_processed: 0,
                blocks_processed: 0,
                deep_canonical_bytes: 0,
                num_deep_canonical_blocks: 0,
                total_num_blocks: paths.len() as u32,
                recent_paths: paths.into_iter(),
                canonical_paths: vec![].into_iter(),
                orphaned_paths: vec![].into_iter(),
            })
        } else {
            Ok(Self::empty(blocks_dir, &[]))
        }
    }

    /// Length-sorted parser for testing without canonical chain discovery
    pub fn new_testing(blocks_dir: &Path) -> anyhow::Result<Self> {
        if blocks_dir.exists() {
            let blocks_dir = blocks_dir.to_owned();
            let mut paths: Vec<PathBuf> = glob(&format!("{}/*-*-*.json", blocks_dir.display()))?
                .filter_map(|x| x.ok())
                .collect();
            paths.sort_by_cached_key(|path| extract_block_height(path));

            println!("===== Testing block parser paths =====");
            for path in &paths {
                println!("{}", path.file_name().unwrap().to_str().unwrap());
            }
            println!("======================================");

            Ok(Self::empty(&blocks_dir, &paths))
        } else {
            bail!("blocks_dir: {:?}, does not exist!", blocks_dir)
        }
    }

    /// Length-sorts `block_dir`'s paths and performs _canonical chain
    /// discovery_ separating the block paths into two categories:
    /// - blocks known to be _canonical_
    /// - blocks that are higher than the _canonical root_
    fn with_canonical_chain_discovery(
        blocks_dir: &Path,
        min_len_filter: Option<u32>,
        max_len_filter: Option<u32>,
        canonical_threshold: u32,
        reporting_freq: u32,
    ) -> anyhow::Result<Self> {
        info!("Block parser with canonical chain discovery");
        if blocks_dir.exists() {
            let pattern = format!("{}/*-*-*.json", blocks_dir.display());
            let blocks_dir = blocks_dir.to_owned();
            let paths: Vec<PathBuf> = glob(&pattern)?.filter_map(|x| x.ok()).collect();
            let total_num_bytes = paths
                .iter()
                .fold(0, |acc, p| acc + p.metadata().unwrap().len());
            if let Ok((canonical_paths, recent_paths, orphaned_paths)) = discovery(
                min_len_filter,
                max_len_filter,
                canonical_threshold,
                reporting_freq,
                paths.iter().collect(),
            ) {
                info!("Canonical chain discovery successful...");
                let deep_canonical_bytes = canonical_paths
                    .iter()
                    .fold(0, |acc, p| acc + p.metadata().unwrap().len());
                Ok(Self {
                    blocks_processed: 0,
                    bytes_processed: 0,
                    total_num_bytes,
                    deep_canonical_bytes,
                    num_deep_canonical_blocks: canonical_paths.len() as u32,
                    total_num_blocks: (canonical_paths.len() + recent_paths.len()) as u32,
                    blocks_dir,
                    canonical_paths: canonical_paths.into_iter(),
                    recent_paths: recent_paths.into_iter(),
                    orphaned_paths: orphaned_paths.into_iter(),
                })
            } else {
                Ok(Self::empty(&blocks_dir, &paths))
            }
        } else {
            bail!("blocks_dir: {:?}, does not exist!", blocks_dir)
        }
    }

    fn update_block(
        &mut self,
        path: &Path,
        designation: &dyn Fn(PrecomputedBlock) -> ParsedBlock,
    ) -> anyhow::Result<Option<(ParsedBlock, u64)>> {
        let block_bytes = path.metadata().unwrap().len();
        match PrecomputedBlock::parse_file(path).map(designation) {
            Ok(parsed_block) => {
                self.blocks_processed += 1;
                self.bytes_processed += block_bytes;
                Ok(Some((parsed_block, block_bytes)))
            }
            Err(e) => bail!("Block parsing error: {}", e),
        }
    }
    /// Traverses `self`'s internal paths
    /// - deep canonical
    /// - recent
    /// - orphaned
    pub fn next_block(&mut self) -> anyhow::Result<Option<(ParsedBlock, u64)>> {
        if let Some(next_path) = self.canonical_paths.next() {
            return self.update_block(&next_path, &ParsedBlock::DeepCanonical);
        }

        if let Some(next_path) = self.recent_paths.next() {
            return self.update_block(&next_path, &ParsedBlock::Recent);
        }

        if let Some(next_path) = self.orphaned_paths.next() {
            return self.update_block(&next_path, &ParsedBlock::Orphaned);
        }

        Ok(None)
    }

    /// Gets the precomputed block with supplied `state_hash`, it must exist
    /// ahead of `self`'s current file
    pub async fn get_precomputed_block(
        &mut self,
        state_hash: &str,
    ) -> anyhow::Result<(PrecomputedBlock, u64)> {
        let mut next_block: (PrecomputedBlock, u64) = self
            .next_block()?
            .map(|p| (p.0.into(), p.1))
            .ok_or(anyhow!("Did not find state hash: {state_hash}"))?;

        while next_block.0.state_hash != state_hash {
            next_block = self
                .next_block()?
                .map(|p| (p.0.into(), p.1))
                .ok_or(anyhow!("Did not find state hash: {state_hash}"))?;
        }

        Ok(next_block)
    }

    fn empty(blocks_dir: &Path, paths: &[PathBuf]) -> Self {
        let total_num_bytes = paths
            .iter()
            .fold(0, |acc, p| acc + p.metadata().unwrap().len());
        Self {
            total_num_bytes,
            bytes_processed: 0,
            blocks_processed: 0,
            deep_canonical_bytes: 0,
            num_deep_canonical_blocks: 0,
            total_num_blocks: paths.len() as u32,
            blocks_dir: blocks_dir.to_path_buf(),
            canonical_paths: vec![].into_iter(),
            recent_paths: Vec::from(paths).into_iter(),
            orphaned_paths: vec![].into_iter(),
        }
    }
}

impl From<ParsedBlock> for PrecomputedBlock {
    fn from(value: ParsedBlock) -> Self {
        match value {
            ParsedBlock::DeepCanonical(b) => b,
            ParsedBlock::Orphaned(b) => b,
            ParsedBlock::Recent(b) => b,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::block::{
        blockchain_length::BlockchainLength, get_blockchain_length, is_valid_block_file,
        length_from_path, BlockHash,
    };
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
    };

    const FILENAMES_VALID: [&str; 23] = [
        "mainnet-113512-3NK9bewd5kDxzB5Kvyt8niqyiccbb365B2tLdEC2u9e8tG36ds5u.json",
        "mainnet-113518-3NLQ2Zop9dfDKvffNg9EBzSmBqyjYgCi2E1zAuLGFzUfJk6uq7YK.json",
        "mainnet-175222-3NKn7ZtT6Axw3hK3HpyUGRxmirkuUhtR4cYzWFk75NCgmjCcqPby.json",
        "mainnet-179591-3NLNMihHhdxEj78r88mK9JGTdyYuUWTP2hHD4yzJ4CvypjqYd2hv.json",
        "mainnet-179594-3NLBTeqaKMdY94Nu1QSnYMhq6qBSELH2HNJw4z8dYEXaJwgwnKey.json",
        "mainnet-195769-3NKbdBu8uaP41gnp2W2kSyEBDpYKqaSCxMdspoANXboxALK2g2Px.json",
        "mainnet-195770-3NK7CQdrzY5RBw9ugVjeQ2K6nR6dZSckP3Hrf18bopVg2LY8yrMy.json",
        "mainnet-196577-3NKPcXyRq9Ywe5e519n1DCNCNuY6fdDukuWXwrY4oWkDzdf3WWsF.json",
        "mainnet-206418-3NKS1csVgEyHj4sSeK2mi6aD2oCy5jYVd2ANhNT7ydo7oy1b5mYu.json",
        "mainnet-216651-3NLp9p3X8oF1ydSC1MgXnB99iJoSTTCV4qs4urmTKfiWTd6BbBsL.json",
        "mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw.json",
        "mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json",
        "mainnet-3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8.json",
        "mainnet-3NK2uq5kh6PwbUEwmhwR5RHfJNBgbwvwxxHQnKtQN5aYANudn3Wx.json",
        "mainnet-3NK2veoFnf9dKkqU7DUg4dAgQnapNaQUZZHHANK3kqaimKD1vFuv.json",
        "mainnet-3NK2xHq4mq5mBEG6jNhWTKSycG315pHwnZKdPqGYiyY58N3tn4oJ.json",
        "mainnet-3NK3c24DBH1aA83x3fhQLMC9UwFRUWVtFJG57o94MsDRqyDvR7us.json",
        "mainnet-40702-3NLkEG6S6Ra8Z1i5U5MPSNWV13hzQV8pYx1xBaeLDFN4EJhSuksw.json",
        "mainnet-750-3NLFkhrNBLRxh8cfCAHEFJSe29MEuT3HGNEcheXBKvexfRuEo9eC.json",
        "mainnet-84160-3NKJCCUhCqpueErQWmPMh67gk8uCY8ttFAK6bqG9xyF26rzjZBJ5.json",
        "mainnet-84161-3NK8iBQSkCQtCpnm2qWCvhixuEsiHQq7SL7YY31nyXkiLGEDMyGk.json",
        "mainnet-9638-3NL51H2ZPJUvuSFBaR56cEMqSt1ytiPpoHx7e6aQgEFNsVUPxSAn.json",
        "mainnet-9644-3NK4apiDvnT4ywWEw6KBEk1UzTd1XK7SGXFZDVC9GPCDaT3EXdsv.json",
    ];

    const FILENAMES_INVALID: [&str; 6] = [
        "mainnet-113512-3NK9bewd5kDxzB5Kvyt8niqyiccbb365B2tLdEC2u9e8tG36ds5u",
        "mainnet-113518-3NLQ2Zop9dfDKvffNg9EBzSmBqyjYgCi2E1zAuLGFzUfJk6uq7YK.j",
        "mainnet-175222.json",
        "LNMihHhdxEj78r88mK9JGTdyYuUWTP2hHD4yzJ4CvypjqYd2hv.json",
        "mainnet.json",
        "mainnet-195769-.json",
    ];

    #[test]
    fn blockchain_lengths_valid_or_default_none() {
        let expected: Vec<Option<u32>> = vec![
            Some(113512),
            Some(113518),
            Some(175222),
            Some(179591),
            Some(179594),
            Some(195769),
            Some(195770),
            Some(196577),
            Some(206418),
            Some(216651),
            Some(220897),
            Some(2),
            None,
            None,
            None,
            None,
            None,
            Some(40702),
            Some(750),
            Some(84160),
            Some(84161),
            Some(9638),
            Some(9644),
        ];
        let actual: Vec<Option<u32>> = Vec::from(FILENAMES_VALID)
            .iter()
            .map(|x| get_blockchain_length(&OsString::from(x)))
            .collect();

        assert_eq!(expected, actual);

        let expected: Vec<Option<u32>> = vec![None, None, None, None, None, None];
        let actual: Vec<Option<u32>> = Vec::from(FILENAMES_INVALID)
            .iter()
            .map(|x| length_from_path(Path::new(x)))
            .collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn blockchain_length_from_path() -> anyhow::Result<()> {
        let expected: Vec<u32> = vec![
            111, 113512, 113518, 175222, 179591, 179594, 195769, 195770, 196577, 2, 206418, 216651,
            220897, 320081, 5929, 26272, 32202, 31640, 31775, 40702, 750, 84160, 84161, 9638, 9644,
        ];
        let paths: Vec<PathBuf> = glob::glob("./tests/data/non_sequential_blocks/*")?
            .flatten()
            .collect();
        let actual: Vec<u32> = paths
            .iter()
            .flat_map(|x| {
                BlockchainLength::from_path(&PathBuf::from(x))
                    .map(<BlockchainLength as Into<u32>>::into)
                    .ok()
            })
            .collect();

        assert_eq!(expected, actual);

        let expected: Vec<Option<u32>> = vec![None, None, None, None, None, None];
        let actual: Vec<Option<u32>> = Vec::from(FILENAMES_INVALID)
            .iter()
            .map(|x| length_from_path(Path::new(x)))
            .collect();

        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn invalid_filenames_have_invalid_state_hash_or_non_json_extension() {
        FILENAMES_INVALID
            .map(PathBuf::from)
            .iter()
            .for_each(|file| assert!(!is_valid_block_file(file)))
    }

    #[test]
    fn valid_filenames_have_valid_state_hash_and_json_extension() {
        FILENAMES_VALID
            .map(PathBuf::from)
            .iter()
            .for_each(|file| assert!(is_valid_block_file(file)))
    }

    #[derive(Debug, Clone)]
    struct BlockFileName(PathBuf);

    #[derive(Debug, Clone)]
    enum Network {
        Mainnet,
        Devnet,
        Testworld,
        Berkeley,
    }

    impl Arbitrary for Network {
        fn arbitrary(g: &mut Gen) -> Self {
            let idx = usize::arbitrary(g) % 4;
            match idx {
                0 => Network::Mainnet,
                1 => Network::Devnet,
                2 => Network::Testworld,
                3 => Network::Berkeley,
                _ => panic!("should never happen"),
            }
        }
    }

    impl std::fmt::Display for Network {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let network = match self {
                Network::Mainnet => "mainnet",
                Network::Devnet => "devnet",
                Network::Testworld => "testworld",
                Network::Berkeley => "berkeley",
            };
            write!(f, "{}", network)
        }
    }

    impl Arbitrary for BlockHash {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut hash = "3N".to_string();
            for _ in 0..50 {
                let mut x = char::arbitrary(g);
                while !x.is_ascii_alphanumeric() {
                    x = char::arbitrary(g);
                }
                hash.push(x)
            }
            Self(hash)
        }
    }

    impl Arbitrary for BlockFileName {
        fn arbitrary(g: &mut Gen) -> Self {
            let network = Network::arbitrary(g);
            let height = u32::arbitrary(g);
            let hash = BlockHash::arbitrary(g);
            let is_first_pattern = bool::arbitrary(g);
            let path = if is_first_pattern {
                format!("{}-{}-{}.json", network, height, hash.0)
            } else {
                format!("{}-{}.json", network, hash.0)
            };
            Self(PathBuf::from(&path))
        }
    }

    #[quickcheck]
    fn check_for_block_file_validity(valid_block: BlockFileName) -> bool {
        is_valid_block_file(valid_block.0.as_path())
    }
}
