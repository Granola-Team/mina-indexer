//! Token store impl

use crate::{
    base::{amount::Amount, public_key::PublicKey},
    ledger::{
        diff::token::{TokenDiff, TokenDiffType},
        token::{holder::TokenHolder, Token, TokenAddress, TokenSymbol},
    },
    store::{
        column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys,
        zkapp::tokens::ZkappTokenStore, IndexerStore,
    },
    utility::store::{
        common::from_be_bytes,
        zkapp::tokens::{zkapp_tokens_holder_key, zkapp_tokens_pk_index_key, zkapp_tokens_pk_key},
    },
};
use anyhow::Context;
use log::trace;

type Result<T> = anyhow::Result<T>;

impl ZkappTokenStore for IndexerStore {
    fn set_token(&self, token: &Token) -> Result<u32> {
        trace!("Setting token {}", token.token);

        let index = match self.get_token(&token.token)? {
            None => {
                let num = self.get_num_tokens()?;

                // increment the number of tokens
                self.database
                    .put(Self::ZKAPP_TOKEN_COUNT, (num + 1).to_be_bytes())?;

                num
            }
            Some(token) => self.get_token_index(&token.token)?.unwrap(),
        };

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
        self.database.put_cf(
            self.zkapp_tokens_symbol_cf(),
            token.token.0.as_bytes(),
            serde_json::to_vec(&token.symbol)?,
        )?;

        // increment token holder count
        let num = self
            .get_token_holders_num(&token.token)?
            .unwrap_or_default();
        self.database.put_cf(
            self.zkapp_tokens_holder_count_cf(),
            token.token.0.as_bytes(),
            (num + 1).to_be_bytes(),
        )?;

        // modify token holder & pk info
        if let Some(public_key) = token.owner.to_owned() {
            let pk_index = self
                .get_token_pk_num(&public_key)?
                .map(|n| n + 1)
                .unwrap_or_default();

            let holder = TokenHolder {
                balance: token.supply,
                token: token.token.to_owned(),
                public_key: public_key.to_owned(),
            };

            self.database.put_cf(
                self.zkapp_tokens_holder_cf(),
                zkapp_tokens_holder_key(&token.token, pk_index),
                serde_json::to_vec(&holder)?,
            )?;

            self.database.put_cf(
                self.zkapp_tokens_pk_cf(),
                zkapp_tokens_pk_key(&public_key, pk_index),
                serde_json::to_vec(&holder)?,
            )?;

            self.database.put_cf(
                self.zkapp_tokens_pk_index_cf(),
                zkapp_tokens_pk_index_key(&token.token, &public_key),
                pk_index.to_be_bytes(),
            )?;

            self.database.put_cf(
                self.zkapp_tokens_pk_num_cf(),
                public_key.0.as_bytes(),
                (pk_index + 1).to_be_bytes(),
            )?;
        }

        Ok(index)
    }

    fn get_token(&self, token: &TokenAddress) -> Result<Option<Token>> {
        trace!("Getting token {}", token);

        Ok(self
            .database
            .get_pinned_cf(self.zkapp_tokens_cf(), token.0.as_bytes())?
            .map(|token| serde_json::from_slice(&token).expect("token")))
    }

    fn update_token(&self, diff: &TokenDiff) -> Result<Option<Token>> {
        trace!("Updating token {}", diff.token);

        let token = self.get_token(&diff.token)?.unwrap_or_default();
        let token = {
            use TokenDiffType::*;

            match &diff.diff {
                Supply(amt) => Token {
                    supply: token.supply + *amt,
                    ..token
                },
                Owner(owner) => Token {
                    owner: Some(owner.to_owned()),
                    ..token
                },
                Symbol(symbol) => Token {
                    symbol: symbol.to_owned(),
                    ..token
                },
            }
        };

        self.set_token(&token)?;
        Ok(Some(token))
    }

    fn get_all_tokens(&self) -> Result<Vec<Token>> {
        trace!("Getting all tokens");
        let mut tokens = vec![];

        for index in 0..self.get_num_tokens()? {
            tokens.push(self.get_token_at_index(index)?.expect("token"));
        }

        Ok(tokens)
    }

    fn get_num_tokens(&self) -> Result<u32> {
        trace!("Getting number of tokens");

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
            .get_pinned_cf(self.zkapp_tokens_at_index_cf(), index.to_be_bytes())?
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
            .get_pinned_cf(self.zkapp_tokens_supply_cf(), token.0.as_bytes())?
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
            .get_pinned_cf(self.zkapp_tokens_owner_cf(), token.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token owner {}", token))
                    .expect("owner")
            }))
    }

    fn get_token_symbol(&self, token: &TokenAddress) -> Result<Option<TokenSymbol>> {
        trace!("Getting symbol for token {}", token);

        Ok(self
            .database
            .get_pinned_cf(self.zkapp_tokens_symbol_cf(), token.0.as_bytes())?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token symbol {}", token))
                    .expect("symbol")
            }))
    }

    fn get_token_holders_num(&self, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting token holders count for {}", token);

        Ok(self
            .database
            .get_cf(self.zkapp_tokens_holder_count_cf(), token.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_token_holder(&self, token: &TokenAddress, index: u32) -> Result<Option<TokenHolder>> {
        trace!("Getting token holder {} index {}", token, index);

        Ok(self
            .database
            .get_pinned_cf(
                self.zkapp_tokens_holder_cf(),
                zkapp_tokens_holder_key(token, index),
            )?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("token {} index {}", token, index))
                    .expect("token holder")
            }))
    }

    fn get_token_holders(&self, token: &TokenAddress) -> Result<Option<Vec<TokenHolder>>> {
        trace!("Getting token holders for {}", token);
        let mut holders = vec![];

        if let Some(num) = self.get_token_holders_num(token)? {
            for index in 0..num {
                holders.push(
                    self.get_token_holder(token, index)?
                        .with_context(|| format!("token {} index {}", token, index))
                        .expect("token holder"),
                );
            }

            return Ok(Some(holders));
        }

        Ok(None)
    }

    fn get_tokens_held(&self, pk: &PublicKey) -> Result<Vec<TokenHolder>> {
        trace!("Getting tokens held by {}", pk);
        let mut tokens = vec![];

        if let Some(num) = self.get_token_pk_num(pk)? {
            for index in 0..num {
                tokens.push(
                    self.get_token_pk(pk, index)?
                        .with_context(|| format!("token held by {} index {}", pk, index))
                        .expect("pk token"),
                );
            }
        }

        Ok(tokens)
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

    fn get_token_pk(&self, pk: &PublicKey, index: u32) -> Result<Option<TokenHolder>> {
        trace!("Getting held token for {} index {}", pk, index);

        Ok(self
            .database
            .get_pinned_cf(self.zkapp_tokens_pk_cf(), zkapp_tokens_pk_key(pk, index))?
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
}

#[cfg(all(test, feature = "tier2"))]
mod tests {
    use super::Result;
    use crate::{
        base::public_key::PublicKey,
        ledger::token::{holder::TokenHolder, Token},
        store::{zkapp::tokens::ZkappTokenStore, IndexerStore},
    };
    use quickcheck::{Arbitrary, Gen};

    #[test]
    fn check() -> Result<()> {
        let tmp = tempfile::TempDir::new()?;
        let store = IndexerStore::new(tmp.path())?;

        assert_eq!(store.get_num_tokens()?, 0);

        // set the token
        let g = &mut Gen::new(1000);
        let mut token = Token::arbitrary(g);
        token.owner = Some(PublicKey::arbitrary(g));

        store.set_token(&token)?;
        assert_eq!(store.get_num_tokens()?, 1);

        // check indexes
        assert_eq!(store.get_token(&token.token)?.unwrap(), token);
        assert_eq!(store.get_token_at_index(0)?.unwrap(), token);
        assert_eq!(store.get_token_index(&token.token)?.unwrap(), 0);

        // check all tokens & properties
        assert_eq!(store.get_all_tokens()?, vec![token.to_owned()]);
        assert_eq!(store.get_token_supply(&token.token)?.unwrap(), token.supply);
        assert_eq!(store.get_token_owner(&token.token)?, token.owner);
        assert_eq!(store.get_token_symbol(&token.token)?.unwrap(), token.symbol);

        // check token holders
        let token_holder = store.get_token_holder(&token.token, 0)?.unwrap();

        assert_eq!(token_holder.balance, token.supply);
        assert_eq!(token_holder.public_key, token.owner.to_owned().unwrap());
        assert_eq!(token_holder.token, token.token);

        assert_eq!(store.get_token_holders_num(&token.token)?.unwrap(), 1);
        assert_eq!(
            store.get_token_holders(&token.token)?.unwrap(),
            vec![TokenHolder {
                public_key: token.owner.to_owned().unwrap(),
                token: token.token.to_owned(),
                balance: token.supply,
            }]
        );
        assert_eq!(
            store.get_tokens_held(&token.owner.to_owned().unwrap())?,
            store.get_token_holders(&token.token)?.unwrap(),
        );

        Ok(())
    }
}
