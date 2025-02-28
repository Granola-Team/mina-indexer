//! Zkapp & MINA token store trait

use crate::{
    base::{amount::Amount, public_key::PublicKey},
    ledger::{
        diff::token::TokenDiff,
        token::{holder::TokenHolder, Token, TokenAddress, TokenSymbol},
    },
    store::Result,
};

pub trait ZkappTokenStore {
    /// Set a token
    fn set_token(&self, token: &Token) -> Result<u32>;

    /// Update a token by applying a diff
    ///
    /// Returns the new token if any
    fn apply_token_diff(&self, diff: &TokenDiff) -> Result<Option<Token>>;

    /// Update a token by unapplying last diff
    ///
    /// Returns the new token if any
    fn unapply_token_diff(&self, token: &TokenAddress) -> Result<Option<Token>>;

    ////////////////////////
    // Current token info //
    ////////////////////////

    /// Get a token
    fn get_token(&self, token: &TokenAddress) -> Result<Option<Token>>;

    /// Get the token at the specified index
    fn get_token_at_index(&self, index: u32) -> Result<Option<Token>>;

    /// Get the index of the specified token
    fn get_token_index(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the list of all tokens
    fn get_all_tokens(&self) -> Result<Vec<Token>>;

    /// Get the number of tokens
    fn get_num_tokens(&self) -> Result<u32>;

    /// Get the supply of a token
    fn get_token_supply(&self, token: &TokenAddress) -> Result<Option<Amount>>;

    /// Get the owner of a token
    fn get_token_owner(&self, token: &TokenAddress) -> Result<Option<PublicKey>>;

    /// Get the symbol of a token
    fn get_token_symbol(&self, token: &TokenAddress) -> Result<Option<TokenSymbol>>;

    ///////////////////////
    // Token holder info //
    ///////////////////////

    /// Get the number of token holders
    fn get_token_holders_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the token holder at `index`
    fn get_token_holder(&self, token: &TokenAddress, index: u32) -> Result<Option<TokenHolder>>;

    /// Get the token holder's index
    fn get_token_holder_index(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the list of all token holders for the specified token
    fn get_token_holders(&self, token: &TokenAddress) -> Result<Option<Vec<TokenHolder>>>;

    /// Get the list of tokens held by `pk`
    fn get_tokens_held(&self, pk: &PublicKey) -> Result<Vec<TokenHolder>>;

    ///////////////////////////
    // Historical token info //
    ///////////////////////////

    /// Get the historical token owner at the specified index
    fn get_token_historical_owner_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<PublicKey>>;

    /// Get the count of historical token owners
    fn get_token_historical_owner_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the historical token symbol at the specified index
    fn get_token_historical_symbol_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<TokenSymbol>>;

    /// Get the count of historical token symbols
    fn get_token_historical_symbol_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the historical token supply at the specified index
    fn get_token_historical_supply_index(
        &self,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<PublicKey>>;

    /// Get the count of historical token supplies
    fn get_token_historical_supply_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the applied token diff with specified index
    fn get_token_diff(&self, token: &TokenAddress, index: u32) -> Result<Option<TokenDiff>>;

    /// Get the count of token diffs applied to the token
    fn get_token_diff_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Updates token diffs & pk token diffs
    fn update_token_diffs(&self, token_diff: &TokenDiff) -> Result<()>;

    /// Get the applied `pk` token diff with specified index
    fn get_token_pk_diff(&self, pk: &PublicKey, index: u32) -> Result<Option<TokenDiff>>;

    /// Get the count of token diffs applied to the token by `pk`
    fn get_token_pk_diff_num(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the number of tokens for `pk`
    fn get_token_pk_num(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the `index` of token held by `pk`
    fn get_token_pk_index(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the token held by `pk` at `index`
    fn get_token_pk(&self, pk: &PublicKey, index: u32) -> Result<Option<TokenHolder>>;

    /// Remove & return last applied token diff: `Some(count, token diff)`
    fn remove_last_token_diff(&self, token: &TokenAddress) -> Result<Option<(u32, TokenDiff)>>;

    /// Remove & return last applied pk token diff: `Some(count, token diff)`
    fn remove_last_pk_token_diff(&self, pk: &PublicKey) -> Result<Option<(u32, TokenDiff)>>;
}
