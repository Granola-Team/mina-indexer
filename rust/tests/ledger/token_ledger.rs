use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    constants::*,
    ledger::{
        account::{Account, Permission, Permissions},
        genesis::{GenesisAccounts, GenesisLedger, GenesisRoot, GenesisTimestamp},
        token::TokenAddress,
    },
    mina_blocks::v2::{VerificationKey, ZkappAccount},
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, str::FromStr, sync::Arc};

#[ignore = "passing in tier 1, failing in tier 2"]
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
        let expect = Account {
            public_key: pk.clone(),
            balance: MAINNET_ACCOUNT_CREATION_FEE,
            nonce: Some(1.into()),
            delegate: pk,
            genesis_account: false,
            token: Some(mina_token.clone()),
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
        };

        assert_eq!(*mina_account, expect);
        assert_eq!(
            mina_account.clone().display(),
            Account {
                balance: 0.into(),
                ..expect
            }
        );
    } else {
        panic!("MINA zkapp account does not exist");
    }

    // check B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P's MINA account
    let pk = "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P".into();
    if let Some(mina_account) = best_ledger.get_account(&pk, &mina_token) {
        let expect = Account {
                public_key: pk.clone(),
                balance: (1e9 as u64).into(),
                nonce: Some(1.into()),
                delegate: pk,
                token: Some(mina_token),
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
                zkapp: Some(ZkappAccount {
                    app_state: [
                        "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    ],
                    verification_key: VerificationKey {
                        data: "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".into(),
                        hash: "0x0F2B2203687484CD2435CF65411C6C0300F7A55A49994517DFF013DF9A2E1659".into(),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            };

        assert_eq!(*mina_account, expect);
        assert_eq!(
            mina_account.clone().display(),
            Account {
                balance: 0.into(),
                ..expect
            }
        );
    } else {
        panic!(
            "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P zkapp account does not exist"
        );
    }

    Ok(())
}
