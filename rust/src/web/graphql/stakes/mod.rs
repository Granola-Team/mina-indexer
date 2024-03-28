use crate::{
    ledger::{staking::StakingAccount, store::LedgerStore},
    store::IndexerStore,
};
use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::sync::Arc;

#[derive(InputObject)]
pub struct StakesQueryInput {
    epoch: u32,
}

#[derive(Default)]
pub struct StakesQueryRoot;

#[Object]
impl StakesQueryRoot {
    async fn stakes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<StakesQueryInput>,
    ) -> Result<Option<Vec<LedgerAccountWithMeta>>> {
        let db = ctx
            .data::<Arc<IndexerStore>>()
            .expect("db to be in context");

        let epoch = match query {
            Some(query) => query.epoch,
            None => return Ok(None),
        };

        let staking_ledger = match db.get_staking_ledger_at_epoch("mainnet", epoch)? {
            Some(staking_ledger) => staking_ledger,
            None => return Ok(None),
        };

        let ledger_hash = staking_ledger.ledger_hash.clone().0;
        let accounts = staking_ledger
            .staking_ledger
            .into_values()
            .map(|account| LedgerAccountWithMeta {
                epoch,
                ledger_hash: ledger_hash.clone(),
                account: LedgerAccount::from(account),
            })
            .collect();

        Ok(Some(accounts))
    }
}

#[derive(SimpleObject)]
pub struct LedgerAccountWithMeta {
    /// Value current epoch
    epoch: u32,
    /// Value current ledger hash
    ledger_hash: String,
    /// Value accounts
    #[graphql(flatten)]
    account: LedgerAccount,
}

#[derive(SimpleObject)]
pub struct LedgerAccount {
    /// Value chainId
    chain_id: String,
    /// Value balance
    balance: f64,
    /// Value nonce
    nonce: u32,
    /// Value delegate
    delegate: String,
    /// Value epoch
    pk: String,
    /// Value public key
    public_key: String,
    /// Value token
    token: u32,
    /// Value receipt chain hash
    receipt_chain_hash: String,
    /// Value voting for
    voting_for: String,
    /// Value balance nanomina
    balance_nanomina: u64,
}

impl From<StakingAccount> for LedgerAccount {
    fn from(acc: StakingAccount) -> Self {
        let balance_nanomina = acc.balance;
        let mut decimal = Decimal::from(balance_nanomina);
        decimal.set_scale(9).ok();
        let balance = decimal.to_f64().unwrap_or_default();
        let nonce = acc.nonce.unwrap_or_default();
        let delegate = acc.delegate.0;
        let pk = acc.pk.0;
        let public_key = pk.clone();
        let token = acc.token;
        let receipt_chain_hash = acc.receipt_chain_hash.0;
        let voting_for = acc.voting_for.0;

        Self {
            chain_id: "5f704c".to_string(),
            balance,
            nonce,
            delegate,
            pk,
            public_key,
            token,
            receipt_chain_hash,
            voting_for,
            balance_nanomina,
        }
    }
}
