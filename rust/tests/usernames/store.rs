use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
    },
    constants::*,
    server::IndexerVersion,
    store::{username::UsernameStore, DbUpdate},
};
use std::path::PathBuf;

#[tokio::test]
async fn set_usernames() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("usernames-db")?;
    let block_dir = &PathBuf::from("./tests/data/non_sequential_blocks");

    let mut state = mainnet_genesis_state(store_dir.as_ref())?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        block_dir,
        IndexerVersion::default().version,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // update usernames
    let block = PrecomputedBlock::parse_file(&PathBuf::from("./tests/data/non_sequential_blocks/mainnet-338728-3NLe2WXRaJq85Ldj1ycEQRa2R6vmemVAoXpvkncccuuKNuWs6WYf.json"), PcbVersion::V1)?;
    store.update_usernames(DbUpdate {
        apply: vec![block.username_updates()],
        ..Default::default()
    })?;

    assert_eq!(
        "Betelgeuse",
        store
            .get_username(&"B62qkEtH1PxqjJPKitAmzfV2ozCuCcibBL4tLgpeXHvsaqVgrENjFhX".into())?
            .unwrap()
            .0
    );
    Ok(())
}
