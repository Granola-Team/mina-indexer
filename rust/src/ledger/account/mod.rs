//! Ledger account representation

mod receipt_chain_hash;
mod timing;
mod voting_for;

use super::{
    diff::{
        account::{
            zkapp::{
                ZkappActionsDiff, ZkappEventsDiff, ZkappFeePayerNonceDiff, ZkappIncrementNonce,
                ZkappPermissionsDiff, ZkappProvedStateDiff, ZkappStateDiff, ZkappTimingDiff,
                ZkappTokenSymbolDiff, ZkappUriDiff, ZkappVerificationKeyDiff, ZkappVotingForDiff,
            },
            AccountDiff, CoinbaseDiff, DelegationDiff, FailedTransactionNonceDiff, UpdateType,
        },
        LedgerDiff,
    },
    token::{account::TokenAccount, TokenAddress, TokenSymbol},
    username::Username,
};
use crate::{
    base::{amount::Amount, nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
    block::genesis::GenesisBlock,
    constants::{MAINNET_ACCOUNT_CREATION_FEE, MINA_TOKEN_ADDRESS},
    ledger::diff::account::PaymentDiff,
    mina_blocks::v2::{self, ZkappAccount},
};
use log::{error, warn};
use mina_serialization_proc_macros::AutoFrom;
use serde::{Deserialize, Serialize};

// re-export types
pub type ReceiptChainHash = receipt_chain_hash::ReceiptChainHash;
pub type VotingFor = voting_for::VotingFor;
pub type Timing = timing::Timing;

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub public_key: PublicKey,
    pub balance: Amount,
    pub delegate: PublicKey,
    pub genesis_account: Option<Amount>,
    pub created_by_zkapp: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<Nonce>,

    // optional
    pub token: Option<TokenAddress>,
    pub receipt_chain_hash: Option<ReceiptChainHash>,
    pub voting_for: Option<VotingFor>,
    pub permissions: Option<Permissions>,
    pub timing: Option<Timing>,
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

//////////
// impl //
//////////

impl Account {
    /// Deduct the account creation fee if necessary
    ///
    /// Removes non-genesis, non-zkapp, MINA account creation fee
    pub fn deduct_mina_account_creation_fee(self) -> Self {
        if self
            .token
            .as_ref()
            .is_none_or(|t| t.0 == MINA_TOKEN_ADDRESS)
            && !self.created_by_zkapp
        {
            return Self {
                balance: self.balance - MAINNET_ACCOUNT_CREATION_FEE,
                ..self
            };
        }

        self
    }

    /// Time-locked balance (subtracted from circulating supply)
    /// as per https://docs.minaprotocol.com/mina-protocol/time-locked-accounts
    pub fn current_minimum_balance(&self, curr_global_slot: u32) -> u64 {
        self.timing.as_ref().map_or(0, |t| {
            if curr_global_slot < t.cliff_time.0 {
                t.initial_minimum_balance.0
            } else {
                t.initial_minimum_balance.0.saturating_sub(
                    ((curr_global_slot - t.cliff_time.0) / t.vesting_period.0) as u64
                        * t.vesting_increment.0,
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
    pub fn empty(public_key: PublicKey, token: TokenAddress, created_by_zkapp: bool) -> Self {
        Self {
            public_key: public_key.clone(),
            delegate: public_key,
            token: Some(token),
            created_by_zkapp,
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
        Self {
            balance: self.balance + amount,
            ..self
        }
    }

    /// Unapply a coinbase
    pub fn coinbase_unapply(self, diff: &CoinbaseDiff) -> Self {
        Self {
            balance: self.balance - diff.amount,
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
    pub fn payment_unapply(self, diff: &PaymentDiff) -> Self {
        match diff.update_type {
            UpdateType::Credit => Self {
                balance: self.balance - diff.amount,
                ..self
            },
            UpdateType::Debit(nonce) => Self {
                balance: self.balance + diff.amount,
                nonce: nonce.map_or(self.nonce, |nonce| {
                    if self.nonce.map(|n| n.0) == Some(0) {
                        None
                    } else {
                        Some(nonce - 1)
                    }
                }),
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
        Self {
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
        Self {
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
        Self {
            delegate,
            nonce: Some(updated_nonce),
            ..self
        }
    }

    /// Unapply a delegation
    pub fn delegation_unapply(self, diff: &DelegationDiff) -> Self {
        Self {
            nonce: if self.nonce.map(|n| n.0) == Some(0) {
                None
            } else {
                Some(diff.nonce - 1)
            },
            delegate: self.public_key.clone(),
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
        Self {
            nonce: Some(updated_nonce),
            ..self
        }
    }

    /// Unapply a failed transaction
    pub fn failed_transaction_unapply(self, diff: &FailedTransactionNonceDiff) -> Self {
        let nonce = if diff.nonce.0 > 0 {
            Some(diff.nonce - 1)
        } else {
            None
        };

        Self { nonce, ..self }
    }

    /// Checks whether account is a zkapp account
    pub fn is_zkapp_account(&self) -> bool {
        self.zkapp.is_some()
    }

    /////////////////////////
    // zkapp account diffs //
    /////////////////////////

    /// Apply zkapp state diff
    pub fn zkapp_state(self, diff: &ZkappStateDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        let mut zkapp = self.zkapp.unwrap_or_default();

        // modify app state
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
    pub fn zkapp_verification_key(
        self,
        diff: &ZkappVerificationKeyDiff,
        state_hash: &StateHash,
    ) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        let mut zkapp = self.zkapp.unwrap_or_default();

        // modify verification key
        zkapp.verification_key = diff.verification_key.to_owned();

        Self {
            zkapp: Some(zkapp),
            ..self
        }
    }

    /// Apply zkapp verification key diff
    pub fn zkapp_proved_state(self, diff: &ZkappProvedStateDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        let mut zkapp = self.zkapp.unwrap_or_default();

        // modify proved state
        zkapp.proved_state = zkapp.proved_state || diff.proved_state;

        Self {
            zkapp: Some(zkapp),
            ..self
        }
    }

    /// Apply zkapp permissions diff
    pub fn zkapp_permissions(self, diff: &ZkappPermissionsDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        Self {
            permissions: Some(diff.permissions.to_owned()),
            ..self
        }
    }

    /// Apply zkapp uri diff
    pub fn zkapp_uri(self, diff: &ZkappUriDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        let mut zkapp = self.zkapp.unwrap_or_default();

        // modify zkapp uri
        zkapp.zkapp_uri = diff.zkapp_uri.to_owned();

        Self {
            zkapp: Some(zkapp),
            ..self
        }
    }

    /// Apply zkapp token symbol diff
    pub fn zkapp_token_symbol(self, diff: &ZkappTokenSymbolDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        Self {
            token_symbol: Some(diff.token_symbol.to_owned()),
            ..self
        }
    }

    /// Apply zkapp timing diff
    pub fn zkapp_timing(self, diff: &ZkappTimingDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        Self {
            timing: Some(diff.timing.to_owned()),
            ..self
        }
    }

    /// Apply zkapp voting for diff
    pub fn zkapp_voting_for(self, diff: &ZkappVotingForDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        Self {
            voting_for: Some(diff.voting_for.to_owned()),
            ..self
        }
    }

    /// Apply zkapp actions diff
    fn zkapp_actions(self, diff: &ZkappActionsDiff, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        let mut zkapp = self.zkapp.unwrap_or_default();

        // modify action state
        let n = zkapp.action_state.len();
        for (idx, action_state) in diff.actions.iter().enumerate() {
            zkapp.action_state[idx % n] = action_state.to_owned();
        }

        Self {
            zkapp: Some(zkapp),
            ..self
        }
    }

    /// Apply zkapp events diff
    fn zkapp_events(self, _diff: &ZkappEventsDiff, _state_hash: &StateHash) -> Self {
        // todo!("events diff {:?}", diff)
        self
    }

    /// Apply zkapp nonce increment
    pub fn zkapp_nonce(self, diff: &ZkappIncrementNonce, state_hash: &StateHash) -> Self {
        self.checks(&diff.public_key, &diff.token, state_hash);

        Self {
            nonce: Some(self.nonce.unwrap_or_default() + 1),
            ..self
        }
    }

    /// Apply zkapp fee payer nonce
    pub fn zkapp_fee_payer_nonce(
        self,
        diff: &ZkappFeePayerNonceDiff,
        state_hash: &StateHash,
    ) -> Self {
        self.check_pk(&diff.public_key, state_hash);
        // assert!(self.nonce.as_ref().map_or(true, |n| *n <= diff.nonce));

        Self {
            nonce: Some(diff.nonce),
            ..self
        }
    }

    /// Apply an account diff to an account
    pub fn apply_account_diff(
        self,
        diff: &AccountDiff,
        block_height: u32,
        state_hash: &StateHash,
    ) -> Self {
        use AccountDiff::*;

        let pk = diff.public_key();

        if PKS_OF_INTEREST.contains(&(&pk.0 as &str)) {
            let summary = format!("mainnet-{}-{}", block_height, state_hash);
            warn!("APPLY {}\n{}\nBEFORE: {}", summary, diff, self)
        }

        let after = match diff {
            Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => self.payment(diff),
            Delegation(delegation_diff) => {
                assert_eq!(self.public_key, delegation_diff.delegator);
                self.delegation(delegation_diff.delegate.clone(), delegation_diff.nonce)
            }
            Coinbase(coinbase_diff) => self.coinbase(coinbase_diff.amount),
            FailedTransactionNonce(failed_diff) => self.failed_transaction(failed_diff.nonce),
            ZkappStateDiff(diff) => self.zkapp_state(diff, state_hash),
            ZkappPermissionsDiff(diff) => self.zkapp_permissions(diff, state_hash),
            ZkappVerificationKeyDiff(diff) => self.zkapp_verification_key(diff, state_hash),
            ZkappProvedStateDiff(diff) => self.zkapp_proved_state(diff, state_hash),
            ZkappUriDiff(diff) => self.zkapp_uri(diff, state_hash),
            ZkappTokenSymbolDiff(diff) => self.zkapp_token_symbol(diff, state_hash),
            ZkappTimingDiff(diff) => self.zkapp_timing(diff, state_hash),
            ZkappVotingForDiff(diff) => self.zkapp_voting_for(diff, state_hash),
            ZkappActionsDiff(diff) => self.zkapp_actions(diff, state_hash),
            ZkappEventsDiff(diff) => self.zkapp_events(diff, state_hash),
            ZkappIncrementNonce(diff) => self.zkapp_nonce(diff, state_hash),
            ZkappFeePayerNonce(diff) => self.zkapp_fee_payer_nonce(diff, state_hash),
            Zkapp(_) => unreachable!(),
        };

        if PKS_OF_INTEREST.contains(&(&pk.0 as &str)) {
            warn!("AFTER: {}", after)
        }

        after
    }

    /// Unapply an account diff to an account
    pub fn unapply_account_diff(
        self,
        diff: &AccountDiff,
        block_height: u32,
        state_hash: &StateHash,
        remove: bool,
    ) -> Option<Self> {
        use AccountDiff::*;

        let pk = diff.public_key();
        let before = self.clone();

        if remove {
            if PKS_OF_INTEREST.contains(&(&pk.0 as &str)) {
                let summary = format!("mainnet-{}-{}", block_height, state_hash);
                warn!("UNAPPLY {}\nDIFF: {}\nREMOVE {}", summary, diff, pk)
            }

            return None;
        }

        let after = match diff {
            Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => {
                self.payment_unapply(diff)
            }
            Delegation(diff) => self.delegation_unapply(diff),
            Coinbase(diff) => self.coinbase_unapply(diff),
            FailedTransactionNonce(diff) => self.failed_transaction_unapply(diff),

            // TODO zkapp unapply
            ZkappStateDiff(_)
            | ZkappPermissionsDiff(_)
            | ZkappVerificationKeyDiff(_)
            | ZkappProvedStateDiff(_)
            | ZkappUriDiff(_)
            | ZkappTokenSymbolDiff(_)
            | ZkappTimingDiff(_)
            | ZkappVotingForDiff(_)
            | ZkappActionsDiff(_)
            | ZkappEventsDiff(_)
            | ZkappIncrementNonce(_)
            | ZkappFeePayerNonce(_) => {
                if PKS_OF_INTEREST.contains(&(&pk.0 as &str)) {
                    let summary = format!("mainnet-{}-{}", block_height, state_hash);
                    warn!("UNIMPLEMENTED UNAPPLY {}\nDIFF: {}", summary, diff)
                }

                self
            }
            Zkapp(_) => unreachable!(),
        };

        if PKS_OF_INTEREST.contains(&(&pk.0 as &str)) {
            let summary = format!("mainnet-{}-{}", block_height, state_hash);
            warn!(
                "UNAPPLY {}\nDIFF: {}\nBEFORE: {}\nAFTER: {}",
                summary, diff, before, after
            )
        }

        Some(after)
    }

    /// Apply a ledger diff to an account
    pub fn apply_ledger_diff(self, diff: &LedgerDiff) -> Self {
        let pk = self.public_key.clone();
        let mut acct = self;

        for acct_diff in diff.account_diffs.iter().flatten() {
            if acct_diff.public_key() == pk {
                acct = acct.apply_account_diff(acct_diff, diff.blockchain_length, &diff.state_hash);
            }
        }

        acct
    }

    /// Checks application to the expected token account
    fn checks(&self, pk: &PublicKey, token: &TokenAddress, state_hash: &StateHash) {
        self.check_pk(pk, state_hash);
        self.check_token(token, state_hash);
    }

    fn check_pk(&self, pk: &PublicKey, state_hash: &StateHash) {
        if self.public_key != *pk {
            error!(
                "Public key mismatch in '{}' zkapp account diff: {} /= {}",
                state_hash, self.public_key, pk
            );
        }
    }

    fn check_token(&self, token: &TokenAddress, state_hash: &StateHash) {
        let msg = |acct_token: Option<&TokenAddress>| -> String {
            let acct_token_str = acct_token
                .as_ref()
                .map_or("null".to_string(), |t| t.to_string());

            format!(
                "Token mismatch in '{}' zkapp account diff: {} /= {}",
                state_hash, acct_token_str, token
            )
        };

        if self.token.is_none() && token.0 != MINA_TOKEN_ADDRESS
            || self.token.as_ref() != Some(token)
        {
            error!("{}", msg(self.token.as_ref()))
        }
    }
}

//////////////
// ordering //
//////////////

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Account {
    /// Order by `(balance, public key)`
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let balance_cmp = self.balance.cmp(&other.balance);

        if balance_cmp == std::cmp::Ordering::Equal {
            self.public_key.cmp(&other.public_key)
        } else {
            balance_cmp
        }
    }
}

/////////////////
// conversions //
/////////////////

impl Account {
    /// Magic mina for genesis block creator
    pub fn from_genesis(block: GenesisBlock) -> Self {
        let block_creator = block.0.block_creator();
        let balance = 1000.into();

        Self {
            balance,
            public_key: block_creator.clone(),
            delegate: block_creator,
            genesis_account: Some(balance),
            token: Some(TokenAddress::default()),
            ..Default::default()
        }
    }
}

///////////////////
// debug/display //
///////////////////

/// Deduct account creation fee
impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let deducted = Self::deduct_mina_account_creation_fee(self.to_owned());

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

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Permission::None => "None",
                Permission::Either => "Either",
                Permission::Proof => "Proof",
                Permission::Signature => "Signature",
                Permission::Impossible => "Impossible",
            }
        )
    }
}

const PKS_OF_INTEREST: [&str; 7] = [
    "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg",
    "B62qmYFLwGSjQuWdnygPLw5TvrMENrLEFQmTow8RhSUw6MCm2sjQEn9",
    "B62qpRMGRzqFVACxPS5AozHp3HxPqwkfrqjJFaPk6KL6Kf9PpaLoXvY",
    "B62qpSSC6FUVbMCBzY69JqxtMM52dWzHaFRHTZY7BQrA9X59e2cAPXH",
    "B62qrYu6vakNxZQH6FeQxBoGNgME7u35Wswqh53YEFPUVr7HwNrhiQC",
    "B62qnUmeSPTktwags2faJTo48nTa4h8yU1cpEuagX3Mb4M74NLi2x9m",
    "B62qp7u4rgGLz35bSpNRWW74DmEjeWQG5jG9BGzjfa4t5KBevGY6GAh",
];

#[cfg(test)]
mod tests {
    use super::{Account, Amount};
    use crate::{
        base::{nonce::Nonce, public_key::PublicKey, state_hash::StateHash},
        constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
        ledger::{
            account::{Permission, Permissions, Timing},
            diff::account::{
                zkapp::{
                    ZkappDiff, ZkappPaymentDiff, ZkappPermissionsDiff, ZkappProvedStateDiff,
                    ZkappVerificationKeyDiff,
                },
                AccountDiff, PaymentDiff, UpdateType,
            },
            token::{TokenAddress, TokenSymbol},
        },
        mina_blocks::v2::{AppState, VerificationKey, ZkappAccount, ZkappUri},
    };

    #[test]
    fn test_mina_account_display() -> anyhow::Result<()> {
        let ledger_account = Account {
            balance: Amount::new(100),
            ..Default::default()
        };
        let deduct_account = ledger_account.clone().deduct_mina_account_creation_fee();

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
    fn test_non_mina_account_display() -> anyhow::Result<()> {
        let ledger_account = Account {
            balance: Amount::new(100),
            token: Some(
                TokenAddress::new("wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd").unwrap(),
            ),
            ..Default::default()
        };

        // account display & debug => deduct "creation fee"
        assert_eq!(
            format!("{ledger_account}"),
            serde_json::to_string_pretty(&ledger_account)?
        );
        assert_eq!(
            format!("{ledger_account:?}"),
            serde_json::to_string_pretty(&ledger_account)?
        );

        // same account display & debug
        assert_eq!(format!("{ledger_account}"), format!("{ledger_account:?}"));
        Ok(())
    }

    #[test]
    fn zkapp_account_diff_payment() {
        let amount = Amount(2000000000);
        let pk = PublicKey::from("B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5");

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            balance: amount,
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            public_key: pk.clone(),
            nonce: Some(185.into()),
            payment_diffs: vec![ZkappPaymentDiff::Payment(PaymentDiff {
                public_key: pk.clone(),
                update_type: UpdateType::Debit(None),
                amount,
                token: TokenAddress::default(),
            })],
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
            }

            after
        };

        // only the balance changes
        assert_eq!(
            after,
            Account {
                balance: before.balance - amount,
                ..before
            }
        );
    }

    #[test]
    fn zkapp_account_diff_app_state() {
        let pk = PublicKey::default();
        let app_state_elem: AppState =
            "0x1FFF56AAB5D3A09432146BC335714ABF14AA6DCCC2603B793E403E868B3383A4".into();

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let mut app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM] =
            Default::default();
        app_state_diff[0] = Some(app_state_elem.clone());

        let zkapp_diff = ZkappDiff {
            nonce: Some(Nonce(1)),
            increment_nonce: true,
            public_key: pk.clone(),
            payment_diffs: vec![],
            app_state_diff,
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in zkapp_diff.clone().expand().iter() {
                after = after.apply_account_diff(diff, block_height, &state_hash);
            }

            after
        };

        // only the first app state element is modified
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

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Some(nonce),
            delegate: Some(delegate.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
            }

            after
        };

        // only the account delegate & nonce changes
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

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diffs = vec![
            AccountDiff::ZkappVerificationKeyDiff(ZkappVerificationKeyDiff {
                token: TokenAddress::default(),
                public_key: pk.clone(),
                verification_key: verification_key.clone(),
            }),
            AccountDiff::ZkappProvedStateDiff(ZkappProvedStateDiff {
                token: TokenAddress::default(),
                public_key: pk.clone(),
                proved_state: true,
            }),
        ];

        // account after applying diffs
        let after = {
            let mut acct = before.clone();

            for diff in diffs {
                acct = acct.apply_account_diff(&diff, block_height, &state_hash);
            }

            acct
        };

        // only the zkapp verification key & proved state change
        assert_eq!(
            after,
            Account {
                zkapp: Some(ZkappAccount {
                    verification_key,
                    proved_state: true,
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

        let block_height = 0;
        let state_hash = StateHash::default();

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
        let after = before
            .clone()
            .apply_account_diff(&diff, block_height, &state_hash);

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

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Some(Nonce(1)),
            zkapp_uri: Some(zkapp_uri.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
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
        let token_symbol = TokenSymbol::from("TOKEN_SYMBOL");

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Some(Nonce(1)),
            increment_nonce: true,
            token_symbol: Some(token_symbol.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
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

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Some(Nonce(1)),
            timing: Some(timing.clone()),
            public_key: pk.clone(),
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
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

        let block_height = 0;
        let state_hash = StateHash::default();

        // account before applying diff
        let before = Account {
            public_key: pk.clone(),
            zkapp: Some(ZkappAccount::default()),
            ..Default::default()
        };

        let diff = ZkappDiff {
            nonce: Some(Nonce(1)),
            voting_for: Some(voting_for.to_owned().into()),
            public_key: pk.clone(),
            increment_nonce: true,
            ..Default::default()
        };

        // account after applying diff
        let after = {
            let mut after = before.clone();

            for diff in diff.expand() {
                after = after.apply_account_diff(&diff, block_height, &state_hash);
            }

            after
        };

        // only the account voting_for changes
        assert_eq!(
            after,
            Account {
                voting_for: Some(voting_for.into()),
                ..before
            }
        );
    }
}
