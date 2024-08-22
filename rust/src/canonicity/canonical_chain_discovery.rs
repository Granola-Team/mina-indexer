use crate::{
    block::{extract_block_height, extract_height_and_hash, previous_state_hash::*},
    collection::bounded_stack::BoundedStack,
};
use log::info;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

// discovers the canonical chain, orphaned blocks, and
// recent blocks within the canonical threshold
pub fn discovery(
    canonical_threshold: u32,
    reporting_freq: u32,
    paths: Vec<&PathBuf>,
) -> anyhow::Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>)> {
    let time = std::time::Instant::now();
    let mut tree_map: HashMap<u32, Vec<&PathBuf>> = HashMap::new();
    let mut lowest_precomputed_block_height = u32::MAX;

    for path in paths {
        let height = extract_block_height(path);
        lowest_precomputed_block_height = std::cmp::min(height, lowest_precomputed_block_height);
        // store multiple paths at a given height
        tree_map.entry(height).or_default().push(path);
    }

    let mut queue: VecDeque<Vec<&PathBuf>> = VecDeque::new();
    let mut canonical_branch: Vec<&PathBuf> = Vec::new();

    if let Some(root_files) = tree_map.get(&lowest_precomputed_block_height) {
        for root_file in root_files {
            queue.push_back(vec![root_file.to_owned()]);
        }
    }

    let mut orphaned_paths: Vec<&PathBuf> = vec![];
    let mut recent_paths: BoundedStack<&PathBuf> = BoundedStack::new(canonical_threshold as usize);

    while let Some(canonical_branch_copy) = queue.pop_front() {
        log_progress(canonical_branch.len() as u32, reporting_freq, &time);
        if let Some(current_tip) = canonical_branch_copy.last() {
            let (height, state_hash) = extract_height_and_hash(current_tip);
            if let Some(possible_next_tips) = tree_map.get(&(height + 1)) {
                for possible_next_tip in possible_next_tips {
                    let prev_hash = PreviousStateHash::from_path(possible_next_tip)?.0;
                    if prev_hash == state_hash {
                        let mut next_canonical_branch = canonical_branch_copy.clone();
                        if let Some(next_confirmed_tip) = recent_paths.push(possible_next_tip) {
                            next_canonical_branch.push(next_confirmed_tip);
                        }
                        canonical_branch = next_canonical_branch.clone();
                        queue.push_back(next_canonical_branch.clone());
                    } else {
                        orphaned_paths.push(possible_next_tip);
                    }
                }
            }
        }
    }

    info!(
        "Found {} blocks in the canonical chain in {:?}",
        canonical_branch.len() + recent_paths.clone().into_vec().len(),
        time.elapsed(),
    );

    Ok((
        canonical_branch.into_iter().cloned().collect::<Vec<_>>(),
        recent_paths
            .into_vec()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        orphaned_paths.into_iter().cloned().collect::<Vec<_>>(),
    ))
}

fn log_progress(length_of_chain: u32, reporting_freq: u32, time: &std::time::Instant) {
    if length_of_chain % reporting_freq == 0 {
        info!(
            "Found {} deep canonical blocks in {:?}",
            length_of_chain,
            time.elapsed()
        );
    }
}
