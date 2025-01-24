use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, DbUpdate, IndexerStore};
use crate::{
    block::{
        store::{BlockStore, BlockUpdate, DbBlockUpdate},
        BlockHash,
    },
    ledger::{
        account::Account,
        diff::account::AccountDiff,
        public_key::PublicKey,
        store::{
            best::{BestLedgerStore, DbAccountUpdate},
            staged::StagedLedgerStore,
        },
        token::TokenAddress,
        Ledger, TokenLedger,
    },
    store::zkapp::{actions::ZkappActionStore, events::ZkappEventStore},
    utility::store::{
        common::{from_be_bytes, pk_index_key},
        ledger::best::*,
    },
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::collections::HashSet;

impl BestLedgerStore for IndexerStore {
    fn get_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Getting best ledger account {pk}");
        Ok(self
            .database
            .get_cf(self.best_ledger_accounts_cf(), best_account_key(token, pk))?
            .map(|bytes| {
                serde_json::from_slice::<Account>(&bytes)
                    .unwrap_or_else(|_| panic!("{} token {} missing", pk, token))
            }))
    }

    fn get_best_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Display best ledger account {pk}");
        if let Some(best_acct) = self.get_best_account(pk, token)? {
            return Ok(Some(best_acct.display()));
        }
        Ok(None)
    }

    fn get_best_ledger(&self, memoize: bool) -> anyhow::Result<Option<Ledger>> {
        Ok(self.build_best_ledger()?.inspect(|best_ledger| {
            if let (Ok(Some(state_hash)), Ok(Some(block_height))) =
                (self.get_best_block_hash(), self.get_best_block_height())
            {
                if memoize {
                    trace!("Memoizing best ledger (state hash {state_hash})");
                    self.add_staged_ledger_at_state_hash(
                        &state_hash,
                        best_ledger.clone(),
                        block_height,
                    )
                    .ok();
                }
            }
        }))
    }

    fn update_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        before_balance: Option<u64>,
        after: Option<Account>,
    ) -> anyhow::Result<()> {
        // remove account
        if after.is_none() {
            if let Some(before_balance) = before_balance {
                self.database
                    .delete_cf(self.best_ledger_accounts_cf(), best_account_key(token, pk))?;
                self.database.delete_cf(
                    self.best_ledger_accounts_balance_sort_cf(),
                    best_account_sort_key(token, before_balance, pk),
                )?;
            }
            return Ok(());
        }

        // update best account
        let after = after.unwrap();
        let balance = after.balance.0;

        if let Some(before_balance) = before_balance {
            // delete stale balance sorting data
            self.database.delete_cf(
                self.best_ledger_accounts_balance_sort_cf(),
                best_account_sort_key(token, before_balance, pk),
            )?;
        }

        // write new account
        self.database.put_cf(
            self.best_ledger_accounts_cf(),
            best_account_key(token, pk),
            serde_json::to_vec(&after)?,
        )?;

        if let Some(_zkapp) = after.zkapp {
            // populate index for best_ledger_zkapps_balance_sort_cf
            // populate index for best_ledger_tokens_balance_sort_cf
        } else {
            self.database.put_cf(
                self.best_ledger_accounts_balance_sort_cf(),
                best_account_sort_key(token, balance, pk),
                serde_json::to_vec(&after)?,
            )?;
        }

        Ok(())
    }

    fn update_block_best_accounts(
        &self,
        state_hash: &BlockHash,
        blocks: &DbBlockUpdate,
    ) -> anyhow::Result<()> {
        let account_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .flat_map(|BlockUpdate { state_hash: a, .. }| {
                    let diff = self.get_block_ledger_diff(a).unwrap();
                    diff.map(|d| {
                        (
                            d.account_diffs.into_iter().flatten().collect(),
                            update_token_accounts(d.new_pk_balances),
                        )
                    })
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .flat_map(|BlockUpdate { state_hash: u, .. }| {
                    let diff = self.get_block_ledger_diff(u).unwrap();
                    diff.map(|d| {
                        (
                            d.account_diffs.into_iter().flatten().collect(),
                            update_token_accounts(d.new_pk_balances),
                        )
                    })
                })
                .collect(),
        };
        self.update_best_accounts(state_hash, account_updates)
    }

    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: DbAccountUpdate,
    ) -> anyhow::Result<()> {
        use AccountDiff::*;
        trace!("Updating best ledger accounts for block {state_hash}");

        // count newly applied & unapplied accounts
        let apply_acc = updates
            .apply
            .iter()
            .fold(0, |acc, update| acc + update.1.len() as i32);
        let adjust = updates
            .unapply
            .iter()
            .fold(apply_acc, |acc, update| acc - update.1.len() as i32);
        self.update_num_accounts(adjust)?;

        // update accounts
        // unapply
        for (unapply_block_diffs, remove_pks) in updates.unapply {
            let token_account_diffs = aggregate_token_account_diffs(unapply_block_diffs);

            for ((pk, token), diffs) in token_account_diffs {
                let before = self.get_best_account(&pk, &token)?;
                let (before_balance, before) = (
                    before.as_ref().map(|a| a.balance.0),
                    before.unwrap_or(Account::empty(pk.clone(), token.clone())),
                );

                let mut after = before.clone();
                for diff in diffs.iter() {
                    after = match diff {
                        Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                            after.payment_unapply(diff)
                        }
                        Coinbase(diff) => after.coinbase_unapply(diff),
                        Delegation(diff) => {
                            self.remove_pk_delegate(pk.clone())?;
                            after.delegation_unapply(diff)
                        }
                        FailedTransactionNonce(diff) => after.failed_transaction_unapply(diff),

                        // zkapp diffs
                        ZkappActionsDiff(diff) => {
                            self.remove_actions(&pk, &token, diff.actions.len() as u32)?;
                            after
                        }
                        ZkappEventsDiff(diff) => {
                            self.remove_events(&pk, &token, diff.events.len() as u32)?;
                            after
                        }

                        // TODO zkapp unapply
                        ZkappStateDiff(_)
                        | ZkappPermissionsDiff(_)
                        | ZkappVerificationKeyDiff(_)
                        | ZkappUriDiff(_)
                        | ZkappTokenSymbolDiff(_)
                        | ZkappTimingDiff(_)
                        | ZkappVotingForDiff(_)
                        | ZkappIncrementNonce(_)
                        | ZkappAccountCreationFee(_) => after,
                        Zkapp(_) => unreachable!(),
                    };
                }

                self.update_best_account(&pk, &token, before_balance, Some(after))?;
            }

            // remove accounts
            for (pk, token) in remove_pks.iter() {
                self.update_best_account(pk, token, None, None)?;
            }
        }

        // apply
        for (block_apply_diffs, _) in updates.apply.into_iter() {
            let token_account_diffs = aggregate_token_account_diffs(block_apply_diffs);

            for ((pk, token), diffs) in token_account_diffs {
                let before = self.get_best_account(&pk, &token)?;
                let (before_balance, before) = (
                    before.as_ref().map(|a| a.balance.0),
                    before.unwrap_or(Account::empty(pk.clone(), token.clone())),
                );

                let mut after = before.clone();
                for diff in diffs.iter() {
                    after = match diff {
                        Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                            after.payment(diff)
                        }
                        Coinbase(diff) => after.coinbase(diff.amount),
                        Delegation(diff) => after.delegation(diff.delegate.clone(), diff.nonce),
                        FailedTransactionNonce(diff) => after.failed_transaction(diff.nonce),
                        ZkappStateDiff(diff) => after.zkapp_state(diff),
                        ZkappPermissionsDiff(diff) => after.zkapp_permissions(diff),
                        ZkappVerificationKeyDiff(diff) => after.zkapp_verification_key(diff),
                        ZkappUriDiff(diff) => after.zkapp_uri(diff),
                        ZkappTokenSymbolDiff(diff) => after.zkapp_token_symbol(diff),
                        ZkappTimingDiff(diff) => after.zkapp_timing(diff),
                        ZkappVotingForDiff(diff) => after.zkapp_voting_for(diff),
                        ZkappIncrementNonce(diff) => after.zkapp_nonce(diff),
                        ZkappAccountCreationFee(diff) => after.zkapp_account_creation(diff),

                        // these diffs do not modify the account
                        ZkappActionsDiff(diff) => {
                            self.add_actions(&diff.public_key, &diff.token, &diff.actions)?;
                            after
                        }
                        ZkappEventsDiff(diff) => {
                            self.add_events(&diff.public_key, &diff.token, &diff.events)?;
                            after
                        }
                        Zkapp(_) => unreachable!(),
                    };
                }

                self.update_best_account(&pk, &token, before_balance, Some(after))?;
            }
        }
        Ok(())
    }

    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> anyhow::Result<()> {
        trace!("Adding pk {pk} delegate {delegate}");
        let num = self.get_num_pk_delegations(pk)?;

        // update num delegations
        self.database.put_cf(
            self.best_ledger_accounts_num_delegations_cf(),
            pk.0.as_bytes(),
            (num + 1).to_be_bytes(),
        )?;

        // append new delegation
        self.database.put_cf(
            self.best_ledger_accounts_delegations_cf(),
            pk_index_key(pk, num),
            delegate.0.as_bytes(),
        )?;
        Ok(())
    }

    fn get_num_pk_delegations(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_num_delegations_cf(),
                pk.0.as_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting pk {pk} delegation index {idx}");
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_delegations_cf(),
                pk_index_key(pk, idx),
            )?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn remove_pk_delegate(&self, pk: PublicKey) -> anyhow::Result<()> {
        trace!("Removing pk {pk} delegate");
        let idx = self.get_num_pk_delegations(&pk)?;
        if idx > 0 {
            // update num delegations
            self.database.put_cf(
                self.best_ledger_accounts_num_delegations_cf(),
                pk.0.as_bytes(),
                (idx - 1).to_be_bytes(),
            )?;

            // drop delegation
            self.database.delete_cf(
                self.best_ledger_accounts_delegations_cf(),
                pk_index_key(&pk, idx - 1),
            )?;
        }
        Ok(())
    }

    fn update_num_accounts(&self, adjust: i32) -> anyhow::Result<()> {
        use std::cmp::Ordering::*;
        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                let old = self.get_num_accounts().ok().flatten().unwrap_or(0);
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                let old = self.get_num_accounts().ok().flatten().unwrap_or(0);
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_sub(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
        }
        Ok(())
    }

    fn get_num_accounts(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .database
            .get(Self::TOTAL_NUM_ACCOUNTS_KEY)?
            .map(from_be_bytes))
    }

    fn build_best_ledger(&self) -> anyhow::Result<Option<Ledger>> {
        trace!("Building best ledger");
        if let (Some(best_block_height), Some(best_block_hash)) =
            (self.get_best_block_height()?, self.get_best_block_hash()?)
        {
            trace!("Best ledger (length {best_block_height}): {best_block_hash}");
            let mut accounts = HashMap::new();

            for (_, value) in self
                .best_ledger_account_balance_iterator(IteratorMode::End)
                .flatten()
            {
                let account: Account = serde_json::from_slice(&value)?;
                accounts.insert(account.public_key.clone(), account);
            }

            return Ok(Some(Ledger::from_mina_ledger(TokenLedger { accounts })));
        }
        Ok(None)
    }

    ///////////////
    // Iterators //
    ///////////////

    fn best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.best_ledger_accounts_balance_sort_cf(), mode)
    }
}

use std::collections::HashMap;

/// Aggregate diffs per token account
fn aggregate_token_account_diffs(
    account_diffs: Vec<AccountDiff>,
) -> HashMap<(PublicKey, TokenAddress), Vec<AccountDiff>> {
    let mut token_account_diffs = <HashMap<(_, _), Vec<_>>>::with_capacity(account_diffs.len());

    for diff in account_diffs {
        let pk = diff.public_key();
        let token = diff.token_address();

        if let Some(mut diffs) = token_account_diffs.remove(&(pk.to_owned(), token.to_owned())) {
            diffs.push(diff);
            token_account_diffs.insert((pk, token), diffs);
        } else {
            token_account_diffs.insert((pk, token), vec![diff]);
        }
    }

    token_account_diffs
}

use std::collections::BTreeMap;

fn update_token_accounts(
    new_pk_balances: BTreeMap<PublicKey, BTreeMap<TokenAddress, u64>>,
) -> HashSet<(PublicKey, TokenAddress)> {
    new_pk_balances
        .into_iter()
        .flat_map(|(pk, tokens)| {
            tokens
                .into_keys()
                .map(|token| (pk.to_owned(), token))
                .collect::<HashSet<_>>()
        })
        .collect()
}
