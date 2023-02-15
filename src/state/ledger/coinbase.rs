use crate::block_log::BlockLog;

use super::{PublicKey, diff::account::AccountDiff};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Coinbase {
    pub receiver: PublicKey,
    supercharge: bool
}

impl Coinbase {
    pub fn from_block_log(block_log: &BlockLog) -> Option<Self> {
        let consensus_state = block_log
            .json
            .as_object()?
            .get("protocol_state")?
            .as_object()?
            .get("body")?
            .as_object()?
            .get("consensus_state")?
            .as_object()?;

        let receiver = consensus_state
            .get("coinbase_receiver")?
            .as_str()?
            .to_string();

        let supercharge = consensus_state.get("supercharge_coinbase")?.as_bool()?;

        Some(Coinbase { receiver, supercharge })
    }

    pub fn as_account_diff(self) -> AccountDiff {
        AccountDiff::from_coinbase(self.receiver, self.supercharge)
    }
}