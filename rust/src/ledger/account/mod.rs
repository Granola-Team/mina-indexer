mod receipt_chain_hash;
mod timing;

use super::{
    amount::Amount,
    diff::{
        account::{
            AccountDiff, UpdateType, ZkappAccountCreationFee, ZkappActionsDiff, ZkappEventsDiff,
            ZkappIncrementNonce, ZkappPermissionsDiff, ZkappStateDiff, ZkappTimingDiff,
            ZkappTokenSymbolDiff, ZkappUriDiff, ZkappVerificationKeyDiff, ZkappVotingForDiff,
        },
        LedgerDiff,
    },
    nonce::Nonce,
    token::{symbol::TokenSymbol, TokenAddress},
    username::Username,
};
use crate::{
    block::{genesis::GenesisBlock, BlockHash},
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
    mina_blocks::v2::{self, ZkappAccount},
};
use mina_serialization_proc_macros::AutoFrom;
use serde::{Deserialize, Serialize};

// re-export types
pub type ReceiptChainHash = receipt_chain_hash::ReceiptChainHash;
pub type Timing = timing::Timing;

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: Amount,
    pub delegate: PublicKey,
    pub genesis_account: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<Nonce>,

    // optional
    pub token: Option<TokenAddress>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<BlockHash>,
    pub permissions: Option<Permissions>,
    pub timing: Option<Timing>,

    #[serde(skip_deserializing)]
    pub token_symbol: Option<TokenSymbol>,

    // for zkapp accounts
    pub zkapp: Option<ZkappAccount>,

    // for mina search
    pub username: Option<Username>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Permissions {
    pub edit_state: Permission,
    pub access: Permission,
    pub send: Permission,
    pub receive: Permission,
    pub set_delegate: Permission,
    pub set_permissions: Permission,
    pub set_verification_key: (Permission, String),
    pub set_zkapp_uri: Permission,
    pub edit_action_state: Permission,
    pub set_token_symbol: Permission,
    pub increment_nonce: Permission,
    pub set_voting_for: Permission,
    pub set_timing: Permission,
}

#[derive(
    Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AutoFrom,
)]
#[auto_from(v2::PermissionKind)]
pub enum Permission {
    #[default]
    None,
    Either,
    Proof,
    Signature,
    Impossible,
}

impl Account {
    /// Display view of account, removes non-genesis account creation fee
    pub fn display(self) -> Self {
        Self {
            balance: self.balance - MAINNET_ACCOUNT_CREATION_FEE,
            ..self
        }
    }

    /// Time-locked balance (subtracted from circulating supply)
    /// as per https://docs.minaprotocol.com/mina-protocol/time-locked-accounts
    pub fn current_minimum_balance(&self, curr_global_slot: u32) -> u64 {
        self.timing.as_ref().map_or(0, |t| {
            if curr_global_slot < t.cliff_time {
                t.initial_minimum_balance
            } else {
                t.initial_minimum_balance.saturating_sub(
                    ((curr_global_slot - t.cliff_time) / t.vesting_period) as u64
                        * t.vesting_increment,
                )
            }
        })
    }

    /// Creates a new empty account with the specified public key.
    /// This function initializes the account with the given public key and sets
    /// the delegate to the same public key. Other fields are set to their
    /// default values.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key to be associated with the new account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the specified public key and token,
    /// default values for other fields.
    pub fn empty(public_key: PublicKey, token: TokenAddress) -> Self {
        Account {
            public_key: public_key.clone(),
            delegate: public_key,
            token: Some(token),
            ..Default::default()
        }
    }

    /// Sets the username for the account
    pub fn set_username(&mut self, username: Username) -> anyhow::Result<()> {
        self.username = Some(username);
        Ok(())
    }

    /// Updates the account's balance based on applying a coinbase reward.
    /// This function takes the current account state (`pre`) and a reward
    /// amount (`amount`), and returns a new account state with the updated
    /// balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The coinbase reward amount to be added to the account's
    ///   balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    pub fn coinbase(self, amount: Amount) -> Self {
        Account {
            balance: self.balance + amount,
            ..self
        }
    }

    pub fn coinbase_unapply(self, amount: Amount) -> Self {
        Account {
            balance: self.balance - amount,
            ..self
        }
    }

    /// Updates the account's state based on applying a payment.
    /// This function handles both credit and debit updates.
    ///
    /// # Arguments
    ///
    /// * `pre` - The current state of the account.
    /// * `payment_diff` - The `PaymentDiff` containing the update type amount.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated state.
    pub fn payment(self, payment_diff: &PaymentDiff) -> Self {
        match payment_diff.update_type {
            UpdateType::Credit => self.credit(payment_diff.amount),
            UpdateType::Debit(nonce) => self.debit(payment_diff.amount, nonce),
        }
    }

    /// Updates the account's state based on unapplying a payment.
    /// This function handles both credit and debit updates.
    ///
    /// # Arguments
    ///
    /// * `payment_diff` - The `PaymentDiff` containing the update type amount.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated state.
    pub fn payment_unapply(self, payment_diff: &PaymentDiff) -> Self {
        match payment_diff.update_type {
            UpdateType::Credit => Account {
                balance: self.balance - payment_diff.amount,
                ..self
            },
            UpdateType::Debit(nonce) => Self {
                balance: self.balance + payment_diff.amount,
                nonce: nonce.or(self.nonce).map(|n| n - 1),
                ..self
            },
        }
    }

    /// Updates the account's balance and nonce based on applying a debit.
    /// If the `nonce` is `None`, the update originates from an internal
    /// command.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to be debited from the account's balance.
    /// * `nonce` - The new nonce for the account. If `None`, the existing nonce
    ///   is retained.
    ///
    /// # Returns
    ///
    /// An `Option` containing the new `Account` state if the debit was
    /// successful, or `None` if the debit amount exceeds the current
    /// balance.
    fn debit(self, amount: Amount, nonce: Option<Nonce>) -> Self {
        Account {
            balance: self.balance - amount,
            nonce: nonce.or(self.nonce),
            ..self
        }
    }

    /// Updates the account's balance based on applying a credit.
    /// This function takes the current account state and returns a new
    /// account state with the updated balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to be credited to the account's balance.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated balance.
    fn credit(self, amount: Amount) -> Self {
        Account {
            balance: self.balance + amount,
            ..self
        }
    }

    /// Updates the account's delegate and nonce based on a Stake Delegation
    /// transaction. This function takes the current account state (`pre`),
    /// a new delegate (`delegate`), and an updated nonce (`updated_nonce`),
    /// and returns a new account state with these changes.
    ///
    /// # Arguments
    ///
    /// * `delegate` - The new delegate public key for the account.
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated delegate and nonce.
    pub fn delegation(self, delegate: PublicKey, updated_nonce: Nonce) -> Self {
        Account {
            delegate,
            nonce: Some(updated_nonce),
            ..self
        }
    }

    pub fn delegation_unapply(self, nonce: Option<Nonce>) -> Self {
        Account {
            delegate: self.public_key.clone(),
            nonce,
            ..self
        }
    }

    /// Updates the account's nonce based on a failed transaction.
    /// This function takes the current account state (`pre`) and an updated
    /// nonce (`updated_nonce`), and returns a new account state with the
    /// updated nonce.
    ///
    /// # Arguments
    ///
    /// * `updated_nonce` - The new nonce for the account.
    ///
    /// # Returns
    ///
    /// A new `Account` instance with the updated nonce.
    pub fn failed_transaction(self, updated_nonce: Nonce) -> Self {
        Account {
            nonce: Some(updated_nonce),
            ..self
        }
    }

    pub fn failed_transaction_unapply(self, nonce: Option<Nonce>) -> Self {
        Account { nonce, ..self }
    }

    /// Apply zkapp state diff
    pub fn zkapp_state(self, diff: &ZkappStateDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        let mut zkapp = self.zkapp.unwrap_or_default();

        for (idx, diff) in diff.diffs.iter().enumerate() {
            if let Some(app_state) = diff.to_owned() {
                zkapp.app_state[idx] = app_state;
            }
        }

        Self {
            zkapp: Some(zkapp),
            ..self
        }
    }

    /// Apply zkapp verification key diff
    pub fn zkapp_verification_key(self, diff: &ZkappVerificationKeyDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        if let Some(mut zkapp) = self.zkapp {
            zkapp.verification_key = diff.verification_key.to_owned();

            return Self {
                zkapp: Some(zkapp),
                ..self
            };
        }

        self
    }

    /// Apply zkapp permissions diff
    pub fn zkapp_permissions(self, diff: &ZkappPermissionsDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            permissions: Some(diff.permissions.to_owned()),
            ..self
        }
    }

    /// Apply zkapp uri diff
    pub fn zkapp_uri(self, diff: &ZkappUriDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        if let Some(mut zkapp) = self.zkapp {
            zkapp.zkapp_uri = diff.zkapp_uri.to_owned();

            return Self {
                zkapp: Some(zkapp),
                ..self
            };
        }

        self
    }

    /// Apply zkapp token symbol diff
    pub fn zkapp_token_symbol(self, diff: &ZkappTokenSymbolDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            token_symbol: Some(diff.token_symbol.to_owned()),
            ..self
        }
    }

    /// Apply zkapp timing diff
    pub fn zkapp_timing(self, diff: &ZkappTimingDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            timing: Some(diff.timing.to_owned()),
            ..self
        }
    }

    /// Apply zkapp voting for diff
    pub fn zkapp_voting_for(self, diff: &ZkappVotingForDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            voting_for: Some(diff.voting_for.to_owned()),
            ..self
        }
    }

    /// Apply zkapp actions diff
    fn zkapp_actions(self, diff: &ZkappActionsDiff) -> Self {
        self.checks(&diff.public_key, &diff.token);

        if let Some(mut zkapp) = self.zkapp {
            let n = zkapp.action_state.len();

            for (idx, action_state) in diff.actions.iter().enumerate() {
                zkapp.action_state[idx % n] = action_state.to_owned();
            }

            return Self {
                zkapp: Some(zkapp),
                ..self
            };
        }

        self
    }

    /// Apply zkapp events diff
    fn zkapp_events(self, diff: &ZkappEventsDiff) -> Self {
        todo!("events diff {:?}", diff)
    }

    /// Apply zkapp increment
    pub fn zkapp_nonce(self, diff: &ZkappIncrementNonce) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            nonce: Some(self.nonce.unwrap_or_default() + 1),
            ..self
        }
    }

    /// Apply zkapp account creation fee
    pub fn zkapp_account_creation(self, diff: &ZkappAccountCreationFee) -> Self {
        self.checks(&diff.public_key, &diff.token);

        Self {
            balance: self.balance + diff.amount,
            ..self
        }
    }

    /// Apply an account diff to an account
    pub fn apply_account_diff(self, diff: &AccountDiff) -> Self {
        use AccountDiff::*;

        match diff {
            Payment(payment_diff) => self.payment(payment_diff),
            Delegation(delegation_diff) => {
                assert_eq!(self.public_key, delegation_diff.delegator);
                self.delegation(delegation_diff.delegate.clone(), delegation_diff.nonce)
            }
            Coinbase(coinbase_diff) => self.coinbase(coinbase_diff.amount),
            FeeTransfer(fee_transfer_diff) => self.payment(fee_transfer_diff),
            FeeTransferViaCoinbase(fee_transfer_diff) => self.payment(fee_transfer_diff),
            FailedTransactionNonce(failed_diff) => self.failed_transaction(failed_diff.nonce),
            ZkappStateDiff(diff) => self.zkapp_state(diff),
            ZkappPermissionsDiff(diff) => self.zkapp_permissions(diff),
            ZkappVerificationKeyDiff(diff) => self.zkapp_verification_key(diff),
            ZkappUriDiff(diff) => self.zkapp_uri(diff),
            ZkappTokenSymbolDiff(diff) => self.zkapp_token_symbol(diff),
            ZkappTimingDiff(diff) => self.zkapp_timing(diff),
            ZkappVotingForDiff(diff) => self.zkapp_voting_for(diff),
            ZkappActionsDiff(diff) => self.zkapp_actions(diff),
            ZkappEventsDiff(diff) => self.zkapp_events(diff),
            ZkappIncrementNonce(diff) => self.zkapp_nonce(diff),
            ZkappAccountCreationFee(diff) => self.zkapp_account_creation(diff),
            Zkapp(_) => unreachable!(),
        }
    }

    /// Unapply an account diff to an account
    pub fn unapply_account_diff(self, diff: &AccountDiff, remove: bool) -> Option<Self> {
        if remove {
            return None;
        }

        use AccountDiff::*;
        Some(match diff {
            Payment(payment_diff) => self.payment_unapply(payment_diff),
            Delegation(delegation_diff) => self.delegation_unapply(Some(delegation_diff.nonce)),
            Coinbase(coinbase_diff) => self.coinbase_unapply(coinbase_diff.amount),
            FeeTransfer(fee_transfer_diff) => self.payment_unapply(fee_transfer_diff),
            FeeTransferViaCoinbase(fee_transfer_diff) => self.payment_unapply(fee_transfer_diff),
            FailedTransactionNonce(diff) => self.failed_transaction_unapply(
                if diff.nonce.0 > 0 {
                    Some(diff.nonce - 1)
                } else {
                    None
                },
            ),
            Zkapp(zkapp_diff) => todo!("unapply zkapp account diff {zkapp_diff:?}"),
            _ => todo!(),
        })
    }

    /// Apply a ledger diff to an account
    pub fn apply_ledger_diff(self, diff: &LedgerDiff) -> Self {
        let pk = self.public_key.clone();
        let mut acct = self;
        for acct_diff in diff.account_diffs.iter().flatten() {
            if acct_diff.public_key() == pk {
                acct = acct.apply_account_diff(acct_diff);
            }
        }
        acct
    }

    /// Checks application to the expected token account
    fn checks(&self, public_key: &PublicKey, token: &TokenAddress) {
        assert_eq!(*public_key, self.public_key);
        self.check_token(token);
    }

    fn check_token(&self, token: &TokenAddress) {
        match self.token.as_ref() {
            None => assert_eq!(*token, TokenAddress::default()),
            Some(account_token) => assert_eq!(account_token, token),
        }
    }
}

impl From<GenesisBlock> for Account {
    fn from(value: GenesisBlock) -> Self {
        // magic mina
        let block_creator = value.0.block_creator();
        Account {
            public_key: block_creator.clone(),
            balance: Amount(1000_u64),
            delegate: block_creator,
            genesis_account: true,
            ..Default::default()
        }
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let balance_cmp = self.balance.cmp(&other.balance);
        if balance_cmp == std::cmp::Ordering::Equal {
            self.public_key.cmp(&other.public_key)
        } else {
            balance_cmp
        }
    }
}

/// Deduct account creation fee
impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let deducted = Account {
            balance: self.balance - MAINNET_ACCOUNT_CREATION_FEE,
            ..self.clone()
        };

        match serde_json::to_string_pretty(&deducted) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

/// Same as display
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::{Account, Amount};
    use crate::{
        constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
        ledger::{
            account::{Permission, Permissions, Timing},
            diff::account::{
                AccountDiff, PaymentDiff, UpdateType, ZkappDiff, ZkappPaymentDiff,
                ZkappPermissionsDiff, ZkappVerificationKeyDiff,
            },
            nonce::Nonce,
            public_key::PublicKey,
            token::{symbol::TokenSymbol, TokenAddress},
        },
        mina_blocks::v2::{AppState, VerificationKey, ZkappAccount, ZkappUri},
    };

    #[test]
    fn test_account_display() -> anyhow::Result<()> {
        let ledger_account = Account {
            balance: Amount::new(100),
            ..Default::default()
        };
        let deduct_account = ledger_account.clone().display();

        // account display & debug => deduct "creation fee"
        assert_eq!(
            format!("{ledger_account}"),
            serde_json::to_string_pretty(&deduct_account)?
        );
        assert_eq!(
            format!("{ledger_account:?}"),
            serde_json::to_string_pretty(&deduct_account)?
        );

        // same account display & debug
        assert_eq!(format!("{ledger_account}"), format!("{ledger_account:?}"));
        Ok(())
    }

    #[test]
    fn zkapp_account_diff_payment() {
        let amount = Amount(2000000000);
        let pk = PublicKey::from("B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5");

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            balance: amount,
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            public_key: pk.clone(),
            payment_diffs: vec![ZkappPaymentDiff::Payment(PaymentDiff {
                public_key: pk.clone(),
                update_type: UpdateType::Debit(Some(185.into())),
                amount,
                token: TokenAddress::default(),
            })],
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the balance & nonce change
        assert_eq!(
            after,
            Account {
                balance: before.balance - amount,
                nonce: Some(185.into()),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_app_state() {
        let pk = PublicKey::default();
        let app_state_elem: AppState =
            "0x1FFF56AAB5D3A09432146BC335714ABF14AA6DCCC2603B793E403E868B3383A4".into();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let mut app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM] =
            Default::default();
        app_state_diff[0] = Some(app_state_elem.clone());

        let diff = ZkappDiff {
            nonce: Nonce(1),
            increment_nonce: true,
            public_key: pk.clone(),
            payment_diffs: vec![],
            app_state_diff,
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the first app state element & nonce are modified
        let expect = {
            let mut app_state = before.zkapp.clone().unwrap().app_state;
            app_state[0] = app_state_elem;

            Account {
                zkapp: Some(ZkappAccount {
                    app_state,
                    ..before.zkapp.unwrap()
                }),
                ..before
            }
        };
        assert_eq!(after, expect);
    }

    #[test]
    fn zkapp_account_diff_delegate() {
        let pk = PublicKey::default();
        let delegate = PublicKey::from("B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5");
        let nonce = Nonce(1);

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce,
            delegate: Some(delegate.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the account delegate changes
        assert_eq!(
            after,
            Account {
                delegate,
                nonce: Some(nonce),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_verification_key() {
        let pk = PublicKey::default();
        let verification_key = VerificationKey {
            data: "VERIFICATION_KEY_DATA".into(),
            hash: "0xVDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into(),
        };

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = AccountDiff::ZkappVerificationKeyDiff(ZkappVerificationKeyDiff {
            token: TokenAddress::default(),
            public_key: pk.clone(),
            verification_key: verification_key.clone(),
        });

        // account after applying diff
        let after = before.clone().apply_account_diff(&diff);

        // only the zkapp verification key changes
        assert_eq!(
            after,
            Account {
                zkapp: Some(ZkappAccount {
                    verification_key,
                    ..before.zkapp.unwrap()
                }),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_permissions() {
        let pk = PublicKey::default();
        let permissions = Permissions {
            edit_state: Permission::Proof,
            ..Default::default()
        };

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
            token: TokenAddress::default(),
            public_key: pk.clone(),
            permissions: permissions.clone(),
        });

        // account after applying diff
        let after = before.clone().apply_account_diff(&diff);

        // only the account permissions changes
        assert_eq!(
            after,
            Account {
                permissions: Some(permissions),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_zkapp_uri() {
        let pk = PublicKey::default();
        let zkapp_uri = ZkappUri("ZKAPP_URI".to_string());

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Nonce(1),
            zkapp_uri: Some(zkapp_uri.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the zkapp uri changes
        assert_eq!(
            after,
            Account {
                zkapp: Some(ZkappAccount {
                    zkapp_uri,
                    ..before.zkapp.unwrap()
                }),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_token_symbol() {
        let pk = PublicKey::default();
        let token_symbol = TokenSymbol("TOKEN_SYMBOL".to_string());

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Nonce(1),
            increment_nonce: true,
            token_symbol: Some(token_symbol.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the zkapp token symbol changes
        assert_eq!(
            after,
            Account {
                token_symbol: Some(token_symbol),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_timing() {
        let pk = PublicKey::default();
        let timing = Timing::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Nonce(1),
            timing: Some(timing.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the account timing changes
        assert_eq!(
            after,
            Account {
                timing: Some(timing),
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_voting_for() {
        let pk = PublicKey::default();
        let voting_for = String::new();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Nonce(1),
            voting_for: Some(voting_for.to_owned().into()),
            public_key: pk.clone(),
            increment_nonce: true,
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff);
            }

            after
        };

        // only the account voting_for changes
        assert_eq!(
            after,
            Account {
                voting_for: Some(voting_for.to_owned().into()),
                ..before
            }
        );
    }
}
