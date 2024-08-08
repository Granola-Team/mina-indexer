use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
    },
    constants::*,
    ledger::genesis::{GenesisLedger, GenesisRoot},
    server::IndexerVersion,
    state::IndexerState,
    store::{username::UsernameStore, DBUpdate, IndexerStore},
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn set_usernames() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("usernames-db")?;
    let blocks_dir = &PathBuf::from("./tests/data/non_sequential_blocks");
    let store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        IndexerVersion::default().version,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;
    let mut state = IndexerState::new(
        genesis_ledger.clone().into(),
        IndexerVersion::default(),
        store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
    )?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    // update usernames
    let block = PrecomputedBlock::parse_file(&PathBuf::from("./tests/data/non_sequential_blocks/mainnet-338728-3NLe2WXRaJq85Ldj1ycEQRa2R6vmemVAoXpvkncccuuKNuWs6WYf.json"), PcbVersion::V1)?;
    store.update_usernames(DBUpdate {
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
