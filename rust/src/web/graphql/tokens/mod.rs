//! GraphQL `tokens` & `tokenHolders` endpoints

use super::{accounts, db};
use crate::{
    base::public_key::PublicKey,
    ledger::{self, account, store::best::BestLedgerStore, token::TokenAddress},
    store::{zkapp::tokens::ZkappTokenStore, IndexerStore},
    utility::store::common::U64_LEN,
    web::graphql::accounts::Account,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject, Default)]
pub struct TokensQueryInput {
    token: Option<String>,
    owner: Option<String>,
    symbol: Option<String>,
    supply: Option<u64>,
}

#[derive(SimpleObject)]
pub struct Token {
    #[graphql(flatten)]
    token: TokenSimple,

    /// Value count of token holders
    #[graphql(name = "num_holders")]
    num_holders: u32,

    /// Value total count of token transactions
    #[graphql(name = "total_num_txns")]
    total_num_txns: u32,

    /// Value total count of locked tokens
    #[graphql(name = "total_num_locked")]
    total_num_locked: u64,

    /// Value total count of tokens
    #[graphql(name = "total_num_tokens")]
    total_num_tokens: u32,
}

#[derive(Enum, Default, Copy, Clone, Eq, PartialEq)]
pub enum TokensSortByInput {
    #[graphql(name = "SUPPLY_ASC")]
    SupplyAsc,

    #[default]
    #[graphql(name = "SUPPLY_DESC")]
    SupplyDesc,
}

#[derive(SimpleObject)]
pub struct TokenSimple {
    /// Value token address
    token: String,

    /// Value token supply
    supply: u64,

    /// Value token owner
    owner: Option<String>,

    /// Value token symbol
    symbol: Option<String>,
}

#[derive(InputObject)]
pub struct TokenHoldersQueryInput {
    /// Value token address of holder account
    token: Option<String>,

    /// Value public key of holder account
    holder: Option<String>,
}

#[derive(SimpleObject)]
pub struct TokenHolder {
    /// Value token address
    token: String,

    /// Value token supply
    supply: u64,

    /// Value token owner
    owner: Option<String>,

    /// Value token symbol
    symbol: Option<String>,

    /// Value token account
    account: accounts::Account,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum TokenHoldersSortByInput {
    #[graphql(name = "BALANCE_ASC")]
    BalanceAsc,

    #[graphql(name = "BALANCE_DESC")]
    BalanceDesc,
}

struct TokenWithMeta {
    token: ledger::token::Token,
    num_holders: u32,
    total_num_txns: u32,
    total_num_locked: u64,
    total_num_tokens: u32,
}

#[derive(Default)]
pub struct TokensQueryRoot;

#[Object]
impl TokensQueryRoot {
    async fn tokens<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<TokensQueryInput>,
        sort_by: Option<TokensSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<Token>> {
        let db = db(ctx);

        // specific token query
        if query
            .as_ref()
            .map(|q| q.token.is_some())
            .unwrap_or_default()
        {
            if let Some(token) = query.as_ref().map(|q| {
                q.token
                    .as_ref()
                    .and_then(TokenAddress::new)
                    .unwrap_or_default()
            }) {
                if let Some(token) = db.get_token(&token)? {
                    let token = TokenWithMeta::new(db, token).expect("token with meta");

                    return Ok(vec![token.into()]);
                }
            } else {
                return Err(async_graphql::Error::new(format!(
                    "Invalid token address: {}",
                    query.as_ref().unwrap().token.as_ref().unwrap()
                )));
            }
        }

        // default query
        let mut tokens = Vec::with_capacity(limit);
        for (_, value) in db.token_iterator().flatten() {
            let token = serde_json::from_slice(&value)?;

            if TokensQueryInput::matches(query.as_ref(), &token) {
                tokens.push(TokenWithMeta::new(db, token)?.into());
            }

            if tokens.len() >= limit {
                break;
            }
        }

        match sort_by {
            Some(TokensSortByInput::SupplyAsc) => {
                tokens.sort_by(|x: &Token, y: &Token| x.token.supply.cmp(&y.token.supply))
            }
            Some(TokensSortByInput::SupplyDesc) | None => {
                tokens.sort_by(|x: &Token, y: &Token| y.token.supply.cmp(&x.token.supply))
            }
        }

        Ok(tokens)
    }

    async fn token_holders<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<TokenHoldersQueryInput>,
        sort_by: Option<TokenHoldersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TokenHolder>> {
        let db = db(ctx);
        let mut holders = Vec::with_capacity(limit);

        // specific token's holder accounts
        if let Some(token) = query
            .as_ref()
            .and_then(|q| q.token.as_ref().and_then(TokenAddress::new).to_owned())
        {
            let direction = match sort_by {
                Some(TokenHoldersSortByInput::BalanceDesc) | None => speedb::Direction::Reverse,
                Some(TokenHoldersSortByInput::BalanceAsc) => speedb::Direction::Forward,
            };

            let mut start = [0u8; TokenAddress::LEN + U64_LEN + PublicKey::LEN];
            start[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());

            if let Direction::Reverse = direction {
                start[TokenAddress::LEN..][..U64_LEN].copy_from_slice(&u64::MAX.to_be_bytes());
                start[TokenAddress::LEN..][U64_LEN..]
                    .copy_from_slice(PublicKey::upper_bound().0.as_bytes());
            };

            let mode = speedb::IteratorMode::From(&start, direction);
            for (key, value) in db.best_ledger_account_balance_iterator(mode).flatten() {
                if key[..TokenAddress::LEN] != *token.0.as_bytes() {
                    break;
                }

                let account = serde_json::from_slice(&value)?;
                let token = db.get_token(&token)?.unwrap_or_default();

                if TokenHoldersQueryInput::matches(query.as_ref(), &account) {
                    let account = TokenAccount {
                        token,
                        account: Account::with_meta(db, account),
                    };
                    holders.push(account.into());
                }

                if holders.len() >= limit {
                    break;
                }
            }

            return Ok(holders);
        }

        // specific holder's token accounts
        if let Some(holder) = query.as_ref().and_then(|q| q.holder.to_owned()) {
            for (key, value) in db.tokens_pk_iterator(&holder.to_owned().into()).flatten() {
                if key[..PublicKey::LEN] != *holder.as_bytes() {
                    break;
                }

                let account: account::Account = serde_json::from_slice(&value)?;
                let token = account.token.to_owned().unwrap_or_default();

                if TokenHoldersQueryInput::matches(query.as_ref(), &account) {
                    let account = TokenAccount {
                        token: db.get_token(&token)?.expect("token"),
                        account: Account::with_meta(db, account),
                    };

                    holders.push(account.into());

                    if holders.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(holders)
    }
}

///////////
// impls //
///////////

impl TokensQueryInput {
    fn matches(query: Option<&Self>, token: &ledger::token::Token) -> bool {
        if let Some(query) = query {
            let Self {
                token: q_token,
                owner,
                symbol,
                supply,
            } = query;

            // token
            if let Some(q_token) = q_token {
                if *q_token != token.token.0 {
                    return false;
                }
            }

            // owner
            if let Some(q_owner) = owner {
                if let Some(owner) = token.owner.as_ref() {
                    if *q_owner != owner.0 {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // symbol
            if let Some(q_symbol) = symbol {
                if *q_symbol != token.symbol.0 {
                    return false;
                }
            }

            // supply
            if let Some(q_supply) = supply {
                if *q_supply != token.supply.0 {
                    return false;
                }
            }
        }

        true
    }
}

impl TokenHoldersQueryInput {
    fn matches(query: Option<&Self>, account: &ledger::account::Account) -> bool {
        if let Some(query) = query {
            let Self {
                token: q_token,
                holder,
            } = query;

            // token
            if let Some(q_token) = q_token {
                if *q_token != account.token.to_owned().unwrap_or_default().0 {
                    return false;
                }
            }

            // holder
            if let Some(holder) = holder {
                if *holder != account.public_key.0 {
                    return false;
                }
            }
        }

        true
    }
}

impl TokenWithMeta {
    fn new(db: &std::sync::Arc<IndexerStore>, token: ledger::token::Token) -> Result<Self> {
        Ok(Self {
            num_holders: db.get_token_holders_num(&token.token)?.unwrap_or_default(),
            total_num_txns: db.get_token_txns_num(&token.token)?.unwrap_or_default(),
            total_num_tokens: db.get_num_tokens()?,
            total_num_locked: 0,
            token,
        })
    }
}

/////////////////
// conversions //
/////////////////

impl From<TokenWithMeta> for Token {
    fn from(value: TokenWithMeta) -> Self {
        Self {
            token: value.token.into(),
            num_holders: value.num_holders,
            total_num_txns: value.total_num_txns,
            total_num_locked: value.total_num_locked,
            total_num_tokens: value.total_num_tokens,
        }
    }
}

impl From<ledger::token::Token> for TokenSimple {
    fn from(value: ledger::token::Token) -> Self {
        Self {
            token: value.token.to_string(),
            supply: value.supply.0,
            owner: value.owner.map(Into::into),
            symbol: Some(value.symbol.to_string()),
        }
    }
}

struct TokenAccount {
    account: accounts::Account,
    token: ledger::token::Token,
}

impl From<TokenAccount> for TokenHolder {
    fn from(value: TokenAccount) -> Self {
        Self {
            account: value.account,
            token: value.token.token.to_string(),
            supply: value.token.supply.0,
            owner: value.token.owner.map(Into::into),
            symbol: Some(value.token.symbol.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TokensQueryInput;
    use crate::{
        base::public_key::PublicKey,
        ledger::token::{Token, TokenAddress, TokenSymbol},
    };

    #[test]
    fn matches() {
        let query = TokensQueryInput {
            symbol: Some("MINU".to_string()),
            ..Default::default()
        };

        // does not match MINA token
        let mina = Token::mina_with_supply(100000000000000);
        assert!(!TokensQueryInput::matches(Some(&query), &mina));

        // matches MINU token
        let minu = Token {
            token: TokenAddress::new("wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd").unwrap(),
            owner: PublicKey::new("B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF").ok(),
            symbol: TokenSymbol::new("MINU"),
            supply: 100000000000000.into(),
        };
        assert!(TokensQueryInput::matches(Some(&query), &minu));
    }
}
