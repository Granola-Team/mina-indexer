use crate::{
    base::state_hash::StateHash,
    block::{
        extract_block_height, extract_height_and_hash, extract_state_hash,
        genesis_state_hash::GenesisStateHash, previous_state_hash::*,
        sort_by_height_and_lexicographical_order,
    },
    utility::functions::pretty_print_duration,
};
use log::info;
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    path::PathBuf,
    time::Instant,
};

/// Discovers the canonical chain, orphaned blocks, and
/// recent blocks within the canonical threshold
pub fn discovery(
    genesis_state_hash: &StateHash,
    canonical_threshold: u32,
    reporting_freq: u32,
    paths: Vec<&PathBuf>,
) -> anyhow::Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>)> {
    if paths.is_empty() {
        return Ok((vec![], vec![], vec![]));
    }

    let time = Instant::now();
    let mut tree_map: BTreeMap<u32, Vec<&PathBuf>> = BTreeMap::new();
    let mut parent_hash_map: HashMap<&str, &str> = HashMap::new();

    // store block paths by height
    for path in paths.iter() {
        let height = extract_block_height(path);
        tree_map.entry(height).or_default().push(path);
    }

    // find the best tip
    let best_tip = find_best_tip(
        genesis_state_hash,
        &tree_map,
        &mut parent_hash_map,
        reporting_freq,
    )
    .to_owned();

    // best tip found
    if let Some(best_tip) = best_tip {
        // walk back from tip to root of tree
        let mut canonical_branch =
            canonical_branch_from_best_tip(&mut tree_map, &parent_hash_map, best_tip)?;

        // split off recent paths from canonical branch and tree map
        let recent_paths =
            split_off_recent_paths(&mut canonical_branch, &mut tree_map, canonical_threshold);

        // all other paths in the tree map are orphaned
        let orphaned_paths = get_orphaned_paths(&mut tree_map);

        assert!(tree_map.is_empty(), "Not all paths have been discovered");
        info!(
            "Found {} blocks in the canonical chain in {}",
            1 + canonical_branch.len() as u32 + canonical_threshold,
            pretty_print_duration(time.elapsed())
        );

        return Ok((
            canonical_branch.into_iter().cloned().collect::<Vec<_>>(),
            recent_paths.into_iter().cloned().collect::<Vec<_>>(),
            orphaned_paths.into_iter().cloned().collect::<Vec<_>>(),
        ));
    }

    // no best tip found
    info!("No best best tip found. Ingesting all blocks in length-sorted order.");
    Ok((
        vec![],
        paths.into_iter().cloned().collect::<Vec<_>>(),
        vec![],
    ))
}

fn find_best_tip<'a>(
    genesis_state_hash: &StateHash,
    tree_map: &BTreeMap<u32, Vec<&'a PathBuf>>,
    parent_hash_map: &mut HashMap<&'a str, &'a str>,
    reporting_freq: u32,
) -> Option<&'a PathBuf> {
    let time = Instant::now();
    let mut queue = VecDeque::new();
    let mut best_tip = None;

    // check if any hardfork blocks at highest height
    if let Some((_, blocks_at_height)) = tree_map.first_key_value() {
        for path in blocks_at_height.iter() {
            if *genesis_state_hash == GenesisStateHash::from_path(path).unwrap() {
                best_tip = Some(path);
                queue.push_back(path);
            }
        }
    }

    while let Some(best_tip_candidate) = queue.pop_front() {
        let (height, state_hash) = extract_height_and_hash(best_tip_candidate);
        let next_height = height + 1;

        log_progress(height, reporting_freq, &time);

        if let Some(next_tips) = tree_map.get(&next_height) {
            for possible_next_tip in next_tips {
                if let Ok(prev_hash) = PreviousStateHash::from_path(possible_next_tip) {
                    if prev_hash.0 == state_hash {
                        parent_hash_map.insert(extract_state_hash(possible_next_tip), state_hash);

                        best_tip = Some(possible_next_tip);
                        queue.push_back(possible_next_tip);
                    }
                }
            }
        }
    }

    best_tip.map(|best_tip| {
        info!(
            "Found best tip {} in {}",
            best_tip.file_stem().unwrap().to_str().unwrap(),
            pretty_print_duration(time.elapsed())
        );

        *best_tip
    })
}

fn canonical_branch_from_best_tip<'a>(
    tree_map: &mut BTreeMap<u32, Vec<&'a PathBuf>>,
    parent_hash_map: &HashMap<&'a str, &'a str>,
    best_tip: &'a PathBuf,
) -> anyhow::Result<Vec<&'a PathBuf>> {
    let time = Instant::now();
    let mut canonical_branch = vec![];
    canonical_branch.push(best_tip);

    // next iteration
    let (mut next_height, state_hash) = extract_height_and_hash(best_tip);
    let mut opt_parent_state_hash = parent_hash_map.get(state_hash);

    while let Some(parent_state_hash) = opt_parent_state_hash {
        next_height -= 1;

        let paths = tree_map.get_mut(&next_height).unwrap();
        let mut i = None;

        for (j, path) in paths.iter().enumerate() {
            let state_hash = extract_state_hash(path);

            if state_hash == *parent_state_hash {
                opt_parent_state_hash = parent_hash_map.get(state_hash);
                canonical_branch.push(path);
                i = Some(j);

                break;
            }
        }

        if let Some(i) = i {
            paths.remove(i);
        }
    }

    info!(
        "Found canonical chain in {}",
        pretty_print_duration(time.elapsed())
    );

    // Reverse to maintain order
    canonical_branch.reverse();

    Ok(canonical_branch)
}

fn get_orphaned_paths<'a>(tree_map: &mut BTreeMap<u32, Vec<&'a PathBuf>>) -> Vec<&'a PathBuf> {
    let time = Instant::now();
    let mut orphaned_paths: Vec<&PathBuf> = vec![];

    while let Some((_height, paths)) = tree_map.pop_first() {
        for path in paths {
            orphaned_paths.push(path);
        }
    }

    info!(
        "Found {} orphaned blocks in {}",
        orphaned_paths.len(),
        pretty_print_duration(time.elapsed())
    );

    orphaned_paths
}

fn split_off_recent_paths<'a>(
    canonical_branch: &mut Vec<&'a PathBuf>,
    tree_map: &mut BTreeMap<u32, Vec<&'a PathBuf>>,
    canonical_threshold: u32,
) -> Vec<&'a PathBuf> {
    let time = Instant::now();
    let split_index = canonical_branch
        .len()
        .saturating_sub(canonical_threshold as usize);
    let split_height = canonical_branch
        .get(split_index)
        .map(|p| extract_block_height(p))
        .unwrap_or_default();

    let mut recent_paths = canonical_branch.split_off(split_index);
    let mut recent_tree_map = tree_map.split_off(&split_height);
    let recent_paths_set: HashSet<&PathBuf> = recent_paths.clone().into_iter().collect();

    while let Some((_height, paths)) = recent_tree_map.pop_first() {
        for path in paths {
            if !recent_paths_set.contains(path) {
                recent_paths.push(path);
            }
        }
    }

    sort_by_height_and_lexicographical_order(&mut recent_paths);

    info!(
        "Found {} recent ({canonical_threshold} canonical) blocks in {}",
        recent_paths.len(),
        pretty_print_duration(time.elapsed())
    );

    recent_paths
}

fn log_progress(length_of_chain: u32, reporting_freq: u32, time: &Instant) {
    if length_of_chain % reporting_freq == 0 {
        info!(
            "Found best tip candidate at height {} in {}",
            length_of_chain,
            pretty_print_duration(time.elapsed())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, path::PathBuf};

    #[test]
    fn test_canonical_branch_from_best_tip() {
        // Prepare the best tip
        let best_tip = PathBuf::from("tests/data/sequential_blocks/mainnet-105500-3NK73T6brdpBFgjbZKMpfYX596q68sfHx8NtMDYRLJ9ai88WzrKQ.json");

        // Prepare the tree map with blocks that form the canonical branch and extra
        // entries
        let path_499_canon = PathBuf::from("tests/data/sequential_blocks/mainnet-105499-3NKEkf29fm6CARN6MAi6ZvmADxEXpu1wUwYfnjsiWCmR5LfCpwSg.json");
        let path_499_extra = PathBuf::from("tests/data/sequential_blocks/mainnet-105499-3NLmMoYPiS3oc6Vj3etc5xQd5Ny9cjcKCadqRqxeEHSRF5icw3es.json");
        let path_498_canon = PathBuf::from("tests/data/sequential_blocks/mainnet-105498-3NKbLiBHzQrAimK7AkP8qAfQpHnezkdsSm8mkt2TzsbjsLN8Axmt.json");
        let path_498_extra = PathBuf::from("tests/data/sequential_blocks/mainnet-105498-3NLmgdEg4HdPNzPNceezVrbahnW3yV2Wo6C8g49AddYUNnHBmd44.json");
        let path_497_canon = PathBuf::from("tests/data/sequential_blocks/mainnet-105497-3NKjngJTXJzRUXF3uH2nK19iYUVtYBFjLhezSrMMFVQyEGwqEi3c.json");
        let path_497_extra = PathBuf::from("tests/data/sequential_blocks/mainnet-105497-3NLpfuGk5gvgaQuSQ3WrhXLX9mNJRZ1cNbRUAfCqdLqvVRjj4mL4.json");

        let mut tree_map: BTreeMap<u32, Vec<&PathBuf>> = BTreeMap::new();
        tree_map.insert(105499, vec![&path_499_canon, &path_499_extra]);
        tree_map.insert(105498, vec![&path_498_canon, &path_498_extra]);
        tree_map.insert(105497, vec![&path_497_canon, &path_497_extra]);

        // Prepare the parent hash map
        let mut parent_hash_map: HashMap<&str, &str> = HashMap::new();
        parent_hash_map.insert(
            "3NK73T6brdpBFgjbZKMpfYX596q68sfHx8NtMDYRLJ9ai88WzrKQ",
            "3NKEkf29fm6CARN6MAi6ZvmADxEXpu1wUwYfnjsiWCmR5LfCpwSg",
        );
        parent_hash_map.insert(
            "3NKEkf29fm6CARN6MAi6ZvmADxEXpu1wUwYfnjsiWCmR5LfCpwSg",
            "3NKbLiBHzQrAimK7AkP8qAfQpHnezkdsSm8mkt2TzsbjsLN8Axmt",
        );
        parent_hash_map.insert(
            "3NKbLiBHzQrAimK7AkP8qAfQpHnezkdsSm8mkt2TzsbjsLN8Axmt",
            "3NKjngJTXJzRUXF3uH2nK19iYUVtYBFjLhezSrMMFVQyEGwqEi3c",
        );

        // Expected canonical branch
        let binding = best_tip.clone();
        let expected_canonical_branch =
            vec![&path_497_canon, &path_498_canon, &path_499_canon, &binding];

        // Run the function
        let canonical_branch =
            canonical_branch_from_best_tip(&mut tree_map, &parent_hash_map, &best_tip).unwrap();

        // Assert that the result matches the expected canonical branch
        assert_eq!(canonical_branch, expected_canonical_branch);

        // Assert that only extra entries remain in the tree map
        assert_eq!(tree_map.get(&105499).unwrap(), &vec![&path_499_extra]);
        assert_eq!(tree_map.get(&105498).unwrap(), &vec![&path_498_extra]);
        assert_eq!(tree_map.get(&105497).unwrap(), &vec![&path_497_extra]);
        assert_eq!(tree_map.len(), 3);
    }

    #[test]
    fn test_get_orphaned_paths() {
        // Prepare the tree map
        let binding_1 = PathBuf::from("mainnet-2-d.json");
        let binding_2 = PathBuf::from("mainnet-3-e.json");

        let mut tree_map: BTreeMap<u32, Vec<&PathBuf>> = BTreeMap::new();
        tree_map.insert(0, vec![&binding_1]);
        tree_map.insert(1, vec![&binding_2]);

        // Expected orphaned paths
        let expected_orphaned_paths = [
            PathBuf::from("mainnet-2-d.json"),
            PathBuf::from("mainnet-3-e.json"),
        ];

        // Get orphaned paths
        let orphaned_paths = get_orphaned_paths(&mut tree_map);

        // Assert that orphaned paths match expected paths
        assert_eq!(
            orphaned_paths,
            expected_orphaned_paths.iter().collect::<Vec<&PathBuf>>()
        );

        assert!(tree_map.is_empty());
    }

    #[test]
    fn test_split_off_recent_paths() {
        let canonical_threshold = 2;

        // Prepare the canonical branch
        let branch_with_best_tip: Vec<PathBuf> = vec![
            PathBuf::from("mainnet-1-a.json"),
            PathBuf::from("mainnet-2-b.json"),
            PathBuf::from("mainnet-3-c.json"),
            PathBuf::from("mainnet-4-d.json"),
            PathBuf::from("mainnet-5-e.json"), // goes to height 5 but not further
        ];
        let mut canonical_refs: Vec<&PathBuf> = branch_with_best_tip.iter().collect();

        // Prepare the tree map
        let binding_a = PathBuf::from("mainnet-1-a.json");
        let binding_b = PathBuf::from("mainnet-2-b.json");
        let binding_c = PathBuf::from("mainnet-3-c.json");
        let binding_1 = PathBuf::from("mainnet-4-x.json");
        let binding_2 = PathBuf::from("mainnet-5-y.json");
        let binding_3 = PathBuf::from("mainnet-6-z.json"); // has not parent

        let mut tree_map: BTreeMap<u32, Vec<&PathBuf>> = BTreeMap::new();
        tree_map.insert(1, vec![&binding_a]);
        tree_map.insert(2, vec![&binding_b]);
        tree_map.insert(3, vec![&binding_c]);
        tree_map.insert(4, vec![&branch_with_best_tip[3], &binding_1]);
        tree_map.insert(5, vec![&branch_with_best_tip[4], &binding_2]);
        tree_map.insert(6, vec![&binding_3]);

        // Expected recent paths
        let expected_recent_paths = [
            PathBuf::from("mainnet-4-d.json"), // in canonical chain
            PathBuf::from("mainnet-4-x.json"), // recent, but not canonical
            PathBuf::from("mainnet-5-e.json"), // best tip
            PathBuf::from("mainnet-5-y.json"), // recent, but not canonical
            PathBuf::from("mainnet-6-z.json"),
        ];

        // Expected canonical branch after split
        let expected_canonical_branch = [
            PathBuf::from("mainnet-1-a.json"),
            PathBuf::from("mainnet-2-b.json"),
            PathBuf::from("mainnet-3-c.json"),
        ];

        // Get recent paths
        let recent_paths =
            split_off_recent_paths(&mut canonical_refs, &mut tree_map, canonical_threshold);

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

        // Assert that tree_map has been correctly mutated
        assert_eq!(tree_map.get(&1), Some(&vec![&binding_a]));
        assert_eq!(tree_map.get(&2), Some(&vec![&binding_b]));
        assert_eq!(tree_map.get(&3), Some(&vec![&binding_c]));
        assert_eq!(tree_map.len(), 3);
    }
}
