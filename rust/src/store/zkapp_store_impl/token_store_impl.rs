//! Zkapp & MINA token store impl
//!
//! The token store keeps track of historical:
//! - token diffs: `TokenAddress -> Vec<TokenDiff>`
//! - token holders: `TokenAddress -> HashSet<PublicKey>`
//! - token per pk: `PublicKey -> HashSet<TokenHolder>`
//! - owners: `TokenAddress -> Vec<PublicKey>`
//! - supplies: `TokenAddress -> Vec<Amount>`
//! - symbols: `TokenAddress -> Vec<TokenSymbol>`

use crate::{
    base::{amount::Amount, public_key::PublicKey, state_hash::StateHash},
    constants::MINA_TOKEN_ADDRESS,
    ledger::{
        account::Account,
        diff::token::TokenDiff,
        store::best::BestLedgerStore,
        token::{Token, TokenAddress, TokenSymbol},
    },
    store::{
        column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys,
        zkapp::tokens::ZkappTokenStore, IndexerStore, Result,
    },
    utility::store::{
        common::{from_be_bytes, U64_LEN},
        zkapp::tokens::*,
    },
};
use anyhow::Context;
use log::trace;
use speedb::{DBIterator, Direction, IteratorMode};
use std::collections::HashMap;

impl ZkappTokenStore for IndexerStore {
    fn set_token(&self, token: &Token) -> Result<u32> {
        trace!("Setting token {}", token.token);

        // delete old sorting data
        if let Some(supply) = self.get_token_supply(&token.token)? {
            self.database.delete_cf(
                self.zkapp_tokens_supply_sort_cf(),
                zkapp_tokens_supply_sort_key(supply.0, &token.token),
            )?;
        }

        let index = self.get_token_index(&token.token)?.unwrap_or_else(|| {
            // no index exists for this token so we create a new one
            let num = self.get_num_tokens().unwrap();
            trace!("Creating new token index {} for {}", num, token.token);

            // increment the number of tokens
            self.database
                .put(Self::ZKAPP_TOKEN_COUNT, (num + 1).to_be_bytes())
                .unwrap();

            // modify owner info
            if let Some(owner) = token.owner.as_ref() {
                // set new token holder count
                self.database
                    .put_cf(
                        self.zkapp_tokens_holder_count_cf(),
                        token.token.0.as_bytes(),
                        1u32.to_be_bytes(),
                    )
                    .unwrap();

                // set owner's token index
                self.database
                    .put_cf(
                        self.zkapp_tokens_holder_index_cf(),
                        zkapp_tokens_holder_index_key(&token.token, owner),
                        0u32.to_be_bytes(),
                    )
                    .unwrap();

                let account = self
                    .get_best_account(owner, &token.token)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| {
                        let token_account = Account {
                            balance: token.supply,
                            ..Account::empty(owner.to_owned(), token.token.to_owned(), true)
                        };

                        self.update_best_account(
                            &token_account.public_key,
                            token_account.token.as_ref().unwrap(),
                            None,
                            Some(token_account.to_owned()),
                            false,
                        )
                        .unwrap();

                        token_account
                    });

                let pk_index = self
                    .get_token_pk_index(owner, &token.token)
                    .unwrap()
                    .unwrap_or_else(|| {
                        let pk_num = self.get_token_pk_num(owner).unwrap().unwrap_or_default();
                        self.database
                            .put_cf(
                                self.zkapp_tokens_pk_index_cf(),
                                zkapp_tokens_pk_index_key(&token.token, owner),
                                pk_num.to_be_bytes(),
                            )
                            .unwrap();

                        self.database
                            .put_cf(
                                self.zkapp_tokens_pk_num_cf(),
                                owner.0.as_bytes(),
                                (pk_num + 1).to_be_bytes(),
                            )
                            .unwrap();

                        pk_num
                    });

                self.database
                    .put_cf(
                        self.zkapp_tokens_pk_cf(),
                        zkapp_tokens_pk_key(owner, pk_index),
                        serde_json::to_vec(&account).unwrap(),
                    )
                    .unwrap();

                self.database
                    .put_cf(
                        self.zkapp_tokens_holder_cf(),
                        zkapp_tokens_holder_key(&token.token, 0),
                        serde_json::to_vec(&account).unwrap(),
                    )
                    .unwrap();
            }

            num
        });

        // set the token at its index
        self.database.put_cf(
            self.zkapp_tokens_at_index_cf(),
            index.to_be_bytes(),
            serde_json::to_vec(token)?,
        )?;

        // set the token's index
        self.database.put_cf(
            self.zkapp_tokens_index_cf(),
            token.token.0.as_bytes(),
            index.to_be_bytes(),
        )?;

        // set the token
        self.database.put_cf(
            self.zkapp_tokens_cf(),
            token.token.0.as_bytes(),
            serde_json::to_vec(token)?,
        )?;

        // set the token's supply
        self.database.put_cf(
            self.zkapp_tokens_supply_cf(),
            token.token.0.as_bytes(),
            serde_json::to_vec(&token.supply)?,
        )?;

        // set the token's owner
        self.database.put_cf(
            self.zkapp_tokens_owner_cf(),
            token.token.0.as_bytes(),
            serde_json::to_vec(&token.owner)?,
        )?;

        // set the token's symbol
        if token.token.0 != MINA_TOKEN_ADDRESS {
            self.database.put_cf(
                self.zkapp_tokens_symbol_cf(),
                token.token.0.as_bytes(),
                serde_json::to_vec(&token.symbol)?,
            )?;
        }

        // sort the token by supply
        self.database.put_cf(
            self.zkapp_tokens_supply_sort_cf(),
            zkapp_tokens_supply_sort_key(token.supply.0, &token.token),
            serde_json::to_vec(&token)?,
        )?;

        Ok(index)
    }

    fn apply_token_diff(&self, state_hash: &StateHash, diff: &TokenDiff) -> Result<Option<Token>> {
        trace!("Applying {} token diff {:?}", state_hash, diff);

        // get token to modify
        let diff_pk = &diff.public_key;
        let diff_token = &diff.token;

        let mut token = self.get_token(diff_token)?.unwrap_or_else(|| {
            // look for a pre-existing owner
            if let Some((owner, owner_token)) = self
                .get_tokens_used(state_hash)
                .expect("tokens used")
                .and_then(|used| used.get(diff_token).cloned())
            {
                // look for a pre-existing symbol
                let symbol = self
                    .get_best_account(&owner, &owner_token)
                    .expect("best ledger account")
                    .and_then(|acct| acct.token_symbol)
                    .unwrap_or_default();

                Token::new_with_owner_symbol(diff_token.to_owned(), owner, symbol)
            } else {
                Token::new_with_owner(diff.token.to_owned(), diff_pk.to_owned())
            }
        });

        self.set_token(&token)?;

        // check token address
        assert_eq!(token.token, *diff_token);

        // update diff count
        let diff_num = self.get_token_diff_num(diff_token)?.unwrap_or_default();
        self.database.put_cf(
            self.zkapp_tokens_historical_diffs_num_cf(),
            diff.token.0.as_bytes(),
            (diff_num + 1).to_be_bytes(),
        )?;

        // write token diff
        self.database.put_cf(
            self.zkapp_tokens_historical_diffs_cf(),
            zkapp_tokens_historical_diffs_key(diff_token, diff_num),
            serde_json::to_vec(diff)?,
        )?;

        // update holder info
        let holder_index = self
            .get_token_holder_index(diff_token, diff_pk)?
            .unwrap_or_else(|| {
                let index = self
                    .get_token_holders_num(diff_token)
                    .with_context(|| format!("token holders count for {}", diff_token))
                    .expect("token holders count")
                    .unwrap_or_default();

                // increment holder count
                self.database
                    .put_cf(
                        self.zkapp_tokens_holder_count_cf(),
                        diff_token.0.as_bytes(),
                        (index + 1).to_be_bytes(),
                    )
                    .unwrap();

                // write holder's token index
                self.database
                    .put_cf(
                        self.zkapp_tokens_holder_index_cf(),
                        zkapp_tokens_holder_index_key(diff_token, diff_pk),
                        index.to_be_bytes(),
                    )
                    .unwrap();

                index
            });

        let account = self
            .get_token_holder(diff_token, holder_index)?
            .unwrap_or_else(|| {
                self.get_best_account(diff_pk, diff_token)
                    .with_context(|| {
                        format!(
                            "token {} holder {} index {}",
                            diff_token, diff_pk, holder_index
                        )
                    })
                    .expect("token holder")
                    .unwrap_or_else(|| {
                        let token_account = Account {
                            balance: token.supply,
                            ..Account::empty(diff_pk.to_owned(), diff_token.to_owned(), true)
                        };

                        self.update_best_account(
                            diff_pk,
                            diff_token,
                            None,
                            Some(token_account.to_owned()),
                            false,
                        )
                        .unwrap();

                        token_account
                    })
            });

        assert_eq!(account.public_key, *diff_pk);

        self.database.put_cf(
            self.zkapp_tokens_holder_cf(),
            zkapp_tokens_holder_key(diff_token, holder_index),
            serde_json::to_vec(&account)?,
        )?;

        // update pk diffs - count & append diff
        let diff_num = self.get_token_pk_diff_num(diff_pk)?.unwrap_or_default();
        self.database.put_cf(
            self.zkapp_tokens_historical_pk_diffs_num_cf(),
            diff_pk.0.as_bytes(),
            (diff_num + 1).to_be_bytes(),
        )?;

        self.database.put_cf(
            self.zkapp_tokens_historical_pk_diffs_cf(),
            zkapp_tokens_historical_pk_diffs_key(diff_pk, diff_num),
            serde_json::to_vec(diff)?,
        )?;

        // update pk token accounts
        let pk_token_index = self
            .get_token_pk_index(diff_pk, diff_token)?
            .unwrap_or_else(|| {
                let pk_index = self.get_token_pk_num(diff_pk).unwrap().unwrap_or_default();

                self.database
                    .put_cf(
                        self.zkapp_tokens_pk_num_cf(),
                        diff_pk.0.as_bytes(),
                        (pk_index + 1).to_be_bytes(),
                    )
                    .unwrap();

                self.database
                    .put_cf(
                        self.zkapp_tokens_pk_index_cf(),
                        zkapp_tokens_pk_index_key(diff_token, diff_pk),
                        pk_index.to_be_bytes(),
                    )
                    .unwrap();

                pk_index
            });

        self.database.put_cf(
            self.zkapp_tokens_pk_cf(),
            zkapp_tokens_pk_key(diff_pk, pk_token_index),
            serde_json::to_vec(&account)?,
        )?;

        token.apply(diff.to_owned());
        self.set_token(&token)?;

        Ok(Some(token))
    }

    fn unapply_token_diff(&self, token: &TokenAddress) -> Result<Option<Token>> {
        trace!("Unapplying the last token diff for {}", token);

        if let Some((_, diff)) = self.remove_last_token_diff(token)? {
            let diff_pk = &diff.public_key;
            let diff_token = &diff.token;

            // get token to modify
            let mut token = self.get_token(diff_token)?.expect("token");

            // check token address
            assert_eq!(token.token, diff.token);

            // update diff count
            let num = self.get_token_diff_num(diff_token)?.unwrap_or_default();
            self.database.put_cf(
                self.zkapp_tokens_historical_diffs_num_cf(),
                diff.token.0.as_bytes(),
                (num - 1).to_be_bytes(),
            )?;

            // delete diff
            self.database.delete_cf(
                self.zkapp_tokens_historical_diffs_cf(),
                zkapp_tokens_historical_diffs_key(diff_token, num - 1),
            )?;

            // update holder info
            let index = self.get_token_holder_index(diff_token, diff_pk)?.unwrap();
            let account = self
                .get_token_holder(diff_token, index)?
                .expect("unapply token holder");

            self.database.put_cf(
                self.zkapp_tokens_holder_cf(),
                zkapp_tokens_holder_key(diff_token, index),
                serde_json::to_vec(&account)?,
            )?;

            // unapply & set token
            token.unapply(diff.to_owned());
            self.database.put_cf(
                self.zkapp_tokens_cf(),
                token.token.0.as_bytes(),
                serde_json::to_vec(&token)?,
            )?;

            return Ok(Some(token));
        }

        Ok(None)
    }

    fn get_token(&self, token: &TokenAddress) -> Result<Option<Token>> {
        trace!("Getting token {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_cf(), token.0.as_bytes())?
            .map(|token| serde_json::from_slice(&token).expect("token")))
    }

    fn get_token_diff(&self, token: &TokenAddress, index: u32) -> Result<Option<TokenDiff>> {
        trace!("Getting token diff index {} for {}", index, token);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_diffs_cf(),
                zkapp_tokens_historical_diffs_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token diff index {} for {}", index, token))
                    .expect("token diff index")
            }))
    }

    fn get_last_token_diff(&self, token: &TokenAddress) -> Result<Option<TokenDiff>> {
        trace!("Getting the last token diff for {}", token);

        let num = self.get_token_diff_num(token)?.unwrap_or_default();
        if num < 1 {
            unreachable!(
                "Tokens should not have a diff unapplied if there aren't any applied diffs"
            )
        }

        self.get_token_diff(token, num - 1)
    }

    fn get_token_diff_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting token diff count for {}", token);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_diffs_num_cf(),
                token.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn update_token_diffs(&self, token_diff: &TokenDiff) -> Result<()> {
        // update diffs
        let num = self
            .get_token_diff_num(&token_diff.token)?
            .unwrap_or_default();

        self.database.put_cf(
            self.zkapp_tokens_historical_diffs_cf(),
            zkapp_tokens_historical_diffs_key(&token_diff.token, num),
            serde_json::to_vec(token_diff)?,
        )?;

        self.database.put_cf(
            self.zkapp_tokens_historical_diffs_num_cf(),
            token_diff.token.0.as_bytes(),
            (num + 1).to_be_bytes(),
        )?;

        // update pk diffs
        let pk_num = self
            .get_token_pk_diff_num(&token_diff.public_key)?
            .unwrap_or_default();

        self.database.put_cf(
            self.zkapp_tokens_historical_pk_diffs_cf(),
            zkapp_tokens_historical_diffs_key(&token_diff.token, pk_num),
            serde_json::to_vec(token_diff)?,
        )?;

        self.database.put_cf(
            self.zkapp_tokens_historical_pk_diffs_num_cf(),
            token_diff.public_key.0.as_bytes(),
            (pk_num + 1).to_be_bytes(),
        )?;

        Ok(())
    }

    fn get_token_pk_diff(&self, pk: &PublicKey, index: u32) -> Result<Option<TokenDiff>> {
        trace!("Getting pk token diff {} index {}", pk, index);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_pk_diffs_cf(),
                zkapp_tokens_historical_pk_diffs_key(pk, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("pk token diff {} index {}", pk, index))
                    .expect("pk token diff")
            }))
    }

    fn get_token_pk_diff_num(&self, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting pk token diff count {}", pk);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_pk_diffs_num_cf(),
                pk.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_last_token_pk_diff(&self, pk: &PublicKey) -> Result<Option<TokenDiff>> {
        trace!("Getting last pk token diff {}", pk);

        let index = self
            .get_token_pk_diff_num(pk)?
            .expect("pk token diffs")
            .saturating_sub(1);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_pk_diffs_cf(),
                zkapp_tokens_historical_pk_diffs_key(pk, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("pk last token diff {} index {}", pk, index))
                    .expect("pk last token diff")
            }))
    }

    fn remove_last_token_diff(&self, token: &TokenAddress) -> Result<Option<(u32, TokenDiff)>> {
        let num = self.get_token_diff_num(token)?.unwrap();
        trace!("Removing last token diff of {} for {}", num, token);

        if num < 1 {
            unreachable!("Cannot remove a non-existent token diff!")
        }

        // decrement diff count
        let index = num - 1;
        self.database.put_cf(
            self.zkapp_tokens_historical_diffs_num_cf(),
            token.0.as_bytes(),
            index.to_be_bytes(),
        )?;

        // get diff to return
        let diff = self
            .database
            .get_cf(
                self.zkapp_tokens_historical_diffs_cf(),
                zkapp_tokens_historical_diffs_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token {} index {}", token, index))
                    .expect("historical diff")
            });

        // delete the diff
        self.database.delete_cf(
            self.zkapp_tokens_historical_diffs_cf(),
            zkapp_tokens_historical_diffs_key(token, index),
        )?;

        // remove pk last token diff
        if let Some(pk) = diff.as_ref().map(|x: &TokenDiff| &x.public_key) {
            self.remove_last_pk_token_diff(pk)?;
        }

        Ok(diff.map(|diff| (index, diff)))
    }

    fn remove_last_pk_token_diff(&self, pk: &PublicKey) -> Result<Option<(u32, TokenDiff)>> {
        let num = self.get_token_pk_diff_num(pk)?.unwrap();
        trace!("Removing last pk token diff of {} for {}", num, pk);

        if num < 1 {
            unreachable!("Cannot remove a non-existent token diff!")
        }

        // decrement pk diff count
        let index = num - 1;
        self.database.put_cf(
            self.zkapp_tokens_historical_pk_diffs_num_cf(),
            pk.0.as_bytes(),
            index.to_be_bytes(),
        )?;

        // get diff to return
        let diff = self
            .database
            .get_cf(
                self.zkapp_tokens_historical_pk_diffs_cf(),
                zkapp_tokens_historical_pk_diffs_key(pk, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("pk token diff {} index {}", pk, index))
                    .expect("pk token diff")
            });

        // delete the diff
        self.database.delete_cf(
            self.zkapp_tokens_historical_pk_diffs_cf(),
            zkapp_tokens_historical_pk_diffs_key(pk, index),
        )?;

        Ok(diff.map(|diff| (index, diff)))
    }

    fn get_all_tokens(&self) -> Result<Vec<Token>> {
        trace!("Getting all tokens");
        let mut tokens = vec![];

        let num = self.get_num_tokens()?;
        for index in 0..num {
            tokens.push(
                self.get_token_at_index(index)?
                    .with_context(|| format!("token at index {}", index))
                    .expect("token"),
            );
        }

        Ok(tokens)
    }

    fn get_num_tokens(&self) -> Result<u32> {
        trace!("Getting tokens count");

        Ok(self
            .database
            .get(Self::ZKAPP_TOKEN_COUNT)?
            .map(from_be_bytes)
            .unwrap_or_default())
    }

    fn get_token_index(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting index for token {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_index_cf(), token.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_token_at_index(&self, index: u32) -> Result<Option<Token>> {
        trace!("Getting token at index {}", index);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_at_index_cf(), index.to_be_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token index {}", index))
                    .expect("token")
            }))
    }

    fn get_token_supply(&self, token: &TokenAddress) -> Result<Option<Amount>> {
        trace!("Getting supply for token {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_supply_cf(), token.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token supply {}", token))
                    .expect("supply")
            }))
    }

    fn get_token_owner(&self, token: &TokenAddress) -> Result<Option<PublicKey>> {
        trace!("Getting owner for token {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_owner_cf(), token.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token owner {}", token))
                    .expect("owner")
            }))
    }

    fn get_token_historical_owner_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<PublicKey>> {
        trace!(
            "Getting the historical owner token {} index {}",
            token,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_owners_cf(),
                zkapp_tokens_historical_owners_key(token, index),
            )?
            .map(|bytes| {
                PublicKey::from_bytes(&bytes)
                    .with_context(|| {
                        format!("historical owner index {} for token {}", index, token)
                    })
                    .expect("historical token owner")
            }))
    }

    fn get_token_historical_owner_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting the historical token owner count for {}", token);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_owners_num_cf(),
                token.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_token_historical_symbol_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<TokenSymbol>> {
        trace!(
            "Getting the historical symbol token {} index {}",
            token,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_symbols_cf(),
                zkapp_tokens_historical_symbols_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| {
                        format!("historical symbols index {} for token {}", index, token)
                    })
                    .expect("historical token symbols")
            }))
    }

    fn get_token_historical_symbol_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting the historical token symbol count for {}", token);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_symbols_num_cf(),
                token.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_token_historical_supply_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<PublicKey>> {
        trace!(
            "Getting the historical supply token {} index {}",
            token,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_supplies_cf(),
                zkapp_tokens_historical_supplies_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| {
                        format!("historical supplies index {} for token {}", index, token)
                    })
                    .expect("historical token supplies")
            }))
    }

    fn get_token_historical_supply_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!(
            "Getting the historical token supply diff count for {}",
            token
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_historical_supplies_num_cf(),
                token.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_token_symbol(&self, token: &TokenAddress) -> Result<Option<TokenSymbol>> {
        trace!("Getting symbol for token {}", token);

        if token.0 == MINA_TOKEN_ADDRESS {
            return Ok(Some(TokenSymbol::mina()));
        }

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_symbol_cf(), token.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token symbol {}", token))
                    .expect("symbol")
            }))
    }

    fn set_mina_token_holders_num(&self, num: u32) -> Result<()> {
        trace!("Setting the count of MINA token holders to {}", num);

        Ok(self.database.put_cf(
            self.zkapp_tokens_holder_count_cf(),
            MINA_TOKEN_ADDRESS.as_bytes(),
            num.to_be_bytes(),
        )?)
    }

    fn get_token_holders_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting token holders count for {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_holder_count_cf(), token.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_token_holder(&self, token: &TokenAddress, index: u32) -> Result<Option<Account>> {
        trace!("Getting token {} holder at index {}", token, index);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_holder_cf(),
                zkapp_tokens_holder_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token {} holder index {}", token, index))
                    .expect("token holder index")
            }))
    }

    fn get_token_holder_index(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting token {} holder {} index", token, pk);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_holder_index_cf(),
                zkapp_tokens_holder_index_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn get_token_holders(&self, token: &TokenAddress) -> Result<Option<Vec<Account>>> {
        trace!("Getting token holders for {}", token);
        let mut holders = vec![];

        if let Some(num) = self.get_token_holders_num(token)? {
            for index in 0..num {
                holders.push(
                    self.get_token_holder(token, index)?
                        .with_context(|| format!("token {} holder index {}", token, index))
                        .expect("token holder index"),
                );
            }

            return Ok(Some(holders));
        }

        Ok(None)
    }

    fn get_tokens_held(&self, pk: &PublicKey) -> Result<Vec<Account>> {
        trace!("Getting tokens held by {}", pk);
        let mut tokens = vec![];

        if let Some(num) = self.get_token_pk_num(pk)? {
            for index in 0..num {
                tokens.push(
                    self.get_token_pk(pk, index)?
                        .with_context(|| format!("token held by {} index {}", pk, index))
                        .expect("tokens held"),
                );
            }
        }

        Ok(tokens)
    }

    fn get_token_txns_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting token transactions count for {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_txns_num_cf(), token.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn increment_token_txns_num(&self, token: &TokenAddress) -> Result<()> {
        trace!("Incrementing token txn count for {}", token);

        let num = self.get_token_txns_num(token)?.unwrap_or_default();
        Ok(self.database.put_cf(
            self.zkapp_tokens_txns_num_cf(),
            token.0.as_bytes(),
            (num + 1).to_be_bytes(),
        )?)
    }

    fn decrement_token_txns_num(&self, token: &TokenAddress) -> Result<()> {
        trace!("Decrementing token txn count for {}", token);

        let num = self.get_token_txns_num(token)?.unwrap_or_default();
        assert!(num >= 1);

        Ok(self.database.put_cf(
            self.zkapp_tokens_txns_num_cf(),
            token.0.as_bytes(),
            (num - 1).to_be_bytes(),
        )?)
    }

    fn get_token_pk_num(&self, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting held token count for {}", pk);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_pk_num_cf(), pk.0.as_bytes())?
            .with_context(|| format!("zkapp token pk num {}", pk))
            .map(from_be_bytes)
            .ok())
    }

    fn get_token_pk(&self, pk: &PublicKey, index: u32) -> Result<Option<Account>> {
        trace!("Getting held token for {} index {}", pk, index);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_pk_cf(), zkapp_tokens_pk_key(pk, index))?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("held token for {} index {}", pk, index))
                    .expect("pk index")
            }))
    }

    fn get_token_pk_index(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting token index for {} token {}", pk, token);

        Ok(self
            .database
            .get_cf(
                self.zkapp_tokens_pk_index_cf(),
                zkapp_tokens_pk_index_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn get_tokens_used(
        &self,
        state_hash: &StateHash,
    ) -> Result<Option<HashMap<TokenAddress, (PublicKey, TokenAddress)>>> {
        trace!("Getting tokens used {}", state_hash);

        Ok(self
            .database
            .get_cf(self.blocks_tokens_used_cf(), state_hash.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("tokens used for {}", state_hash))
                    .expect("tokens used")
            }))
    }

    ///////////////
    // Iterators //
    ///////////////

    /// CF: [zkapp_tokens_at_index_cf]
    fn token_iterator(&self) -> DBIterator<'_> {
        self.database
            .iterator_cf(self.zkapp_tokens_at_index_cf(), IteratorMode::Start)
    }

    /// Key: [zkapp_tokens_supply_sort_key]
    ///
    /// CF:  [zkapp_tokens_supply_sort_cf]
    fn token_supply_iterator(&self, supply: Option<u64>, direction: Direction) -> DBIterator<'_> {
        let start = if let Some(supply) = supply {
            let mut start = [0u8; U64_LEN];

            match direction {
                Direction::Forward => {
                    start[..U64_LEN].copy_from_slice(&supply.to_be_bytes());
                }
                Direction::Reverse => {
                    start[..U64_LEN].copy_from_slice(&supply.saturating_add(1).to_be_bytes());
                }
            }

            Some(start)
        } else {
            None
        };

        let mode = if let Some(start) = start.as_ref() {
            IteratorMode::From(start, direction)
        } else {
            match direction {
                Direction::Forward => IteratorMode::Start,
                Direction::Reverse => IteratorMode::End,
            }
        };

        self.database
            .iterator_cf(self.zkapp_tokens_supply_sort_cf(), mode)
    }

    /// Iterator for holder-specific token accounts
    ///```
    /// key: [zkapp_tokens_pk_key]
    /// cf:  [zkapp_tokens_pk_cf]
    fn tokens_pk_iterator(&self, pk: &PublicKey) -> DBIterator<'_> {
        self.database.iterator_cf(
            self.zkapp_tokens_pk_cf(),
            IteratorMode::From(pk.0.as_bytes(), Direction::Forward),
        )
    }
}

#[cfg(test)]
mod token_store_tests {
    use crate::{
        block::precomputed::PrecomputedBlock,
        command::{UserCommandWithStatus, UserCommandWithStatusT},
    };
    use std::path::PathBuf;

    #[test]
    fn test_token_symbol_parsed() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-359630-3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL.json");

        let block: PrecomputedBlock = PrecomputedBlock::from_path(&path)?;
        let commands = block.zkapp_commands();
        let zkapp_command: &UserCommandWithStatus = commands.get(3).unwrap();

        assert!(zkapp_command.is_zkapp_command());

        match zkapp_command.clone() {
            UserCommandWithStatus::V2(zkapp_command) => {
                match zkapp_command.data.1.to_owned() {
                    crate::mina_blocks::v2::staged_ledger_diff::UserCommandData::SignedCommandData(_) =>
                        panic!("Unexpected command type"),
                    crate::mina_blocks::v2::staged_ledger_diff::UserCommandData::ZkappCommandData(data) => {
                        if let Some(account_update) = data.account_updates.first() {
                            let token_symbol: crate::mina_blocks::v2::staged_ledger_diff::UpdateKind =
                                account_update.clone().elt.account_update.body.update.token_symbol;
                            match token_symbol {
                                crate::mina_blocks::v2::staged_ledger_diff::UpdateKind::Keep(_) =>
                                    panic!("Should have been Set"),
                                crate::mina_blocks::v2::staged_ledger_diff::UpdateKind::Set((_, token_symbol)) => {
                                    assert_eq!(token_symbol, "MINU");
                                    Ok(())
                                },
                            }
                        } else {
                            Err(anyhow::anyhow!("No account updates found")) // Add error return for empty case
                        }
                    },
                }
            }
            _ => panic!("Unexpected command type"),
        }
    }
}
