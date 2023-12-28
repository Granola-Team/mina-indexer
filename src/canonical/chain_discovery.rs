use crate::{
    block::{get_blockchain_length, get_state_hash, is_valid_block_file},
    BLOCK_REPORTING_FREQ_NUM,
};
use std::{
    fs::File,
    io::{prelude::*, SeekFrom},
    path::{Path, PathBuf},
    time::Instant,
};
use tracing::{debug, info};

pub fn discovery(
    length_filter: Option<u32>,
    canonical_threshold: u32,
    mut paths: Vec<&PathBuf>,
) -> anyhow::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    // separate all blocks into the canonical chain
    // and the blocks that follow the canonical tip
    let mut canonical_paths = vec![];
    let mut successive_paths = vec![];

    if !paths.is_empty() {
        info!("Sorting startup blocks by length");

        let time = Instant::now();
        paths.sort_by_key(|x| length_from_path_or_max(x));

        info!(
            "{} blocks sorted by length in {:?}",
            paths.len(),
            time.elapsed(),
        );

        if let Some(blockchain_length) = length_filter {
            info!("Applying block filter: blockchain_length < {blockchain_length}");
            let filtered_paths: Vec<&PathBuf> = paths
                .iter()
                .map_while(|&path| {
                    if length_from_path_or_max(path) < blockchain_length {
                        Some(path)
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
            if length > curr_length || idx == 0 {
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
            paths.as_slice(),
            &length_start_indices_and_diffs,
            length_start_indices_and_diffs
                .iter()
                .position(|x| x.0 == last_contiguous_start_idx)
                .unwrap_or(0),
            last_contiguous_idx,
            canonical_threshold,
        );

        if canonical_tip_opt.is_none()
            || max_num_canonical_blocks(&length_start_indices_and_diffs, last_contiguous_start_idx)
                < canonical_threshold
        {
            info!("No canoncial blocks can be confidently found. Adding all blocks to the witness tree.");
            return Ok((vec![], paths.iter().cloned().cloned().collect()));
        }

        // backtrack `MAINNET_CANONICAL_THRESHOLD` blocks from
        // the `last_contiguous_idx` to find the canonical tip
        let (mut curr_length_idx, mut curr_start_idx) = canonical_tip_opt.unwrap();
        let mut curr_path = paths[curr_length_idx];

        info!(
            "Found canonical tip (length {}): {}",
            length_from_path(curr_path).unwrap_or(0),
            hash_from_path(curr_path),
        );

        // handle all blocks that are higher than the canonical tip
        if let Some(successive_start_idx) =
            next_length_start_index(paths.as_slice(), curr_length_idx)
        {
            debug!("Handle successive blocks");
            if successive_start_idx < length_start_indices_and_diffs.len() {
                for path in paths[successive_start_idx..]
                    .iter()
                    .filter(|p| length_from_path(p).is_some())
                {
                    successive_paths.push(path.to_path_buf());
                }
            }
        }

        // collect the canonical blocks
        canonical_paths.push(curr_path.clone());

        if canonical_paths.len() < BLOCK_REPORTING_FREQ_NUM as usize {
            info!("Walking the canonical chain back to the beginning");
        } else {
            info!("Walking the canonical chain back to the beginning, reporting every {BLOCK_REPORTING_FREQ_NUM} blocks");
        }

        let time = Instant::now();
        let mut count = 1;

        // descend from the canonical tip to the lowest block in the dir,
        // segment by segment, searching for ancestors
        while curr_start_idx > 0 {
            if count % BLOCK_REPORTING_FREQ_NUM == 0 {
                info!("Found {count} canonical blocks in {:?}", time.elapsed());
            }

            // search for parent in previous segment's blocks
            let mut parent_found = false;
            let prev_length_idx = length_start_indices_and_diffs[curr_start_idx - 1].0;
            let parent_hash = extract_parent_hash_from_path(curr_path)?;

            for path in paths[prev_length_idx..curr_length_idx].iter() {
                if parent_hash == hash_from_path(path) {
                    canonical_paths.push(path.to_path_buf());
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
                return Ok((vec![], paths.iter().cloned().cloned().collect()));
            }
        }

        // push the lowest canonical block
        for path in paths[..curr_length_idx].iter() {
            if extract_parent_hash_from_path(curr_path)? == hash_from_path(path) {
                canonical_paths.push(path.to_path_buf());
                break;
            }
        }

        // sort lowest to highest
        canonical_paths.reverse();

        info!(
            "Found {} blocks in the canonical chain in {:?}",
            canonical_paths.len() + 1,
            time.elapsed(),
        );
    }

    Ok((canonical_paths.to_vec(), successive_paths.to_vec()))
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
fn next_length_start_index(paths: &[&PathBuf], path_idx: usize) -> Option<usize> {
    let length = length_from_path_or_max(paths[path_idx]);
    for (n, path) in paths[path_idx..].iter().enumerate() {
        if length_from_path_or_max(path) > length {
            return Some(path_idx + n);
        }
    }
    None
}

/// Finds the _canonical tip_, i.e. the _highest_ block in the
/// _lowest contiguous chain_ with `canonical_threshold` ancestors.
/// Unfortunately, the existence of this value does not necessarily imply
/// the existence of a canonical chain within the collection of blocks.
///
/// Returns the index of the caonical tip in `paths` and the start index of the first successive block.
fn find_canonical_tip(
    paths: &[&PathBuf],
    length_start_indices_and_diffs: &[(usize, u32)],
    mut curr_start_idx: usize,
    mut curr_length_idx: usize,
    canonical_threshold: u32,
) -> Option<(usize, usize)> {
    if length_start_indices_and_diffs.len() <= canonical_threshold as usize {
        None
    } else {
        let mut curr_path = &paths[curr_length_idx];

        for n in 1..=canonical_threshold {
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
                if curr_start_idx > canonical_threshold as usize {
                    return find_canonical_tip(
                        paths,
                        length_start_indices_and_diffs,
                        curr_start_idx.saturating_sub(1),
                        prev_length_start_idx,
                        canonical_threshold,
                    );
                } else {
                    // canonical tip cannot be found
                    return None;
                }
            }

            // canonical tip found
            if n == canonical_threshold && parent_found {
                break;
            }
        }
        Some((curr_length_idx, curr_start_idx))
    }
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
    if is_valid_block_file(path) {
        get_blockchain_length(path.file_name()?)
    } else {
        None
    }
}

fn length_from_path_or_max(path: &Path) -> u32 {
    length_from_path(path).unwrap_or(u32::MAX)
}

fn hash_from_path(path: &Path) -> String {
    get_state_hash(path.file_name().unwrap()).unwrap()
}
