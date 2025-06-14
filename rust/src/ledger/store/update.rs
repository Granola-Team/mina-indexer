//! Update account in ledger store

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    ledger::{
        account::Account,
        diff::{
            account::{zkapp::ZkappPaymentDiff, AccountDiff},
            token::TokenDiff,
        },
        token::{account::TokenAccount, Token, TokenAddress},
    },
    store::{
        zkapp::{
            actions::ZkappActionStore, events::ZkappEventStore, tokens::ZkappTokenStore, ZkappStore,
        },
        DbUpdate, IndexerStore, Result,
    },
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub account_diffs: Vec<AccountDiff>,
    pub token_diffs: Vec<TokenDiff>,
    pub new_accounts: HashSet<(PublicKey, TokenAddress)>,
    pub new_zkapp_accounts: HashSet<(PublicKey, TokenAddress)>,
}

pub type DbAccountUpdate = DbUpdate<AccountUpdate>;

impl DbAccountUpdate {
    pub fn new(apply: Vec<AccountUpdate>, unapply: Vec<AccountUpdate>) -> Self {
        Self { apply, unapply }
    }

    pub fn apply_updates(
        db: &IndexerStore,
        apply: Vec<AccountUpdate>,
        state_hash: &StateHash,
        block_height: u32,
    ) -> Result<()> {
        for AccountUpdate {
            account_diffs,
            token_diffs,
            new_accounts,
            ..
        } in apply.into_iter()
        {
            let token_account_diffs = aggregate_token_account_diffs(account_diffs);

            // apply account diffs
            for ((pk, token), diffs) in token_account_diffs {
                let before = db.get_best_account(&pk, &token)?;
                let (before_values, mut after) = (
                    before.as_ref().map(|a| (a.is_zkapp_account(), a.balance.0)),
                    before.unwrap_or_else(|| {
                        Account::empty(
                            pk.clone(),
                            token.clone(),
                            diffs.iter().any(|diff| diff.creation_fee_paid()),
                        )
                    }),
                );

                for diff in diffs.iter() {
                    use AccountDiff::*;

                    after = match diff {
                        Payment(diff)
                        | FeeTransfer(diff)
                        | FeeTransferViaCoinbase(diff)
                        | ZkappPayment(ZkappPaymentDiff::Payment { payment: diff, .. }) => {
                            after.payment(diff)
                        }
                        Coinbase(diff) => after.coinbase(diff.amount),
                        Delegation(diff) => after.delegation(diff.delegate.clone(), diff.nonce),
                        FailedTransactionNonce(diff) => after.failed_transaction(diff.nonce),

                        // zkapp diffs
                        ZkappPayment(ZkappPaymentDiff::IncrementNonce(diff))
                        | ZkappIncrementNonce(diff) => after.zkapp_nonce(diff, state_hash),
                        ZkappFeePayerNonce(diff) => after.zkapp_fee_payer_nonce(diff, state_hash),
                        ZkappState(diff) => {
                            let after = after.zkapp_state(diff, state_hash);
                            db.add_zkapp_state(
                                &diff.token,
                                &diff.public_key,
                                &after.zkapp.as_ref().expect("zkapp").app_state,
                            )?;
                            after
                        }
                        ZkappPermissions(diff) => {
                            db.add_zkapp_permissions(
                                &diff.token,
                                &diff.public_key,
                                &diff.permissions,
                            )?;
                            after.zkapp_permissions(diff, state_hash)
                        }
                        ZkappVerificationKey(diff) => {
                            db.add_zkapp_verification_key(
                                &diff.token,
                                &diff.public_key,
                                &diff.verification_key,
                            )?;
                            after.zkapp_verification_key(diff, state_hash)
                        }
                        ZkappUri(diff) => {
                            db.add_zkapp_uri(&diff.token, &diff.public_key, &diff.zkapp_uri)?;
                            after.zkapp_uri(diff, state_hash)
                        }
                        ZkappTokenSymbol(diff) => {
                            db.add_zkapp_token_symbol(
                                &diff.token,
                                &diff.public_key,
                                &diff.token_symbol,
                            )?;
                            after.zkapp_token_symbol(diff, state_hash)
                        }
                        ZkappTiming(diff) => {
                            db.add_zkapp_timing(&diff.token, &diff.public_key, &diff.timing)?;
                            after.zkapp_timing(diff, state_hash)
                        }
                        ZkappVotingFor(diff) => after.zkapp_voting_for(diff, state_hash),
                        ZkappProvedState(diff) => after.zkapp_proved_state(diff, state_hash),

                        // these diffs do not modify the account
                        ZkappActions(diff) => {
                            db.add_actions(
                                &diff.public_key,
                                &diff.token,
                                &diff.actions,
                                state_hash,
                                block_height,
                                &diff.txn_hash,
                            )?;

                            after
                        }
                        ZkappEvents(diff) => {
                            db.add_events(
                                &diff.public_key,
                                &diff.token,
                                &diff.events,
                                state_hash,
                                block_height,
                                &diff.txn_hash,
                            )?;

                            after
                        }
                        // zkapp account diffs should be expanded
                        Zkapp(_) => unreachable!(),
                    };
                }

                // update staged ledger account
                db.set_staged_account(&pk, &token, state_hash, block_height, &after)?;

                db.update_best_account(
                    &pk,
                    &token,
                    before_values,
                    Some(after),
                    new_accounts.contains(&(pk.clone(), token.clone())),
                )?;
            }

            // apply token diffs
            for diffs in aggregate_token_diffs(token_diffs).values() {
                if !diffs.is_empty() {
                    db.apply_best_token_diffs(state_hash, diffs)?;
                }
            }
        }

        // adjust MINA token supply
        if let Some(supply) = db.get_block_total_currency(state_hash)? {
            db.set_token(&Token::mina_with_supply(supply))?;
        }

        Ok(())
    }

    pub fn unapply_updates(
        db: &IndexerStore,
        unapply: Vec<AccountUpdate>,
        state_hash: &StateHash,
        block_height: u32,
    ) -> Result<()> {
        // unapply account & token diffs, remove accounts
        for AccountUpdate {
            account_diffs,
            token_diffs,
            new_accounts,
            ..
        } in unapply
        {
            let token_account_diffs = aggregate_token_account_diffs(account_diffs);

            for ((pk, token), diffs) in token_account_diffs {
                let before = db.get_best_account(&pk, &token)?;
                let (before_values, mut after) = (
                    before.as_ref().map(|a| (a.is_zkapp_account(), a.balance.0)),
                    before.expect("account to unapply"),
                );

                for diff in diffs.iter() {
                    use AccountDiff::*;

                    after = match diff {
                        Payment(diff)
                        | FeeTransfer(diff)
                        | FeeTransferViaCoinbase(diff)
                        | ZkappPayment(ZkappPaymentDiff::Payment { payment: diff, .. }) => {
                            after.payment_unapply(diff)
                        }
                        Coinbase(diff) => after.coinbase_unapply(diff),
                        Delegation(diff) => {
                            db.remove_pk_delegate(pk.clone())?;
                            after.delegation_unapply(diff)
                        }
                        FailedTransactionNonce(diff) => after.failed_transaction_unapply(diff),

                        // zkapp diffs
                        ZkappPayment(ZkappPaymentDiff::IncrementNonce(_))
                        | ZkappIncrementNonce(_) => after.zkapp_nonce_unapply(),
                        ZkappFeePayerNonce(diff) => after.zkapp_fee_payer_nonce_unapply(diff),
                        ZkappState(diff) => {
                            let zkapp_state = db
                                .remove_last_zkapp_state(&diff.token, &diff.public_key)
                                .ok();

                            if let Some(app_state) = zkapp_state {
                                let mut zkapp = after.zkapp.expect("zkapp");
                                zkapp.app_state = app_state;

                                Account {
                                    zkapp: Some(zkapp),
                                    ..after
                                }
                            } else {
                                Account {
                                    zkapp: None,
                                    ..after
                                }
                            }
                        }
                        ZkappPermissions(diff) => {
                            let permissions = db
                                .remove_last_zkapp_permissions(&diff.token, &diff.public_key)
                                .ok();

                            Account {
                                permissions,
                                ..after
                            }
                        }
                        ZkappVerificationKey(diff) => {
                            let vk = db
                                .remove_last_zkapp_verification_key(&diff.token, &diff.public_key)
                                .ok();

                            if let Some(vk) = vk {
                                let mut zkapp = after.zkapp.expect("zkapp");
                                zkapp.verification_key = vk;

                                Account {
                                    zkapp: Some(zkapp),
                                    ..after
                                }
                            } else {
                                Account {
                                    zkapp: None,
                                    ..after
                                }
                            }
                        }
                        ZkappUri(diff) => {
                            let zkapp_uri =
                                db.remove_last_zkapp_uri(&diff.token, &diff.public_key).ok();

                            if let Some(zkapp_uri) = zkapp_uri {
                                let mut zkapp = after.zkapp.expect("zkapp");
                                zkapp.zkapp_uri = zkapp_uri;

                                Account {
                                    zkapp: Some(zkapp),
                                    ..after
                                }
                            } else {
                                Account {
                                    zkapp: None,
                                    ..after
                                }
                            }
                        }
                        ZkappTokenSymbol(diff) => {
                            let token_symbol = db
                                .remove_last_zkapp_token_symbol(&diff.token, &diff.public_key)
                                .ok();

                            Account {
                                token_symbol,
                                ..after
                            }
                        }
                        ZkappTiming(diff) => {
                            let timing = db
                                .remove_last_zkapp_timing(&diff.token, &diff.public_key)
                                .ok();

                            Account { timing, ..after }
                        }
                        ZkappActions(diff) => {
                            db.remove_actions(&pk, &token, diff.actions.len() as u32)?;
                            after
                        }
                        ZkappEvents(diff) => {
                            db.remove_events(&pk, &token, diff.events.len() as u32)?;
                            after
                        }
                        ZkappProvedState(_) | ZkappVotingFor(_) => after,

                        // zkapp diffs should be expanded by this point
                        Zkapp(_) => unreachable!(),
                    };
                }

                if new_accounts.contains(&(pk.clone(), token.clone())) {
                    db.remove_staged_account(
                        &pk,
                        &token,
                        state_hash,
                        block_height,
                        after.balance.0,
                    )?;
                }

                db.update_best_account(&pk, &token, before_values, Some(after), false)?;
            }

            // unapply token diffs
            for diffs in aggregate_token_diffs(token_diffs).values() {
                if !diffs.is_empty() {
                    db.unapply_best_token_diffs(diffs)?;
                }
            }

            // remove accounts
            for (pk, token) in new_accounts.iter() {
                db.update_best_account(pk, token, None, None, false)?;
            }

            // adjust MINA token supply
            if let Some(supply) = db.get_block_total_currency(state_hash)? {
                db.set_token(&Token::mina_with_supply(supply))?;
            }
        }

        Ok(())
    }
}

use super::{best::BestLedgerStore, staged::StagedLedgerStore};
use std::collections::HashMap;

/// Aggregate diffs per token account
fn aggregate_token_account_diffs(
    account_diffs: Vec<AccountDiff>,
) -> HashMap<(PublicKey, TokenAddress), Vec<AccountDiff>> {
    let mut token_account_diffs = <HashMap<(_, _), Vec<_>>>::with_capacity(account_diffs.len());

    for diff in account_diffs {
        let pk = diff.public_key();
        let token = diff.token();

        if let Some(mut diffs) = token_account_diffs.remove(&(pk.to_owned(), token.to_owned())) {
            diffs.push(diff);
            token_account_diffs.insert((pk, token), diffs);
        } else {
            token_account_diffs.insert((pk, token), vec![diff]);
        }
    }

    token_account_diffs
}

/// Aggregate token diffs per token
fn aggregate_token_diffs(token_diffs: Vec<TokenDiff>) -> HashMap<TokenAddress, Vec<TokenDiff>> {
    let mut acc = <HashMap<TokenAddress, Vec<TokenDiff>>>::with_capacity(token_diffs.len());

    for diff in token_diffs {
        let token = diff.token.to_owned();

        if let Some(mut diffs) = acc.remove(&token) {
            diffs.push(diff);
            acc.insert(token, diffs);
        } else {
            acc.insert(token, vec![diff]);
        }
    }

    acc
}
