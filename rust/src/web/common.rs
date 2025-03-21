use crate::{
    block::store::BlockStore,
    store::IndexerStore,
    utility::store::common::{from_be_bytes, state_hash_suffix, U32_LEN},
};
use log::error;
use speedb::{Direction, IteratorMode};
use std::{collections::HashSet, sync::Arc};

pub fn unique_block_producers_last_n_blocks(
    db: &Arc<IndexerStore>,
    num_blocks: u32,
) -> anyhow::Result<Option<u32>> {
    const MAX_NUM_BLOCKS: u32 = 1000;

    if let Some(best_height) = db.get_best_block_height()? {
        let start_height = 1.max(best_height.saturating_sub(num_blocks.min(MAX_NUM_BLOCKS)));
        let mut producers = HashSet::new();

        for (key, _) in db
            .blocks_height_iterator(IteratorMode::From(
                &(best_height + 1).to_be_bytes(),
                Direction::Reverse,
            ))
            .flatten()
        {
            let height = from_be_bytes(key[..U32_LEN].to_vec());
            if height <= start_height {
                break;
            }

            let state_hash = state_hash_suffix(&key)?;

            if let Some(creator) = db.get_block_creator(&state_hash)? {
                producers.insert(creator);
                continue;
            }

            error!("Block creator index missing (length {height}) {state_hash}")
        }

        return Ok(Some(producers.len() as u32));
    }

    Ok(None)
}
