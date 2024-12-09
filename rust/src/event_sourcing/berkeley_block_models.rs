use crate::constants::MAINNET_COINBASE_REWARD;
use serde::{Deserialize, Serialize};
use sonic_rs::{JsonValueTrait, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct BerkeleyBlock {
    pub version: u32,
    pub data: Data,
}

impl BerkeleyBlock {
    pub fn get_previous_state_hash(&self) -> String {
        self.data.protocol_state.previous_state_hash.clone()
    }

    pub fn get_last_vrf_output(&self) -> String {
        self.data.protocol_state.body.consensus_state.last_vrf_output.clone()
    }

    pub fn get_staged_ledger_pre_diff(&self) -> Option<Diff> {
        self.data.staged_ledger_diff.diff.first().and_then(|opt| opt.clone())
    }

    pub fn get_staged_ledger_post_diff(&self) -> Option<Diff> {
        self.data.staged_ledger_diff.diff.last().and_then(|opt| opt.clone())
    }

    pub fn get_user_commands_count(&self) -> usize {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff
                    .as_ref()
                    .map(|diff| diff.commands.iter().filter(|command| matches!(command, Command::SignedCommand(_))).count())
            })
            .sum()
    }

    pub fn get_zk_app_commands_count(&self) -> usize {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff
                    .as_ref()
                    .map(|diff| diff.commands.iter().filter(|command| matches!(command, Command::ZkappCommand(_))).count())
            })
            .sum()
    }

    pub fn get_coinbase_reward_nanomina(&self) -> u64 {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff.as_ref().and_then(|diff| match diff.coinbase.first() {
                    Some(v) if v == "Zero" => None,
                    _ => {
                        let multiplier = match self.data.protocol_state.body.consensus_state.supercharge_coinbase {
                            true => 2,
                            false => 1,
                        };
                        Some(multiplier * MAINNET_COINBASE_REWARD)
                    }
                })
            })
            .sum()
    }

    pub fn get_timestamp(&self) -> u64 {
        self.data.protocol_state.body.blockchain_state.timestamp.parse::<u64>().unwrap()
    }

    pub fn get_coinbase_receiver(&self) -> String {
        self.data.protocol_state.body.consensus_state.coinbase_receiver.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockchainState {
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Body {
    pub consensus_state: ConsensusState,
    pub blockchain_state: BlockchainState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    pub last_vrf_output: String,
    pub coinbase_receiver: String,
    pub supercharge_coinbase: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Diff {
    pub commands: Vec<Command>,
    pub coinbase: Vec<Value>,
}

#[derive(Serialize, Debug, Clone)]
pub enum Command {
    SignedCommand(SignedCommand),
    ZkappCommand(ZkappCommand),
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;

        if !value.is_object() {
            return Err(serde::de::Error::custom("Expected an object for Command"));
        }

        // Extract the "data" field and ensure it is a two-element array
        let data = value
            .get("data")
            .and_then(|v| v.clone().into_array())
            .ok_or_else(|| serde::de::Error::custom("Missing or invalid 'data' field"))?;

        if data.len() != 2 {
            return Err(serde::de::Error::custom("Expected 'data' field to have exactly 2 elements"));
        }

        // Extract the type and details
        let command_type = data[0]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("First element in 'data' must be a string"))?;
        let details = &data[1];

        // Match on the command type and deserialize appropriately
        match command_type {
            "Signed_command" => {
                let signed_command = sonic_rs::from_value::<SignedCommand>(details).map_err(serde::de::Error::custom)?;
                Ok(Command::SignedCommand(signed_command))
            }
            "Zkapp_command" => {
                let zkapp_command = sonic_rs::from_value::<ZkappCommand>(details).map_err(serde::de::Error::custom)?;
                Ok(Command::ZkappCommand(zkapp_command))
            }
            _ => Err(serde::de::Error::custom(format!("Unknown command type: {}", command_type))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedCommand {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkappCommand {}

#[cfg(test)]
mod berkeley_block_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_berkeley_block_summary_info() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/berkeley_blocks/berkeley-4969-3NL8QoLQMtsBH8vUnccQw3vt8PgYuZRMApq1yZT1jwhZjbBLMRJU.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into BerkeleyBlock struct
        let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        // Test user commands count
        assert_eq!(berkeley_block.get_user_commands_count(), 12, "User commands count should match");

        // Test zkApp commands count
        assert_eq!(berkeley_block.get_zk_app_commands_count(), 1, "zkApp commands count should match");

        assert_eq!(berkeley_block.get_timestamp(), 1708432621000);

        assert_eq!(
            berkeley_block.get_coinbase_receiver(),
            "B62qpfgnUm7zVqi8MJHNB2m37rtgMNDbFNhC2DpMmmVpQt8x6gKv9Ww",
            "Coinbase receiver should match"
        );

        assert_eq!(berkeley_block.get_coinbase_reward_nanomina(), 1_440_000_000_000, "Coinbase reward should match");
    }
}
