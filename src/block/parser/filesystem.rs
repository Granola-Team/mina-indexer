use crate::{
    block::{
        get_blockchain_length, get_state_hash, is_valid_block_file,
        precomputed::{BlockLogContents, PrecomputedBlock},
    },
    display_duration, BLOCK_REPORTING_FREQ_NUM, MAINNET_CANONICAL_THRESHOLD,
};
use async_trait::async_trait;
use glob::glob;
use std::{
    fs::File,
    io::{prelude::*, SeekFrom},
    path::{Path, PathBuf},
    time::Instant,
    vec::IntoIter,
};
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

use super::BlockParser;

pub enum SearchRecursion {
    None,
    Recursive,
}

/// Splits block paths into two collections: canonical and successive
///
/// Traverses canoncial paths first, then successive
pub struct FilesystemParser {
    pub num_canonical: u32,
    pub total_num_blocks: u32,
    pub blocks_dir: PathBuf,
    pub recursion: SearchRecursion,
    canonical_paths: IntoIter<PathBuf>,
    successive_paths: IntoIter<PathBuf>,
}

#[async_trait]
impl BlockParser for FilesystemParser {
    async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        self.next().await
    }
}

impl std::fmt::Debug for FilesystemParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recursion = match self.recursion {
            SearchRecursion::None => "None",
            SearchRecursion::Recursive => "Recursive",
        }.to_string();
        f.debug_struct("FilesystemParser")
            .field("num_canonical", &self.num_canonical)
            .field("total_num_blocks", &self.total_num_blocks)
            .field("blocks_dir", &self.blocks_dir)
            .field("recursion", &recursion)
            .field("canonical_paths", &self.canonical_paths)
            .field("successive_paths", &self.successive_paths)
            .finish()
    }
}

impl FilesystemParser {
    pub fn new(blocks_dir: &Path) -> anyhow::Result<Self> {
        Self::new_internal(blocks_dir, SearchRecursion::None, None)
    }

    pub fn new_recursive(blocks_dir: &Path) -> anyhow::Result<Self> {
        Self::new_internal(blocks_dir, SearchRecursion::Recursive, None)
    }

    pub fn new_filtered(blocks_dir: &Path, blocklength: u32) -> anyhow::Result<Self> {
        Self::new_internal(blocks_dir, SearchRecursion::None, Some(blocklength))
    }

    /// Simplified `BlockParser` for testing without canonical chain discovery.
    pub fn new_testing(blocks_dir: &Path) -> anyhow::Result<Self> {
        if blocks_dir.exists() {
            let blocks_dir = blocks_dir.to_owned();
            let paths: Vec<PathBuf> = glob(&format!("{}/*.json", blocks_dir.display()))
                .expect("Failed to read glob pattern")
                .filter_map(|x| x.ok())
                .collect();

            Ok(Self {
                num_canonical: 0,
                total_num_blocks: paths.len() as u32,
                blocks_dir,
                recursion: SearchRecursion::None,
                canonical_paths: vec![].into_iter(),
                successive_paths: paths.into_iter(),
            })
        } else {
            Err(anyhow::Error::msg(format!(
                "[BlockParser::new_testing] log path {blocks_dir:?} does not exist!"
            )))
        }
    }

    /// Length-sorts `block_dir`'s paths and performs _canonical chain discovery_
    /// separating the block paths into two categories:
    /// - blocks known to be _canonical_
    /// - blocks that are higher than the canonical tip
    fn new_internal(
        blocks_dir: &Path,
        recursion: SearchRecursion,
        length_filter: Option<u32>,
    ) -> anyhow::Result<Self> {
        debug!("Building parser");
        if blocks_dir.exists() {
            let pattern = match &recursion {
                SearchRecursion::None => format!("{}/*.json", blocks_dir.display()),
                SearchRecursion::Recursive => format!("{}/**/*.json", blocks_dir.display()),
            };
            let blocks_dir = blocks_dir.to_owned();
            let mut paths: Vec<PathBuf> = glob(&pattern)
                .expect("Failed to read glob pattern")
                .filter_map(|x| x.ok())
                .filter(|path| length_from_path(path).is_some())
                .collect();

            // separate all blocks into the canonical chain
            // and the blocks that follow the canonical tip
            let mut canonical_paths = vec![];
            let mut successive_paths = vec![];

            if !paths.is_empty() {
                info!("Sorting startup blocks by length");

                let time = Instant::now();
                paths.sort_by_key(|x| length_from_path_or_max(x));

                info!(
                    "{} blocks sorted by length in {}",
                    paths.len(),
                    display_duration(time.elapsed()),
                );

                if let Some(blockchain_length) = length_filter {
                    info!("Applying block filter: blockchain_length < {blockchain_length}");
                    let filtered_paths: Vec<PathBuf> = paths
                        .iter()
                        .map_while(|path| {
                            if length_from_path_or_max(path) < blockchain_length {
                                Some(path.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    paths = filtered_paths;
                }

                // keep track of:
                // - diffs between blocks of successive lengths (to find gaps)
                // - starting index for each collection of blocks of a fixed length
                // - length of the current path under investigation
                let mut length_start_indices_and_diffs = vec![];
                let mut curr_length = length_from_path(paths.first().unwrap()).unwrap();

                info!("Searching for canonical chain in startup blocks");

                for (idx, path) in paths.iter().enumerate() {
                    let length = length_from_path_or_max(path);
                    if length > curr_length {
                        length_start_indices_and_diffs.push((idx, length - curr_length));
                        curr_length = length;
                    } else {
                        continue;
                    }
                }

                // check that there are enough contiguous blocks for a canonical chain
                let last_contiguous_first_noncontiguous_start_idx =
                    last_contiguous_first_noncontiguous_start_idx(&length_start_indices_and_diffs);
                let last_contiguous_start_idx = last_contiguous_first_noncontiguous_start_idx
                    .map(|i| i.0)
                    .unwrap_or(length_start_indices_and_diffs.last().unwrap().0);
                let last_contiguous_idx = last_contiguous_first_noncontiguous_start_idx
                    .map(|i| i.1.saturating_sub(1))
                    .unwrap_or(paths.len() - 1);
                let canonical_tip_opt = find_canonical_tip(
                    &paths,
                    &length_start_indices_and_diffs,
                    length_start_indices_and_diffs
                        .iter()
                        .position(|x| x.0 == last_contiguous_start_idx)
                        .unwrap_or(0),
                    last_contiguous_idx,
                );

                if canonical_tip_opt.is_none()
                    || max_num_canonical_blocks(
                        &length_start_indices_and_diffs,
                        last_contiguous_start_idx,
                    ) < MAINNET_CANONICAL_THRESHOLD
                {
                    info!("No canoncial blocks can be confidently found. Adding all blocks to the witness tree.");
                    return Ok(Self {
                        num_canonical: 0,
                        total_num_blocks: paths.len() as u32,
                        blocks_dir,
                        recursion,
                        canonical_paths: vec![].into_iter(),
                        successive_paths: paths.into_iter(),
                    });
                }

                // backtrack `MAINNET_CANONICAL_THRESHOLD` blocks from
                // the `last_contiguous_idx` to find the canonical tip
                let time = Instant::now();
                let (mut curr_length_idx, mut curr_start_idx) = canonical_tip_opt.unwrap();
                let mut curr_path = &paths[curr_length_idx];

                info!(
                    "Found canonical tip with length {} and hash {} in {}",
                    length_from_path(curr_path).unwrap_or(0),
                    hash_from_path(curr_path),
                    display_duration(time.elapsed()),
                );

                // handle all blocks that are higher than the canonical tip
                if let Some(successive_start_idx) = next_length_start_index(&paths, curr_length_idx)
                {
                    debug!("Handle successive blocks");
                    if successive_start_idx < length_start_indices_and_diffs.len() {
                        for path in paths[successive_start_idx..]
                            .iter()
                            .filter(|p| length_from_path(p).is_some())
                        {
                            successive_paths.push(path.clone());
                        }
                    }
                }

                // collect the canonical blocks
                canonical_paths.push(curr_path.clone());

                if canonical_paths.len() < BLOCK_REPORTING_FREQ_NUM as usize {
                    info!("Walking the canonical chain back to the beginning.");
                } else {
                    info!("Walking the canonical chain back to the beginning, reporting every {BLOCK_REPORTING_FREQ_NUM} blocks.");
                }

                let time = Instant::now();
                let mut count = 1;

                // descend from the canonical tip to the lowest block in the dir,
                // segment by segment, searching for ancestors
                while curr_start_idx > 0 {
                    if count % BLOCK_REPORTING_FREQ_NUM == 0 {
                        info!(
                            "Found {count} canonical blocks in {}",
                            display_duration(time.elapsed())
                        );
                    }

                    // search for parent in previous segment's blocks
                    let mut parent_found = false;
                    let prev_length_idx = length_start_indices_and_diffs[curr_start_idx - 1].0;
                    let parent_hash = extract_parent_hash_from_path(curr_path)?;

                    for path in paths[prev_length_idx..curr_length_idx].iter() {
                        if parent_hash == hash_from_path(path) {
                            canonical_paths.push(path.clone());
                            curr_path = path;
                            curr_length_idx = prev_length_idx;
                            count += 1;
                            curr_start_idx -= 1;
                            parent_found = true;
                            continue;
                        }
                    }

                    // handle case where we fail to find parent
                    if !parent_found {
                        info!(
                            "Unable to locate parent block: mainnet-{}-{parent_hash}.json",
                            length_from_path_or_max(curr_path) - 1,
                        );
                        return Ok(Self {
                            num_canonical: 0,
                            total_num_blocks: paths.len() as u32,
                            blocks_dir,
                            recursion,
                            canonical_paths: vec![].into_iter(),
                            successive_paths: paths.into_iter(),
                        });
                    }
                }

                // push the lowest canonical block
                for path in paths[..curr_length_idx].iter() {
                    if extract_parent_hash_from_path(curr_path)? == hash_from_path(path) {
                        canonical_paths.push(path.clone());
                        break;
                    }
                }

                info!("Canonical chain discovery finished!");
                info!(
                    "Found {} blocks in the canonical chain in {}",
                    canonical_paths.len(),
                    display_duration(time.elapsed()),
                );

                // sort lowest to highest
                canonical_paths.reverse();
            }

            Ok(Self {
                num_canonical: canonical_paths.len() as u32,
                total_num_blocks: (canonical_paths.len() + successive_paths.len()) as u32,
                blocks_dir,
                recursion,
                canonical_paths: canonical_paths.into_iter(),
                successive_paths: successive_paths.into_iter(),
            })
        } else {
            Err(anyhow::Error::msg(format!(
                "[BlockParser::new_internal] log path {blocks_dir:?} does not exist!"
            )))
        }
    }

    /// Traverses `self`'s internal paths. First canonical, then successive.
    pub async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        if let Some(next_path) = self.canonical_paths.next() {
            return self.parse_file(&next_path).await.map(Some);
        }

        if let Some(next_path) = self.successive_paths.next() {
            return self.parse_file(&next_path).await.map(Some);
        }

        Ok(None)
    }

    /// Gets the precomputed block with supplied `state_hash`, it must exist ahead
    /// of `self`'s current file in the order imposed by glob/filesystem.
    pub async fn get_precomputed_block(
        &mut self,
        state_hash: &str,
    ) -> anyhow::Result<PrecomputedBlock> {
        let mut next_block = self.next().await?.ok_or(anyhow::Error::msg(format!(
            "
[BlockPasrser::get_precomputed_block]
    Looking in blocks dir: {}
    Did not find state hash: {state_hash}
    It may have been skipped unintentionally!",
            self.blocks_dir.display()
        )))?;

        while next_block.state_hash != state_hash {
            next_block = self.next().await?.ok_or(anyhow::Error::msg(format!(
                "
[BlockPasrser::get_precomputed_block]
    Looking in blocks dir: {}
    Did not find state hash: {state_hash}
    It may have been skipped unintentionally!",
                self.blocks_dir.display()
            )))?;
        }

        Ok(next_block)
    }

    /// Parses the precomputed block's JSON file, throws if a read error occurs.
    pub async fn parse_file(&mut self, filename: &Path) -> anyhow::Result<PrecomputedBlock> {
        if is_valid_block_file(filename) {
            let blockchain_length =
                get_blockchain_length(filename.file_name().expect("filename already checked"));
            let state_hash =
                get_state_hash(filename.file_name().expect("filename already checked"))
                    .expect("state hash already checked");

            let mut log_file = tokio::fs::File::open(&filename).await?;
            let mut log_file_contents = Vec::new();

            log_file.read_to_end(&mut log_file_contents).await?;
            drop(log_file);
            let precomputed_block = PrecomputedBlock::from_log_contents(BlockLogContents {
                state_hash,
                blockchain_length,
                contents: log_file_contents,
            })?;

            Ok(precomputed_block)
        } else {
            Err(anyhow::Error::msg(format!(
                "
[BlockParser::parse_file]
    Could not find valid block!
    {} is not a valid precomputed block",
                filename.display()
            )))
        }
    }
}

/// Gets the parent hash from the contents of the block's JSON file.
/// This function depends on the current JSON layout for precomputed blocks
/// and should be modified to use a custom `prev_state_hash` field deserializer.
fn extract_parent_hash_from_path(path: &Path) -> anyhow::Result<String> {
    let mut parent_hash_offset = 75;
    let mut parent_hash = read_parent_hash(path, parent_hash_offset)?;

    while !parent_hash.starts_with("3N") {
        parent_hash_offset += 1;
        parent_hash = read_parent_hash(path, parent_hash_offset)?;
    }
    Ok(parent_hash)
}

fn read_parent_hash(path: &Path, parent_hash_offset: u64) -> anyhow::Result<String> {
    let parent_hash_length = 52;
    let mut f = File::open(path)?;
    let mut buf = vec![0; parent_hash_length];

    f.seek(SeekFrom::Start(parent_hash_offset))?;
    f.read_exact(&mut buf)?;
    drop(f);
    String::from_utf8(buf).map_err(anyhow::Error::from)
}

/// Checks if the block at `curr_path` is the _parent_ of the block at `path`.
fn is_parent(path: &Path, curr_path: &Path) -> bool {
    extract_parent_hash_from_path(curr_path).unwrap() == hash_from_path(path)
}

/// Returns the start index of the paths with next higher length.
fn next_length_start_index(paths: &[PathBuf], path_idx: usize) -> Option<usize> {
    let length = length_from_path_or_max(&paths[path_idx]);
    for (n, path) in paths[path_idx..].iter().enumerate() {
        if length_from_path_or_max(path) > length {
            return Some(path_idx + n);
        }
    }
    None
}

/// Finds the _canonical tip_, i.e. the _highest_ block in the
/// _lowest contiguous chain_ with `MAINNET_CANONICAL_THRESHOLD` ancestors.
/// Unfortunately, the existence of this value does not necessarily imply
/// the existence of a canonical chain within the collection of blocks.
///
/// Returns the index of the caonical tip in `paths` and the start index of the first successive block.
fn find_canonical_tip(
    paths: &[PathBuf],
    length_start_indices_and_diffs: &[(usize, u32)],
    mut curr_start_idx: usize,
    mut curr_length_idx: usize,
) -> Option<(usize, usize)> {
    let mut curr_path = &paths[curr_length_idx];

    for n in 1..=MAINNET_CANONICAL_THRESHOLD {
        let mut parent_found = false;
        let prev_length_start_idx = if curr_start_idx > 0 {
            length_start_indices_and_diffs[curr_start_idx - 1].0
        } else {
            0
        };

        for path in paths[prev_length_start_idx..curr_length_idx].iter() {
            // if the parent is found, check that it has a parent, etc
            if is_parent(path, curr_path) {
                curr_path = path;
                curr_length_idx = prev_length_start_idx;
                curr_start_idx = curr_start_idx.saturating_sub(1);
                parent_found = true;
                continue;
            }
        }

        // if a parent was not found
        if !parent_found {
            // begin the search again at the previous length
            if curr_start_idx > MAINNET_CANONICAL_THRESHOLD as usize {
                return find_canonical_tip(
                    paths,
                    length_start_indices_and_diffs,
                    curr_start_idx.saturating_sub(1),
                    prev_length_start_idx,
                );
            } else {
                // canonical tip cannot be found
                return None;
            }
        }

        // canonical tip found
        if n == MAINNET_CANONICAL_THRESHOLD && parent_found {
            break;
        }
    }
    Some((curr_length_idx, curr_start_idx))
}

/// Finds the index of the _highest possible block in the lowest contiguous chain_
/// and the starting index of the next higher blocks.
fn last_contiguous_first_noncontiguous_start_idx(
    length_start_indices_and_diffs: &[(usize, u32)],
) -> Option<(usize, usize)> {
    let mut prev = 0;
    for (idx, diff) in length_start_indices_and_diffs.iter() {
        if *diff > 1 {
            return Some((prev, *idx));
        } else {
            prev = *idx;
        }
    }
    None
}

fn max_num_canonical_blocks(
    length_start_indices_and_diffs: &[(usize, u32)],
    last_contiguous_start_idx: usize,
) -> u32 {
    length_start_indices_and_diffs
        .iter()
        .position(|x| x.0 == last_contiguous_start_idx)
        .unwrap_or(0) as u32
        + 1
}

// path helpers
fn length_from_path(path: &Path) -> Option<u32> {
    get_blockchain_length(path.file_name()?)
}

fn length_from_path_or_max(path: &Path) -> u32 {
    length_from_path(path).unwrap_or(u32::MAX)
}

fn hash_from_path(path: &Path) -> String {
    get_state_hash(path.file_name().unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use crate::block::{get_blockchain_length, is_valid_block_file};

    const FILENAMES_VALID: [&'static str; 23] = [
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

    const FILENAMES_INVALID: [&'static str; 6] = [
        "mainnet-113512-3NK9bewd5kDxzB5Kvyt8niqyiccbb365B2tLdEC2u9e8tG36ds5u",
        "mainnet-113518-3NLQ2Zop9dfDKvffNg9EBzSmBqyjYgCi2E1zAuLGFzUfJk6uq7YK.j",
        "mainnet-175222.json",
        "LNMihHhdxEj78r88mK9JGTdyYuUWTP2hHD4yzJ4CvypjqYd2hv.json",
        "mainnet.json",
        "mainnet-195769-.json",
    ];

    #[test]
    fn blockchain_lengths_valid_or_default_none() {
        Vec::from(FILENAMES_VALID)
            .into_iter()
            .map(OsString::from)
            .map(|x| get_blockchain_length(&x))
            .for_each(|x| {
                println!("{x:?}");
            });
        Vec::from(FILENAMES_INVALID)
            .into_iter()
            .map(OsString::from)
            .map(|x| get_blockchain_length(&x))
            .for_each(|x| {
                println!("{x:?}");
            });
    }

    #[test]
    fn invalid_filenames_have_invalid_state_hash_or_non_json_extension() {
        Vec::from(FILENAMES_INVALID)
            .into_iter()
            .map(OsString::from)
            .map(|os_string| {
                (
                    os_string.clone(),
                    is_valid_block_file(&PathBuf::from(os_string)),
                )
            })
            .for_each(|(os_string, result)| {
                dbg!(os_string);
                assert!(result == false)
            });
    }

    #[test]
    fn valid_filenames_have_valid_state_hash_and_json_extension() {
        Vec::from(FILENAMES_VALID)
            .into_iter()
            .map(OsString::from)
            .map(|os_string| {
                (
                    os_string.clone(),
                    is_valid_block_file(&PathBuf::from(os_string)),
                )
            })
            .for_each(|(os_string, result)| {
                dbg!(os_string);
                assert!(result == true)
            });
    }
}