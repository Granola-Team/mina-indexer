use crate::{
    ledger::{public_key::PublicKey, token::TokenAddress},
    mina_blocks::v2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsed {
    pub used_token: TokenAddress,
    pub token_owner: Option<PublicKey>,
    pub payment_token: Option<TokenAddress>,
}

impl From<v2::TokenUsed> for TokenUsed {
    fn from(value: v2::TokenUsed) -> Self {
        let used_token = value.0;
        let mut token_owner = None;
        let mut payment_token = None;

        if let Some((owner, token)) = value.1 {
            token_owner = Some(owner);
            payment_token = Some(token);
        }

        Self {
            used_token,
            token_owner,
            payment_token,
        }
    }
}
