use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    constants::*,
    ledger::{
        account::{Account, Permission, Permissions},
        genesis::{GenesisAccounts, GenesisLedger, GenesisRoot, GenesisTimestamp},
        token::TokenAddress,
    },
    mina_blocks::v2::ZkappAccount,
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, str::FromStr, sync::Arc};

#[tokio::test]
async fn check_token_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("token-ledger")?;
    let blocks_dir = &PathBuf::from("./tests/data/hardfork");
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);

    // start with an empty ledger
    let genesis_ledger = GenesisRoot {
        genesis: GenesisTimestamp {
            genesis_state_timestamp: "2024-06-05T00:00:00Z".to_string(),
        },
        ledger: GenesisAccounts {
            name: "mainnet_v2".to_string(),
            accounts: vec![],
        },
    };
    let genesis_ledger: GenesisLedger = genesis_ledger.into();

    let mut state = IndexerState::new(
        genesis_ledger,
        IndexerVersion::v2(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
        false,
    )?;

    let mut bp = BlockParser::new_testing(blocks_dir)?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    println!("{}", state);

    let best_ledger = state.best_ledger();
    let pk = "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into();
    let minu_token = TokenAddress::from_str("wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd")?;
    let mina_token = TokenAddress::default();

    // full ledger contains both token ledgers
    let mut tracked_tokens = best_ledger.tokens.keys().cloned().collect::<Vec<_>>();
    tracked_tokens.sort();

    assert_eq!(tracked_tokens, vec![mina_token.clone(), minu_token.clone()]);

    // check MINA token ledger
    assert_eq!(best_ledger.get_token_ledger(&mina_token).unwrap().len(), 78);

    // check MINU token ledger/account
    assert_eq!(best_ledger.get_token_ledger(&minu_token).unwrap().len(), 1);

    if let Some(minu_account) = best_ledger.get_account(&pk, &minu_token) {
        assert_eq!(
            *minu_account,
            Account {
                balance: 100000000000000.into(),
                public_key: pk.clone(),
                nonce: None,
                delegate: pk.clone(),
                genesis_account: false,
                token: Some(minu_token),
                token_symbol: None,
                receipt_chain_hash: None,
                voting_for: None,
                permissions: None,
                timing: None,
                zkapp: None,
                username: None,
            }
        );
    } else {
        panic!("MINU account does not exist");
    }

    // check MINA account
    if let Some(mina_account) = best_ledger.get_account(&pk, &mina_token) {
        assert_eq!(
            *mina_account,
            Account {
                public_key: pk.clone(),
                balance: 0.into(),
                delegate: pk,
                genesis_account: false,
                token: Some(mina_token),
                token_symbol: Some("MINU".into()),
                receipt_chain_hash: None,
                voting_for: None,
                permissions: Some(Permissions {
                    edit_state: Permission::Proof,
                    access: Permission::None,
                    send: Permission::Proof,
                    receive: Permission::None,
                    set_delegate: Permission::Signature,
                    set_permissions: Permission::Signature,
                    set_verification_key: (Permission::Signature, "3".into()),
                    set_zkapp_uri: Permission::Signature,
                    edit_action_state: Permission::Proof,
                    set_token_symbol: Permission::Signature,
                    increment_nonce: Permission::Signature,
                    set_voting_for: Permission::Signature,
                    set_timing: Permission::Signature,
                }),
                timing: None,
                zkapp: Some(ZkappAccount::default()),
                username: None,

                // TODO should be 1
                nonce: Some(4.into()),
            }
        )
    } else {
        panic!("MINA account does not exist");
    }

    Ok(())
}
