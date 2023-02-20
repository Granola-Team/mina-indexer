use crate::block_log::BlockLog;

use super::{diff::account::AccountDiff, PublicKey};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Coinbase {
    pub receiver: PublicKey,
    supercharge: bool,
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

        Some(Coinbase {
            receiver,
            supercharge,
        })
    }

    pub fn as_account_diff(self) -> AccountDiff {
        AccountDiff::from_coinbase(self.receiver, self.supercharge)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use tokio::{fs::File, io::AsyncReadExt};

    use crate::{block_log::BlockLog, state::ledger::coinbase::Coinbase};

    const BLOCK_LOG_STATE_HASH: &'static str =
        "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH";
    const BLOCK_LOG_PATH: &'static str =
        "./tests/data/block_logs/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";

    #[tokio::test]
    async fn from_block_log_deserializes() {
        let mut log_file = File::open(BLOCK_LOG_PATH)
            .await.expect("block log exists");
        let mut contents = Vec::new();
        log_file
            .read_to_end(&mut contents)
            .await.expect("block log doesn't IO Error");
        let str = unsafe { std::str::from_utf8_unchecked(&contents) };
        let json: Value = serde_json::from_str(str).expect("block log parses into a Value");

        let block_log = BlockLog { state_hash: BLOCK_LOG_STATE_HASH.to_string(), json };

        Coinbase::from_block_log(&block_log).expect("coinbase deserializes");
    }
}