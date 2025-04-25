use crate::helpers::{state::*, store::*};
use anyhow::Context;
use mina_indexer::{
    block::parser::BlockParser,
    ledger::{
        account::{Account, Permission, Permissions},
        store::best::BestLedgerStore,
        token::TokenAddress,
    },
    mina_blocks::v2::{zkapp::app_state::ZkappState, VerificationKey, ZkappAccount},
};
use std::{path::PathBuf, str::FromStr};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn check_token_accounts() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("token-ledger")?;
    let blocks_dir = &PathBuf::from("./tests/data/hardfork");

    // start with the hardfork genesis ledger
    let mut state = hardfork_genesis_state(store_dir.path())?;
    let mut bp = BlockParser::new_testing(blocks_dir)?;

    // ingest the blocks
    state.add_blocks(&mut bp).await?;

    let indexer_store = state.indexer_store.as_ref().unwrap();
    let best_ledger = state.best_ledger();

    let pk = "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into();
    let minu_token = TokenAddress::from_str("wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd")?;
    let mina_token = TokenAddress::default();

    // full ledger contains both token ledgers
    let mut tracked_tokens = best_ledger.tokens.keys().cloned().collect::<Vec<_>>();
    tracked_tokens.sort();

    assert_eq!(tracked_tokens, vec![mina_token.clone(), minu_token.clone()]);

    // check all token accounts
    assert_eq!(best_ledger.len(), 228180);
    assert_eq!(
        best_ledger.len(),
        indexer_store
            .best_ledger_account_balance_iterator(speedb::IteratorMode::End)
            .count()
    );

    assert_eq!(
        best_ledger.len(),
        indexer_store.get_num_accounts()?.unwrap() as usize
    );

    let num_best_zkapps = {
        let mut num = 0;

        for token_ledger in best_ledger.tokens.values() {
            for account in token_ledger.accounts.values() {
                if account.is_zkapp_account() {
                    num += 1;
                }
            }
        }

        num
    };

    // check zkapp accounts
    assert_eq!(
        num_best_zkapps,
        indexer_store
            .zkapp_best_ledger_account_balance_iterator(speedb::IteratorMode::End)
            .count(),
    );

    assert_eq!(
        num_best_zkapps,
        indexer_store.get_num_zkapp_accounts()?.unwrap() as usize
    );

    // check MINA token ledger
    assert_eq!(
        best_ledger.get_token_ledger(&mina_token).unwrap().len(),
        indexer_store.get_num_mina_accounts()?.unwrap() as usize
    );

    // check MINU token ledger/account
    assert_eq!(best_ledger.get_token_ledger(&minu_token).unwrap().len(), 1);

    // check MINU account balance
    if let Some(minu_account) = best_ledger.get_account(&pk, &minu_token) {
        assert_eq!(
            *minu_account,
            Account {
                balance: 100000000000000.into(),
                public_key: pk.clone(),
                delegate: pk.clone(),
                token: Some(minu_token.clone()),
                creation_fee_paid: true,
                ..Default::default()
            }
        );
    } else {
        panic!("MINU account does not exist");
    }

    // check MINA account is a zkapp
    if let Some(mina_account) = best_ledger.get_account(&pk, &mina_token) {
        let expect = Account {
            public_key: pk.clone(),
            balance: 0.into(),
            nonce: Some(1.into()),
            delegate: pk,
            token: Some(mina_token.clone()),
            token_symbol: Some("MINU".into()),
            creation_fee_paid: true,
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
                proved_state: true,
                verification_key: VerificationKey {
                    data: "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".into(),
                    hash: "0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".into()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(*mina_account, expect);
        assert_eq!(
            mina_account.clone().deduct_mina_account_creation_fee(),
            expect
        );
    } else {
        panic!("MINA zkapp account does not exist");
    }

    // check B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P's MINA account
    let pk = "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P".into();
    if let Some(mina_account) = best_ledger.get_account(&pk, &mina_token) {
        let expect = Account {
                public_key: pk.clone(),
                balance: 0.into(),
                nonce: Some(1.into()),
                delegate: pk,
                token: Some(mina_token.clone()),
                creation_fee_paid: true,
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
                    app_state: ZkappState([
                        "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default()
                    ]),
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
            mina_account.clone().deduct_mina_account_creation_fee(),
            expect
        );
    } else {
        panic!(
            "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P zkapp account does not exist"
        );
    }

    // check best ledger store against state best ledger
    for (token, token_ledger) in best_ledger.tokens.iter() {
        for (pk, state_account) in token_ledger.accounts.iter() {
            let store_best_account = {
                use mina_indexer::ledger::store::best::BestLedgerStore;

                indexer_store
                    .get_best_account(pk, token)?
                    .context(format!("missing best token account ({pk}, {token})"))
                    .unwrap()
            };

            assert_eq!(store_best_account, *state_account);
        }
    }

    Ok(())
}
