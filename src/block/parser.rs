use crate::{
    block::{
        get_blockchain_length, get_state_hash, is_valid_block_file,
        precomputed::{BlockLogContents, PrecomputedBlock},
    },
    BLOCK_REPORTING_FREQ_NUM, MAINNET_CANONICAL_THRESHOLD,
};
use glob::glob;
use std::{
    fs::File,
    io::{prelude::*, SeekFrom},
    path::{Path, PathBuf},
    time::Instant,
    u32::MAX,
    vec::IntoIter,
};
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

use super::extract_global_slot_since_genesis;

pub enum SearchRecursion {
    None,
    Recursive,
}

/// Splits block paths into two collections: canonical and successive
///
/// Traverses canoncial paths first, then successive
pub struct BlockParser {
    pub blocks_dir: PathBuf,
    pub recursion: SearchRecursion,
    canonical_paths: IntoIter<PathBuf>,
    successive_paths: IntoIter<PathBuf>,
}

impl BlockParser {
    pub fn new(blocks_dir: &Path) -> anyhow::Result<Self> {
        Self::new_internal(blocks_dir, SearchRecursion::None)
    }

    pub fn new_recursive(blocks_dir: &Path) -> anyhow::Result<Self> {
        Self::new_internal(blocks_dir, SearchRecursion::Recursive)
    }

    pub fn new_testing(blocks_dir: &Path) -> anyhow::Result<Self> {
        if blocks_dir.exists() {
            let blocks_dir = blocks_dir.to_owned();
            let paths: Vec<PathBuf> = glob(&format!("{}/*.json", blocks_dir.display()))
                .expect("Failed to read glob pattern")
                .filter_map(|x| x.ok())
                .collect();

            Ok(Self {
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

    fn new_internal(blocks_dir: &Path, recursion: SearchRecursion) -> anyhow::Result<Self> {
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
                .collect();

            let mut successive_paths = vec![];
            let mut canonical_paths = vec![];

            if !paths.is_empty() {
                info!("Sorting startup blocks by length");

                let time = Instant::now();
                paths.sort_by(|x, y| {
                    length_from_path(x)
                        .unwrap_or(MAX)
                        .cmp(&length_from_path(y).unwrap_or(MAX))
                });

                info!(
                    "{} blocks sorted by length in {:?}",
                    paths.len(),
                    time.elapsed()
                );
                info!("Searching for canonical chain in startup blocks");

                let mut length_start_indices = vec![];
                let mut curr_length = length_from_path(paths.first().unwrap()).unwrap();

                // build the length_start_indices vec corresponding to the
                // longest contiguous chain starting from the lowest block
                for (idx, path) in paths.iter().enumerate() {
                    let height = length_from_path(path).unwrap_or(MAX);
                    if idx == 0 || height > curr_length {
                        length_start_indices.push(idx);
                        curr_length = height;
                    } else {
                        continue;
                    }
                }

                // check that there are enough contiguous blocks
                let check_lengths = length_start_indices
                    .iter()
                    .take(MAINNET_CANONICAL_THRESHOLD as usize + 1)
                    .map(|idx| length_from_path(paths.get(*idx).unwrap()).unwrap_or(MAX));

                let check = check_lengths.enumerate().fold(None, |acc, (n, x)| {
                    if acc.is_none() && n == 0 || x == acc.unwrap_or(0) + 1 {
                        Some(x)
                    } else {
                        None
                    }
                });

                if check.is_none() {
                    info!("No canoncial blocks can be confidently found. Adding all blocks to the witness tree.");
                    return Ok(Self {
                        blocks_dir,
                        recursion,
                        canonical_paths: vec![].into_iter(),
                        successive_paths: paths.into_iter(),
                    });
                }

                let (max_start_idx, max_length_idx) =
                    if length_from_path(paths.last().unwrap()).is_some() {
                        (
                            length_start_indices.len() - 1,
                            *length_start_indices.last().unwrap(),
                        )
                    } else {
                        (
                            length_start_indices.len() - 2,
                            length_start_indices[length_start_indices.len() - 2],
                        )
                    };

                // backtrack canonical_threshold blocks to find a canonical one
                let mut curr_start_idx = max_start_idx;
                let mut curr_length_idx = max_length_idx;
                let mut curr_path = paths.get(curr_length_idx).unwrap();
                let time = Instant::now();

                for _ in 1..=MAINNET_CANONICAL_THRESHOLD {
                    if curr_start_idx > 0 {
                        let prev_length_idx = length_start_indices[curr_start_idx - 1];

                        for path in paths[prev_length_idx..curr_length_idx].iter() {
                            if extract_parent_hash_from_path(curr_path).unwrap()
                                == hash_from_path(path).unwrap()
                            {
                                curr_path = path;
                                curr_length_idx = prev_length_idx;
                                curr_start_idx -= 1;
                                continue;
                            }
                        }
                    }
                }

                let successive_idx = length_start_indices[curr_start_idx + 1];

                // curr_path represents a canonical block
                info!(
                    "Found canonical tip {} in {:?}",
                    curr_path.file_name().unwrap().to_str().unwrap(),
                    time.elapsed()
                );

                canonical_paths.push(curr_path.clone());
                info!("Walking the canonical chain back to the beginning, Will report every {BLOCK_REPORTING_FREQ_NUM} blocks found.", );

                let time = Instant::now();
                let mut count = 1;
                while curr_start_idx > 0 {
                    if count % BLOCK_REPORTING_FREQ_NUM == 0 {
                        info!("Found {count} canonical blocks in {:?}", time.elapsed());
                    }

                    let prev_length_idx = if curr_start_idx > 0 {
                        length_start_indices[curr_start_idx - 1]
                    } else {
                        0
                    };

                    for path in paths[prev_length_idx..curr_length_idx].iter() {
                        if extract_parent_hash_from_path(curr_path).unwrap()
                            == hash_from_path(path).unwrap()
                        {
                            canonical_paths.push(path.clone());
                            curr_path = path;
                            curr_length_idx = prev_length_idx;
                            count += 1;
                            curr_start_idx -= 1;
                            continue;
                        }
                    }
                }

                // final canonical block
                for path in paths[0..curr_length_idx].iter() {
                    if extract_parent_hash_from_path(curr_path).unwrap()
                        == hash_from_path(path).unwrap()
                    {
                        canonical_paths.push(path.clone());
                        break;
                    }
                }

                info!("Canonical chain discovery finished");
                info!(
                    "Found {} blocks in the canonical chain in {:?}",
                    canonical_paths.len() + 1, // +1 for starting block
                    time.elapsed()
                );
                canonical_paths.reverse();

                // add all blocks successive to the canonical chain
                for path in paths[successive_idx..]
                    .iter()
                    .filter(|p| length_from_path(p).is_some())
                {
                    successive_paths.push(path.clone());
                }
            }

            Ok(Self {
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

    /// Traverse the internal paths. First canonical, then successive.
    pub async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        if let Some(next_path) = self.canonical_paths.next() {
            return Self::handle_path(&next_path).await;
        }

        if let Some(next_path) = self.successive_paths.next() {
            return Self::handle_path(&next_path).await;
        }

        Ok(None)
    }

    async fn handle_path(path: &Path) -> anyhow::Result<Option<PrecomputedBlock>> {
        if is_valid_block_file(path) {
            let blockchain_length =
                get_blockchain_length(path.file_name().expect("filename already checked"));
            let state_hash = get_state_hash(path.file_name().expect("filename already checked"))
                .expect("state hash already checked");

            let mut log_file = tokio::fs::File::open(&path).await?;
            let mut log_file_contents = Vec::new();

            log_file.read_to_end(&mut log_file_contents).await?;

            let global_slot_since_genesis = extract_global_slot_since_genesis(&PathBuf::from(path));

            let precomputed_block = PrecomputedBlock::from_log_contents(BlockLogContents {
                state_hash,
                blockchain_length,
                contents: log_file_contents,
                global_slot_since_genesis,
            })?;

            Ok(Some(precomputed_block))
        } else {
            Err(anyhow::Error::msg(format!(
                "Invalid block path: {:?}",
                path.display()
            )))
        }
    }

    /// get the precomputed block with supplied hash
    /// it must exist ahead of the current block parser file
    pub async fn get_precomputed_block(
        &mut self,
        state_hash: &str,
    ) -> anyhow::Result<PrecomputedBlock> {
        let error = anyhow::Error::msg(format!(
            "
[BlockPasrser::get_precomputed_block]
    Looking in blocks dir: {:?}
    Did not find state hash: {state_hash}
    It may have been skipped unintentionally!",
            self.blocks_dir
        ));
        let mut next_block = self.next().await?.ok_or(error)?;

        while next_block.state_hash != state_hash {
            next_block = self.next().await?.ok_or(anyhow::Error::msg(format!(
                "
[BlockPasrser::get_precomputed_block]
    Looking in blocks dir: {:?}
    Did not find state hash: {state_hash}
    It may have been skipped unintentionally!",
                self.blocks_dir
            )))?;
        }

        Ok(next_block)
    }

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

            let global_slot_since_genesis = extract_global_slot_since_genesis(&PathBuf::from(filename));

            let precomputed_block = PrecomputedBlock::from_log_contents(BlockLogContents {
                state_hash,
                blockchain_length,
                contents: log_file_contents,
                global_slot_since_genesis,
            })?;

            Ok(precomputed_block)
        } else {
            Err(anyhow::Error::msg(format!(
                "
[BlockParser::parse_file]
    Could not find valid block!
    {:} is not a valid precomputed block",
                filename.display()
            )))
        }
    }
}

fn length_from_path(path: &Path) -> Option<u32> {
    get_blockchain_length(path.file_name().unwrap())
}

fn hash_from_path(path: &Path) -> Option<String> {
    get_state_hash(path.file_name().unwrap())
}

fn extract_parent_hash_from_path(path: &Path) -> anyhow::Result<String> {
    let parent_hash_offset = 75;
    let parent_hash_length = 52;

    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(parent_hash_offset))?;
    let mut buf = vec![0; parent_hash_length];
    f.read_exact(&mut buf)?;
    let parent_hash = String::from_utf8(buf)?;
    Ok(parent_hash)
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
                println!("{:?}", x);
            });
        Vec::from(FILENAMES_INVALID)
            .into_iter()
            .map(OsString::from)
            .map(|x| get_blockchain_length(&x))
            .for_each(|x| {
                println!("{:?}", x);
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
