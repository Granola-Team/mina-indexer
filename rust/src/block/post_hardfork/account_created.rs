use crate::{
    ledger::{amount::Amount, public_key::PublicKey, token::TokenAddress},
    mina_blocks::v2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCreated {
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub creation_fee: Amount,
}

impl From<v2::AccountCreated> for AccountCreated {
    fn from(value: v2::AccountCreated) -> Self {
        let public_key = value.0 .0;
        let token = value.0 .1;
        let creation_fee = value.1.parse().expect("account creation fee");

        Self {
            public_key,
            token,
            creation_fee,
        }
    }
}
