use crate::block_log::PrecomputedBlock;

use super::{diff::account::AccountDiff, PublicKey};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Coinbase {
    pub receiver: PublicKey,
    supercharge: bool,
}

impl Coinbase {
    pub fn from_precomputed_block(precomputed_block: &PrecomputedBlock) -> Self {
        let consensus_state = precomputed_block
            .protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .consensus_state
            .inner()
            .inner();
        let receiver = consensus_state.coinbase_receiver.into();
        let supercharge = consensus_state.supercharge_coinbase;

        Self {
            receiver,
            supercharge,
        }
    }

    pub fn as_account_diff(self) -> AccountDiff {
        AccountDiff::from_coinbase(self.receiver.into(), self.supercharge)
    }
}
