use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::parser::BlockParser,
    ledger::{account::Account, store::best::BestLedgerStore},
};
use std::path::PathBuf;

#[tokio::test]
async fn zkapp_best_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("zkapp-best-ledger-accounts")?;
    let block_dir = PathBuf::from("./tests/data/hardfork");

    let mut state = hardfork_genesis_state(store_dir.as_ref())?;
    let mut bp = BlockParser::new_testing(&block_dir)?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // check zkapp accounts
    let mut zkapp_accounts = vec![];

    for (_, value) in store
        .zkapp_best_ledger_account_balance_iterator(speedb::IteratorMode::End)
        .flatten()
    {
        let account: Account = serde_json::from_slice(&value)?;
        assert!(account.is_zkapp_account());

        zkapp_accounts.push((
            account.public_key.to_string(),
            account.token.unwrap().to_string(),
        ));
    }

    assert_eq!(
        zkapp_accounts,
        vec![
            (
                "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P".to_string(),
                "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf".to_string(),
            ),
            (
                "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".to_string(),
                "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf".to_string(),
            )
        ]
    );

    Ok(())
}
