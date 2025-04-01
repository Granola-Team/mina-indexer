use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, DbUpdate, IndexerStore};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{
        store::{BlockStore, BlockUpdate, DbBlockUpdate},
        AccountCreated,
    },
    chain::store::ChainStore,
    constants::MINA_TOKEN_ADDRESS,
    ledger::{
        account::Account,
        diff::{account::AccountDiff, token::TokenDiff},
        store::{
            best::{AccountUpdate, BestLedgerStore, DbAccountUpdate},
            staged::StagedLedgerStore,
        },
        token::{account::TokenAccount, Token, TokenAddress},
        Ledger, TokenLedger,
    },
    store::{
        zkapp::{actions::ZkappActionStore, events::ZkappEventStore, tokens::ZkappTokenStore},
        Result,
    },
    utility::store::{
        common::{from_be_bytes, pk_index_key},
        ledger::best::*,
    },
};
use log::trace;
use speedb::{DBIterator, IteratorMode};
use std::collections::HashSet;

impl BestLedgerStore for IndexerStore {
    fn get_best_account(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<Account>> {
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
    ) -> Result<Option<Account>> {
        trace!("Display best ledger account {pk}");
        if let Some(best_acct) = self.get_best_account(pk, token)? {
            return Ok(Some(best_acct.display()));
        }
        Ok(None)
    }

    fn update_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        before: Option<(bool, u64)>,
        after: Option<Account>,
    ) -> Result<()> {
        // remove account
        if after.is_none() {
            if let Some(before) = before {
                // generic token account
                let account_key = best_account_key(token, pk);
                let sort_key = best_account_sort_key(token, before.1, pk);

                // delete account
                self.database
                    .delete_cf(self.best_ledger_accounts_cf(), account_key)?;
                self.database
                    .delete_cf(self.best_ledger_accounts_balance_sort_cf(), sort_key)?;

                // zkapp account
                if before.0 {
                    self.decrement_num_zkapp_accounts()?;

                    // delete
                    self.database
                        .delete_cf(self.zkapp_best_ledger_accounts_cf(), account_key)?;
                    self.database
                        .delete_cf(self.zkapp_best_ledger_accounts_balance_sort_cf(), sort_key)?;
                }
            }

            return Ok(());
        }

        // update best account
        let after = after.unwrap();
        let balance = after.balance.0;

        // delete stale balance sorting data
        if let Some(before) = before {
            let sort_key = best_account_sort_key(token, before.1, pk);

            // general account
            self.database
                .delete_cf(self.best_ledger_accounts_balance_sort_cf(), sort_key)?;

            // zkapp account
            if before.0 {
                self.database
                    .delete_cf(self.zkapp_best_ledger_accounts_balance_sort_cf(), sort_key)?;
            }
        }

        let account_key = best_account_key(token, pk);
        let sort_key = best_account_sort_key(token, balance, pk);

        // store new account
        let value = serde_json::to_vec(&after)?;
        self.database
            .put_cf(self.best_ledger_accounts_cf(), account_key, &value)?;

        // balance-sort new account
        self.database.put_cf(
            self.best_ledger_accounts_balance_sort_cf(),
            sort_key,
            &value,
        )?;

        // zkapp account
        if after.is_zkapp_account() {
            self.increment_num_zkapp_accounts()?;

            // store new zkapp account
            self.database
                .put_cf(self.zkapp_best_ledger_accounts_cf(), account_key, &value)?;

            // balance-sort new zkapp account
            self.database.put_cf(
                self.zkapp_best_ledger_accounts_balance_sort_cf(),
                sort_key,
                &value,
            )?;
        }

        Ok(())
    }

    fn apply_best_token_diffs(
        &self,
        state_hash: &StateHash,
        token_diffs: &[TokenDiff],
    ) -> Result<()> {
        trace!("Applying best ledger token diffs {:#?}", token_diffs);

        // TODO get token once & apply all diffs
        for token_diff in token_diffs {
            self.apply_token_diff(state_hash, token_diff)?;
        }

        Ok(())
    }

    fn unapply_best_token_diffs(&self, token_diffs: &[TokenDiff]) -> Result<()> {
        trace!("Unapplying best ledger token diffs {:#?}", token_diffs);

        for token_diff in token_diffs {
            let mut token = self
                .get_token(&token_diff.token)?
                .unwrap_or_else(|| Token::new(token_diff.token.to_owned()));

            if let Some((_, token_diff)) = self.remove_last_token_diff(&token_diff.token)? {
                trace!("Unapplying best ledger token diff {:?}", token_diff);

                // TODO get previous owner/symbol/supply
                token.unapply(token_diff);

                self.set_token(&token)?;
            }
        }

        Ok(())
    }

    fn update_block_best_accounts(
        &self,
        state_hash: &StateHash,
        blocks: &DbBlockUpdate,
    ) -> Result<()> {
        let account_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .flat_map(|BlockUpdate { state_hash: a, .. }| {
                    let diff = self.get_block_ledger_diff(a).unwrap();

                    diff.map(|d| {
                        let (new_accounts, new_zkapp_accounts) =
                            update_token_accounts(self, d.new_pk_balances, d.accounts_created);

                        AccountUpdate {
                            account_diffs: d.account_diffs.into_iter().flatten().collect(),
                            token_diffs: d.token_diffs.into_iter().collect(),
                            new_accounts,
                            new_zkapp_accounts,
                        }
                    })
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .flat_map(|BlockUpdate { state_hash: u, .. }| {
                    let diff = self.get_block_ledger_diff(u).unwrap();

                    diff.map(|d| {
                        let (new_accounts, new_zkapp_accounts) =
                            update_token_accounts(self, d.new_pk_balances, d.accounts_created);

                        AccountUpdate {
                            account_diffs: d.account_diffs.into_iter().flatten().collect(),
                            token_diffs: d.token_diffs.into_iter().collect(),
                            new_accounts,
                            new_zkapp_accounts,
                        }
                    })
                })
                .collect(),
        };

        self.update_best_accounts(state_hash, account_updates)
    }

    fn update_best_accounts(&self, state_hash: &StateHash, updates: DbAccountUpdate) -> Result<()> {
        use AccountDiff::*;
        trace!("Updating best ledger accounts for block {state_hash}");

        // count newly applied & unapplied accounts

        // all accounts
        let apply_acc = updates
            .apply
            .iter()
            .fold(0, |acc, update| acc + update.new_accounts.len() as i32);
        let adjust = updates.unapply.iter().fold(apply_acc, |acc, update| {
            acc - update.new_accounts.len() as i32
        });

        if adjust != 0 {
            self.update_num_accounts(adjust)?;
        }

        // mina accounts
        let mina_accounts = |update: &AccountUpdate| -> i32 {
            update
                .new_accounts
                .iter()
                .filter(|(_, token)| token.0 == MINA_TOKEN_ADDRESS)
                .count() as i32
        };
        let apply_acc = updates
            .apply
            .iter()
            .fold(0, |acc, update| acc + mina_accounts(update));
        let mina_adjust = updates
            .unapply
            .iter()
            .fold(apply_acc, |acc, update| acc - mina_accounts(update));

        if mina_adjust != 0 {
            self.update_num_mina_accounts(mina_adjust)?;
        }

        // zkapp accounts
        let apply_acc = updates.apply.iter().fold(0, |acc, update| {
            acc + update.new_zkapp_accounts.len() as i32
        });
        let zkapp_adjust = updates.unapply.iter().fold(apply_acc, |acc, update| {
            acc - update.new_zkapp_accounts.len() as i32
        });

        if zkapp_adjust != 0 {
            self.update_num_zkapp_accounts(zkapp_adjust)?;
        }

        // unapply account & token diffs, remove accounts
        for AccountUpdate {
            account_diffs,
            token_diffs,
            new_accounts,
            new_zkapp_accounts,
            ..
        } in updates.unapply
        {
            let token_account_diffs = aggregate_token_account_diffs(account_diffs);

            for ((pk, token), diffs) in token_account_diffs {
                let before = self.get_best_account(&pk, &token)?;
                let (before_values, mut after) = (
                    before.as_ref().map(|a| (a.is_zkapp_account(), a.balance.0)),
                    before.unwrap_or_else(|| Account::empty(pk.clone(), token.clone())),
                );

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
                        | ZkappAccountCreationFee(_)
                        | ZkappFeePayerNonce(_) => after,
                        Zkapp(_) => unreachable!(),
                    };
                }

                self.update_best_account(&pk, &token, before_values, Some(after))?;
            }

            // unapply token diffs
            for diffs in aggregate_token_diffs(token_diffs).values() {
                if !diffs.is_empty() {
                    self.unapply_best_token_diffs(diffs)?;
                }
            }

            // remove accounts
            for (pk, token) in new_accounts.iter().chain(new_zkapp_accounts.iter()) {
                self.update_best_account(pk, token, None, None)?;
            }

            // adjust MINA token supply
            if let Some(supply) = self.get_block_total_currency(state_hash)? {
                self.set_token(&Token::mina_with_supply(supply))?;
            }
        }

        // apply account & token diffs
        for AccountUpdate {
            account_diffs,
            token_diffs,
            ..
        } in updates.apply.into_iter()
        {
            let token_account_diffs = aggregate_token_account_diffs(account_diffs);

            // apply account diffs
            for ((pk, token), diffs) in token_account_diffs {
                let before = self.get_best_account(&pk, &token)?;
                let (before_values, mut after) = (
                    before.as_ref().map(|a| (a.is_zkapp_account(), a.balance.0)),
                    before.unwrap_or_else(|| Account::empty(pk.clone(), token.clone())),
                );

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
                        ZkappFeePayerNonce(diff) => after.zkapp_fee_payer_nonce(diff),

                        // these diffs do not modify the account
                        ZkappActionsDiff(diff) => {
                            self.add_actions(&diff.public_key, &diff.token, &diff.actions)?;
                            after
                        }
                        ZkappEventsDiff(diff) => {
                            self.add_events(&diff.public_key, &diff.token, &diff.events)?;
                            after
                        }
                        // zkapp account diffs should be expanded
                        Zkapp(_) => unreachable!(),
                    };
                }

                self.update_best_account(&pk, &token, before_values, Some(after))?;
            }

            // apply token diffs
            for diffs in aggregate_token_diffs(token_diffs).values() {
                if !diffs.is_empty() {
                    self.apply_best_token_diffs(state_hash, diffs)?;
                }
            }
        }

        // adjust MINA token supply
        if let Some(supply) = self.get_block_total_currency(state_hash)? {
            self.set_token(&Token::mina_with_supply(supply))?;
        }

        Ok(())
    }

    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> Result<()> {
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

    fn get_num_pk_delegations(&self, pk: &PublicKey) -> Result<u32> {
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_num_delegations_cf(),
                pk.0.as_bytes(),
            )?
            .map_or(0, from_be_bytes))
    }

    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> Result<Option<PublicKey>> {
        trace!("Getting pk {pk} delegation index {idx}");
        Ok(self
            .database
            .get_cf(
                self.best_ledger_accounts_delegations_cf(),
                pk_index_key(pk, idx),
            )?
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok()))
    }

    fn remove_pk_delegate(&self, pk: PublicKey) -> Result<()> {
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

    //////////////////
    // All accounts //
    //////////////////

    fn update_num_accounts(&self, adjust: i32) -> Result<()> {
        use std::cmp::Ordering::*;

        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                // add to num accounts
                let old = self.get_num_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                // sub from num accounts
                let old = self.get_num_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_ACCOUNTS_KEY,
                    old.saturating_sub(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
        }

        Ok(())
    }

    fn get_num_accounts(&self) -> Result<Option<u32>> {
        trace!("Getting count of all accounts");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_ACCOUNTS_KEY)?
            .map(from_be_bytes))
    }

    fn set_num_accounts(&self, num: u32) -> Result<()> {
        trace!("Setting count of all accounts to {}", num);

        Ok(self
            .database
            .put(Self::TOTAL_NUM_ACCOUNTS_KEY, num.to_be_bytes())?)
    }

    fn decrement_num_accounts(&self) -> Result<()> {
        trace!("Decrementing count of all accounts");

        let old = self.get_num_accounts()?.unwrap_or_default();
        assert!(old >= 1);

        self.set_num_accounts(old - 1)
    }

    fn increment_num_accounts(&self) -> Result<()> {
        trace!("Incrementing count of all accounts");

        let old = self.get_num_accounts()?.unwrap_or_default();
        self.set_num_accounts(old + 1)
    }

    ///////////////////
    // MINA accounts //
    ///////////////////

    fn update_num_mina_accounts(&self, adjust: i32) -> Result<()> {
        use std::cmp::Ordering::*;

        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                // add to num mina accounts
                let old = self.get_num_mina_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_MINA_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                // sub from num mina accounts
                let old = self.get_num_mina_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_MINA_ACCOUNTS_KEY,
                    old.saturating_sub(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
        }

        Ok(())
    }

    fn get_num_mina_accounts(&self) -> Result<Option<u32>> {
        trace!("Getting count of mina accounts");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_MINA_ACCOUNTS_KEY)?
            .map(from_be_bytes))
    }

    fn set_num_mina_accounts(&self, num: u32) -> Result<()> {
        trace!("Setting count of mina accounts to {}", num);

        Ok(self
            .database
            .put(Self::TOTAL_NUM_MINA_ACCOUNTS_KEY, num.to_be_bytes())?)
    }

    fn decrement_num_mina_accounts(&self) -> Result<()> {
        trace!("Decrementing count of mina accounts");

        let old = self.get_num_mina_accounts()?.unwrap_or_default();
        assert!(old >= 1);

        self.set_num_mina_accounts(old - 1)
    }

    fn increment_num_mina_accounts(&self) -> Result<()> {
        trace!("Incrementing count of mina accounts");

        let old = self.get_num_mina_accounts()?.unwrap_or_default();
        self.set_num_mina_accounts(old + 1)
    }

    ////////////////////
    // zkApp accounts //
    ////////////////////

    fn update_num_zkapp_accounts(&self, adjust: i32) -> Result<()> {
        use std::cmp::Ordering::*;

        match adjust.cmp(&0) {
            Equal => (),
            Greater => {
                // add to num accounts
                let old = self.get_num_zkapp_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_ZKAPP_ACCOUNTS_KEY,
                    old.saturating_add(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
            Less => {
                // sub from num accounts
                let old = self.get_num_zkapp_accounts()?.unwrap_or_default();
                self.database.put(
                    Self::TOTAL_NUM_ZKAPP_ACCOUNTS_KEY,
                    old.saturating_sub(adjust.unsigned_abs()).to_be_bytes(),
                )?;
            }
        }

        Ok(())
    }

    fn get_num_zkapp_accounts(&self) -> Result<Option<u32>> {
        trace!("Getting count of zkapp accounts");

        Ok(self
            .database
            .get(Self::TOTAL_NUM_ZKAPP_ACCOUNTS_KEY)?
            .map(from_be_bytes))
    }

    fn set_num_zkapp_accounts(&self, num: u32) -> Result<()> {
        trace!("Setting count of zkapp accounts to {}", num);

        Ok(self
            .database
            .put(Self::TOTAL_NUM_ZKAPP_ACCOUNTS_KEY, num.to_be_bytes())?)
    }

    fn decrement_num_zkapp_accounts(&self) -> Result<()> {
        trace!("Decrementing count of zkapp accounts");

        let old = self.get_num_zkapp_accounts()?.unwrap_or_default();
        assert!(old >= 1);

        self.set_num_zkapp_accounts(old - 1)
    }

    fn increment_num_zkapp_accounts(&self) -> Result<()> {
        trace!("Incrementing count of zkapp accounts");

        let old = self.get_num_zkapp_accounts()?.unwrap_or_default();
        self.set_num_zkapp_accounts(old + 1)
    }

    fn is_zkapp_account(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<bool>> {
        trace!("Checking if ({}, {}) is a zkapp account", pk, token);

        Ok(self
            .get_best_account(pk, token)?
            .as_ref()
            .map(|a| a.zkapp.is_some()))
    }

    /////////////////////////
    // Best ledger builder //
    /////////////////////////

    fn get_best_ledger(&self, memoize: bool) -> Result<Option<Ledger>> {
        Ok(self.build_best_ledger()?.inspect(|best_ledger| {
            if let (Ok(Some(state_hash)), Ok(Some(block_height))) =
                (self.get_best_block_hash(), self.get_best_block_height())
            {
                if memoize {
                    trace!("Memoizing best ledger (state hash {state_hash})");
                    self.add_staged_ledger_at_state_hash(&state_hash, best_ledger, block_height)
                        .ok();
                }
            }
        }))
    }

    fn build_best_ledger(&self) -> Result<Option<Ledger>> {
        trace!("Building best ledger");

        if let (Some(best_block_height), Some(best_block_hash)) =
            (self.get_best_block_height()?, self.get_best_block_hash()?)
        {
            let network = self.get_current_network()?;
            trace!("Best ledger {network}-{best_block_height}-{best_block_hash}");

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

    fn zkapp_best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.zkapp_best_ledger_accounts_balance_sort_cf(), mode)
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

use std::collections::BTreeMap;

type AllAndZkappAccounts = (
    HashSet<(PublicKey, TokenAddress)>, // all accounts
    HashSet<(PublicKey, TokenAddress)>, // zkapp accounts
);

fn update_token_accounts(
    db: &IndexerStore,
    new_pk_balances: BTreeMap<PublicKey, BTreeMap<TokenAddress, u64>>,
    accounts_created: Vec<AccountCreated>,
) -> AllAndZkappAccounts {
    (
        new_pk_balances
            .iter()
            .flat_map(|(pk, tokens)| {
                tokens
                    .keys()
                    .map(|token| (pk.to_owned(), token.to_owned()))
                    .collect::<HashSet<_>>()
            })
            .collect(),
        accounts_created
            .iter()
            .filter_map(
                |AccountCreated {
                     public_key, token, ..
                 }| {
                    if let Ok(Some(true)) = db.is_zkapp_account(public_key, token) {
                        return Some((public_key.to_owned(), token.to_owned()));
                    }

                    None
                },
            )
            .collect(),
    )
}
