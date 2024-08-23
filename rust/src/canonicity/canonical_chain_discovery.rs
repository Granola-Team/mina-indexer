use crate::{
    block::{
        extract_block_height, extract_height_and_hash, previous_state_hash::*,
        sort_by_height_and_lexicographical_order,
    },
    collection::bounded_stack::BoundedStack,
};
use log::info;
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
            let branch_candidate = vec![root_file.to_owned()];
            queue.push_back(branch_candidate);
        }
    }

    let mut recent_canonical_heights: BoundedStack<u32> =
        BoundedStack::new(canonical_threshold as usize);

    while let Some(branch_candidate) = queue.pop_front() {
        log_progress(branch_candidate.len() as u32, reporting_freq, &time);
        if let Some(tip_candidate) = branch_candidate.last() {
            let (height, state_hash) = extract_height_and_hash(tip_candidate);
            let next_height = height + 1;
            if let Some(next_tips) = tree_map.get(&(next_height)) {
                let mut parent_found = false;
                for possible_next_tip in next_tips {
                    let prev_hash = PreviousStateHash::from_path(possible_next_tip)?.0;
                    if prev_hash == state_hash {
                        let mut next_branch_candidate = branch_candidate.clone();
                        next_branch_candidate.push(possible_next_tip);
                        canonical_branch = next_branch_candidate.clone();
                        queue.push_back(next_branch_candidate);
                        parent_found = true;
                    }
                }
                if parent_found
                    && !recent_canonical_heights
                        .clone()
                        .into_vec()
                        .contains(&next_height)
                {
                    recent_canonical_heights.push(next_height);
                }
            }
        }
    }

    let recent_heights_vec = recent_canonical_heights.into_vec();
    let (mut recent_paths, recent_paths_set) =
        if let Some(&split_height) = recent_heights_vec.first() {
            if let Some(split_index) = canonical_branch
                .iter()
                .rposition(|path| extract_block_height(path) <= split_height)
            {
                let recent_paths = canonical_branch.split_off(split_index);
                let recent_paths_set: HashSet<&PathBuf> = recent_paths.iter().copied().collect();
                (recent_paths, recent_paths_set)
            } else {
                (vec![], HashSet::new())
            }
        } else {
            (vec![], HashSet::new())
        };

    recent_paths.extend(recent_heights_vec.iter().flat_map(|&height| {
        tree_map
            .remove(&height)
            .into_iter()
            .flatten()
            .filter(|path| !recent_paths_set.contains(path))
    }));
    sort_by_height_and_lexicographical_order(&mut recent_paths);

    let canonical_set: HashSet<&PathBuf> = canonical_branch.iter().copied().collect();
    let mut orphaned_paths: Vec<&PathBuf> = tree_map
        .drain()
        .flat_map(|(_, paths)| {
            paths
                .into_iter()
                .filter(|path| !canonical_set.contains(path))
        })
        .collect();
    sort_by_height_and_lexicographical_order(&mut orphaned_paths);

    info!(
        "Found {} blocks in the canonical chain in {:?}",
        canonical_branch.len() + recent_paths.len(),
        time.elapsed(),
    );

    Ok((
        canonical_branch.into_iter().cloned().collect::<Vec<_>>(),
        recent_paths.into_iter().cloned().collect::<Vec<_>>(),
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
