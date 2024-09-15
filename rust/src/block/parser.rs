use super::precomputed::PcbVersion;
use crate::{
    block::{extract_block_height, precomputed::PrecomputedBlock},
    canonicity::canonical_chain_discovery::discovery,
    utility::functions::calculate_total_size,
};
use anyhow::{anyhow, bail};
use glob::glob;
use log::info;
use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};

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
    pub num_recent_blocks: u32,
    pub num_deep_canonical_blocks: u32,
    pub bytes_processed: u64,
    pub total_num_bytes: u64,
    pub deep_canonical_bytes: u64,
    pub version: PcbVersion,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub async fn new_with_canonical_chain_discovery(
        blocks_dir: &Path,
        version: PcbVersion,
        canonical_threshold: u32,
        do_not_ingest_orphan_blocks: bool,
        reporting_freq: u32,
    ) -> anyhow::Result<Self> {
        Self::with_canonical_chain_discovery(
            blocks_dir,
            version,
            canonical_threshold,
            do_not_ingest_orphan_blocks,
            reporting_freq,
        )
        .await
    }

    /// Returns a new length-sorted block parser with paths filtered by a min
    /// length
    pub fn new_length_sorted_min_filtered(
        blocks_dir: &Path,
        version: PcbVersion,
        min_length_filter: Option<u32>,
    ) -> anyhow::Result<Self> {
        Self::new_length_sorted_filtered(blocks_dir, version, min_length_filter, None)
    }

    /// Returns a new length-sorted block parser with paths filtered by min or
    /// max length
    pub fn new_length_sorted_filtered(
        blocks_dir: &Path,
        version: PcbVersion,
        min_length: Option<u32>,
        max_length: Option<u32>,
    ) -> anyhow::Result<Self> {
        if blocks_dir.exists() {
            let blocks_dir = blocks_dir.to_owned();
            let mut paths: Vec<PathBuf> = glob(&format!("{}/*-*-*.json", blocks_dir.display()))?
                .filter_map(|x| x.ok())
                .collect();
            let total_num_bytes = paths
                .iter()
                .fold(0, |acc, p| acc + p.metadata().unwrap().len());

            if let Some(min) = min_length {
                paths.retain(|p| extract_block_height(p) > min)
            }

            if let Some(max) = max_length {
                paths.retain(|p| extract_block_height(p) < max)
            }

            paths.sort_by_cached_key(|path| extract_block_height(path));
            Ok(Self {
                version,
                blocks_dir,
                total_num_bytes,
                bytes_processed: 0,
                blocks_processed: 0,
                deep_canonical_bytes: 0,
                num_deep_canonical_blocks: 0,
                num_recent_blocks: paths.len() as u32,
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
    async fn with_canonical_chain_discovery(
        blocks_dir: &Path,
        version: PcbVersion,
        canonical_threshold: u32,
        do_not_ingest_orphan_blocks: bool,
        reporting_freq: u32,
    ) -> anyhow::Result<Self> {
        info!("Block parser with canonical chain discovery");
        if blocks_dir.exists() {
            let pattern = format!("{}/*-*-*.json", blocks_dir.display());
            let blocks_dir = blocks_dir.to_owned();
            let paths: Vec<PathBuf> = glob(&pattern)?.filter_map(|x| x.ok()).collect();
            if let Ok((canonical_paths, recent_paths, orphaned_paths)) =
                discovery(canonical_threshold, reporting_freq, paths.iter().collect())
            {
                info!("Canonical chain discovery successful");
                let deep_canonical_bytes = canonical_paths
                    .iter()
                    .fold(0, |acc, p| acc + p.metadata().unwrap().len());

                let total_num_bytes = if do_not_ingest_orphan_blocks {
                    calculate_total_size(&canonical_paths) + calculate_total_size(&recent_paths)
                } else {
                    calculate_total_size(&paths)
                };

                let total_num_blocks = if do_not_ingest_orphan_blocks {
                    canonical_paths.len() + recent_paths.len()
                } else {
                    paths.len()
                };

                Ok(Self {
                    version,
                    blocks_dir,
                    total_num_bytes,
                    bytes_processed: 0,
                    blocks_processed: 0,
                    deep_canonical_bytes,
                    num_deep_canonical_blocks: canonical_paths.len() as u32,
                    num_recent_blocks: recent_paths.len() as u32,
                    total_num_blocks: total_num_blocks as u32,
                    canonical_paths: canonical_paths.into_iter(),
                    recent_paths: recent_paths.into_iter(),
                    orphaned_paths: if do_not_ingest_orphan_blocks {
                        vec![].into_iter()
                    } else {
                        orphaned_paths.into_iter()
                    },
                })
            } else {
                Ok(Self::empty(&blocks_dir, &paths))
            }
        } else {
            bail!("blocks_dir {} does not exist!", blocks_dir.display())
        }
    }

    fn consume_block(
        &mut self,
        path: &Path,
        designation: &dyn Fn(PrecomputedBlock) -> ParsedBlock,
    ) -> anyhow::Result<Option<(ParsedBlock, u64)>> {
        let block_bytes = path.metadata().unwrap().len();
        match PrecomputedBlock::parse_file(path, self.version.clone()).map(designation) {
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
    pub async fn next_block(&mut self) -> anyhow::Result<Option<(ParsedBlock, u64)>> {
        if let Some(next_path) = self.canonical_paths.next() {
            return self.consume_block(&next_path, &ParsedBlock::DeepCanonical);
        }

        if let Some(next_path) = self.recent_paths.next() {
            return self.consume_block(&next_path, &ParsedBlock::Recent);
        }

        if let Some(next_path) = self.orphaned_paths.next() {
            return self.consume_block(&next_path, &ParsedBlock::Orphaned);
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
            .next_block()
            .await?
            .map(|p| (p.0.into(), p.1))
            .ok_or(anyhow!("Did not find state hash: {state_hash}"))?;

        while next_block.0.state_hash().0 != state_hash {
            next_block = self
                .next_block()
                .await?
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
            version: PcbVersion::default(),
            deep_canonical_bytes: 0,
            num_deep_canonical_blocks: 0,
            num_recent_blocks: paths.len() as u32,
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
    use crate::{block::BlockHash, chain::Network};
    use quickcheck::{Arbitrary, Gen};
    use std::path::PathBuf;
    impl Arbitrary for Network {
        fn arbitrary(g: &mut Gen) -> Self {
            let idx = usize::arbitrary(g) % 4;
            match idx {
                0 => Network::Mainnet,
                1 => Network::Devnet,
                2 => Network::Testworld,
                3 => Network::Berkeley,
                _ => panic!("unknown network {idx}"),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct BlockFileName(PathBuf);

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
}
