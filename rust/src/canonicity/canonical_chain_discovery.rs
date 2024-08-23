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

    find_best_tip(
        &mut queue,
        &mut canonical_branch,
        &mut tree_map,
        &mut recent_canonical_heights,
        &time,
        reporting_freq,
    );

    let recent_paths = split_off_recent_paths(
        &mut canonical_branch,
        &mut tree_map,
        recent_canonical_heights.into_vec(),
    );

    let orphaned_paths = get_orphaned_paths(&canonical_branch, &mut tree_map);

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

fn find_best_tip<'a>(
    queue: &mut VecDeque<Vec<&'a PathBuf>>,
    canonical_branch: &mut Vec<&'a PathBuf>,
    tree_map: &mut HashMap<u32, Vec<&'a PathBuf>>,
    recent_canonical_heights: &mut BoundedStack<u32>,
    time: &std::time::Instant,
    reporting_freq: u32,
) {
    while let Some(branch_candidate) = queue.pop_front() {
        log_progress(branch_candidate.len() as u32, reporting_freq, time);
        if let Some(tip_candidate) = branch_candidate.last() {
            let (height, state_hash) = extract_height_and_hash(tip_candidate);
            let next_height = height + 1;
            if let Some(next_tips) = tree_map.get(&(next_height)) {
                let mut parent_found = false;
                for possible_next_tip in next_tips {
                    if let Ok(prev_hash) = PreviousStateHash::from_path(possible_next_tip) {
                        if prev_hash.0 == state_hash {
                            let mut next_branch_candidate = branch_candidate.clone();
                            next_branch_candidate.push(possible_next_tip);
                            queue.push_back(next_branch_candidate);
                            parent_found = true;
                        }
                    }
                }
                if parent_found {
                    if !recent_canonical_heights
                        .clone()
                        .into_vec()
                        .contains(&next_height)
                    {
                        recent_canonical_heights.push(next_height);
                    }
                    canonical_branch.clear();
                    canonical_branch.extend(branch_candidate);
                }
            }
        }
    }
}

fn get_orphaned_paths<'a>(
    deep_canonical_branch: &[&'a PathBuf],
    tree_map: &mut HashMap<u32, Vec<&'a PathBuf>>,
) -> Vec<&'a PathBuf> {
    let canonical_set: HashSet<&PathBuf> = deep_canonical_branch.iter().copied().collect();
    let mut orphaned_paths: Vec<&PathBuf> = tree_map
        .drain()
        .flat_map(|(_, paths)| {
            paths
                .into_iter()
                .filter(|path| !canonical_set.contains(path))
        })
        .collect();
    sort_by_height_and_lexicographical_order(&mut orphaned_paths);
    orphaned_paths
}

fn split_off_recent_paths<'a>(
    canonical_branch: &mut Vec<&'a PathBuf>,
    tree_map: &mut HashMap<u32, Vec<&'a PathBuf>>,
    recent_heights: Vec<u32>,
) -> Vec<&'a PathBuf> {
    let (mut recent_paths, recent_paths_set) = if let Some(&split_height) = recent_heights.first() {
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
    recent_paths.extend(recent_heights.iter().flat_map(|&height| {
        tree_map
            .remove(&height)
            .into_iter()
            .flatten()
            .filter(|path| !recent_paths_set.contains(path))
    }));
    sort_by_height_and_lexicographical_order(&mut recent_paths);
    recent_paths
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

#[cfg(test)]
mod discovery_algorithm_tests {
    use super::*;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_get_orphaned_paths() {
        // Prepare the canonical branch
        let deep_canonical_branch: Vec<PathBuf> = vec![
            PathBuf::from("mainnet-1-a.json"),
            PathBuf::from("mainnet-2-b.json"),
            PathBuf::from("mainnet-3-c.json"),
        ];
        let canonical_refs: Vec<&PathBuf> = deep_canonical_branch.iter().collect();

        // Prepare the tree map
        let binding_1 = PathBuf::from("mainnet-2-d.json");
        let binding_2 = PathBuf::from("mainnet-3-e.json");

        let mut tree_map: HashMap<u32, Vec<&PathBuf>> = HashMap::new();
        tree_map.insert(2, vec![&deep_canonical_branch[1], &binding_1]);
        tree_map.insert(3, vec![&deep_canonical_branch[2], &binding_2]);

        // Expected orphaned paths
        let expected_orphaned_paths = vec![
            PathBuf::from("mainnet-2-d.json"),
            PathBuf::from("mainnet-3-e.json"),
        ];

        // Get orphaned paths
        let orphaned_paths = get_orphaned_paths(&canonical_refs, &mut tree_map);

        // Assert that orphaned paths match expected paths
        assert_eq!(
            orphaned_paths,
            expected_orphaned_paths.iter().collect::<Vec<&PathBuf>>()
        );
    }

    #[test]
    fn test_split_off_recent_paths() {
        // Prepare the canonical branch
        let branch_with_best_tip: Vec<PathBuf> = vec![
            PathBuf::from("mainnet-1-a.json"),
            PathBuf::from("mainnet-2-b.json"),
            PathBuf::from("mainnet-3-c.json"),
            PathBuf::from("mainnet-4-d.json"),
            PathBuf::from("mainnet-5-e.json"),
        ];
        let mut canonical_refs: Vec<&PathBuf> = branch_with_best_tip.iter().collect();

        // Prepare the tree map
        let binding_1 = PathBuf::from("mainnet-4-x.json");
        let binding_2 = PathBuf::from("mainnet-5-y.json");

        let mut tree_map: HashMap<u32, Vec<&PathBuf>> = HashMap::new();
        tree_map.insert(4, vec![&branch_with_best_tip[3], &binding_1]);
        tree_map.insert(5, vec![&branch_with_best_tip[4], &binding_2]);

        // Prepare the recent heights
        let recent_heights = vec![4, 5];

        // Expected recent paths
        let expected_recent_paths = vec![
            PathBuf::from("mainnet-4-d.json"),
            PathBuf::from("mainnet-4-x.json"),
            PathBuf::from("mainnet-5-e.json"),
            PathBuf::from("mainnet-5-y.json"),
        ];

        // Expected canonical branch after split
        let expected_canonical_branch = vec![
            PathBuf::from("mainnet-1-a.json"),
            PathBuf::from("mainnet-2-b.json"),
            PathBuf::from("mainnet-3-c.json"),
        ];

        // Get recent paths
        let recent_paths =
            split_off_recent_paths(&mut canonical_refs, &mut tree_map, recent_heights);

        // Assert that recent paths match expected paths
        assert_eq!(
            recent_paths,
            expected_recent_paths.iter().collect::<Vec<&PathBuf>>()
        );

        // Assert that canonical branch has been correctly mutated
        assert_eq!(
            canonical_refs,
            expected_canonical_branch.iter().collect::<Vec<&PathBuf>>()
        );
    }
}
