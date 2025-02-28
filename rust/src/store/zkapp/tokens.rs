//! Zkapp & MINA token store trait

use crate::{
    base::{amount::Amount, public_key::PublicKey},
    ledger::{
        diff::token::TokenDiff,
        token::{holder::TokenHolder, Token, TokenAddress, TokenSymbol},
    },
};

type Result<T> = anyhow::Result<T>;

pub trait ZkappTokenStore {
    /// Set a token
    fn set_token(&self, token: &Token) -> Result<u32>;

    /// Get a token
    fn get_token(&self, token: &TokenAddress) -> Result<Option<Token>>;

    /// Update a token with a diff
    fn update_token(&self, diff: &TokenDiff) -> Result<Option<Token>>;

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

    /// Get the number of token holders
    fn get_token_holders_num(&self, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the token holder at `index`
    fn get_token_holder(&self, token: &TokenAddress, index: u32) -> Result<Option<TokenHolder>>;

    /// Get the list of all token holders
    fn get_token_holders(&self, token: &TokenAddress) -> Result<Option<Vec<TokenHolder>>>;

    /// Get the number of tokens for `pk`
    fn get_token_pk_num(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the `index` of token held by `pk`
    fn get_token_pk_index(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>>;

    /// Get the token held by `pk` at `index`
    fn get_token_pk(&self, pk: &PublicKey, index: u32) -> Result<Option<TokenHolder>>;

    /// Get the list of tokens held by `pk`
    fn get_tokens_held(&self, pk: &PublicKey) -> Result<Vec<TokenHolder>>;
}
