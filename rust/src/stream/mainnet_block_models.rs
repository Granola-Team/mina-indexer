use crate::constants::MAINNET_COINBASE_REWARD;
use serde::{Deserialize, Serialize};
use sonic_rs::Value; // To handle arbitrary JSON objects

#[derive(Serialize, Deserialize, Debug)]
pub struct MainnetBlock {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: Option<StagedLedgerDiff>,
}

impl MainnetBlock {
    pub fn get_previous_state_hash(&self) -> String {
        self.protocol_state.previous_state_hash.clone()
    }

    pub fn get_last_vrf_output(&self) -> String {
        self.protocol_state.body.consensus_state.last_vrf_output.to_string()
    }

    pub fn get_coinbase_receiver(&self) -> String {
        self.protocol_state.body.consensus_state.coinbase_receiver.to_string()
    }

    pub fn get_coinbase_reward_nanomina(&self) -> u64 {
        self.staged_ledger_diff
            .as_ref()
            .map(|ledger_diff| {
                ledger_diff
                    .diff
                    .iter()
                    .take(2)
                    .filter_map(|opt_diff| {
                        opt_diff.as_ref().and_then(|diff| {
                            if diff.coinbase[0] != "Zero" {
                                if self.protocol_state.body.consensus_state.supercharge_coinbase {
                                    Some(2 * MAINNET_COINBASE_REWARD)
                                } else {
                                    Some(MAINNET_COINBASE_REWARD)
                                }
                            } else {
                                None
                            }
                        })
                    })
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn get_global_slot_since_genesis(&self) -> u64 {
        self.protocol_state.body.consensus_state.global_slot_since_genesis.parse::<u64>().unwrap()
    }

    pub fn get_snark_work_count(&self) -> usize {
        self.staged_ledger_diff
            .as_ref()
            .map(|ledger_diff| {
                ledger_diff
                    .diff
                    .iter()
                    .take(2)
                    .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.completed_works.len()))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn get_user_commands_count(&self) -> usize {
        self.staged_ledger_diff
            .as_ref()
            .map(|ledger_diff| {
                ledger_diff
                    .diff
                    .iter()
                    .take(2)
                    .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.commands.len()))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn get_timestamp(&self) -> u64 {
        self.protocol_state.body.blockchain_state.timestamp.parse::<u64>().unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Body {
    pub consensus_state: ConsensusState,
    pub blockchain_state: BlockchainState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockchainState {
    timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    pub last_vrf_output: String,
    pub global_slot_since_genesis: String,
    pub supercharge_coinbase: bool,
    pub coinbase_receiver: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Diff {
    pub commands: Vec<Command>,
    pub completed_works: Vec<CompletedWorks>,
    pub coinbase: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CoinbaseData {
    pub receiver_pk: String,
    pub fee: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompletedWorks {
    pub fee: String,
    pub prover: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub data: Vec<Value>, // Placeholder type to avoid parsing nested structures now
}

#[cfg(test)]
mod mainnet_block_parsing_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_mainnet_block_parsing() {
        // Path to your test JSON file
        let path = Path::new("./src/stream/test_data/misc_blocks/mainnet-185-3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        // Test global_slot_since_genesis
        assert_eq!(mainnet_block.get_global_slot_since_genesis(), 263);

        // Test coinbase reward
        assert_eq!(mainnet_block.get_coinbase_reward_nanomina(), 720_000_000_000);

        // Test user commands count
        assert_eq!(mainnet_block.get_user_commands_count(), 2);

        // Test snark work count
        assert_eq!(mainnet_block.get_snark_work_count(), 64);

        // Test parsing timestamp
        assert_eq!(mainnet_block.get_timestamp(), 1615986540000);

        // Test parsing coinbase receiver
        assert_eq!(
            &mainnet_block.get_coinbase_receiver(),
            "B62qjA7LFMvKuzFbGZK9yb3wAkBThba1pe5ap8UZx8jEvfAEcnDgDBE"
        );
    }
}
