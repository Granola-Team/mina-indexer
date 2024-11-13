use crate::constants::MAINNET_COINBASE_REWARD;
use core::fmt;
use serde::{
    de::{SeqAccess, Visitor},
    Deserializer,
};
use sonic_rs::{Deserialize, Serialize, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct MainnetBlock {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,
}

impl MainnetBlock {
    pub fn get_block_creator(&self) -> String {
        self.protocol_state.body.consensus_state.block_creator.to_string()
    }

    pub fn get_previous_state_hash(&self) -> String {
        self.protocol_state.previous_state_hash.clone()
    }

    pub fn get_last_vrf_output(&self) -> String {
        self.protocol_state.body.consensus_state.last_vrf_output.to_string()
    }

    pub fn get_coinbase_receiver(&self) -> String {
        self.protocol_state.body.consensus_state.coinbase_receiver.to_string()
    }

    fn get_staged_ledger_pre_diff(&self) -> Option<Diff> {
        self.staged_ledger_diff.diff.first().and_then(|opt| opt.clone())
    }

    fn get_staged_ledger_post_diff(&self) -> Option<Diff> {
        self.staged_ledger_diff.diff.get(1).and_then(|opt| opt.clone())
    }

    pub fn get_coinbase_reward_nanomina(&self) -> u64 {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff.as_ref().and_then(|diff| match diff.coinbase.first() {
                    Some(v) if *v == "Zero" => None,
                    _ => {
                        let multiplier = match self.protocol_state.body.consensus_state.supercharge_coinbase {
                            true => 2,
                            false => 1,
                        };
                        Some(multiplier * MAINNET_COINBASE_REWARD)
                    }
                })
            })
            .sum()
    }

    pub fn get_snark_work_count(&self) -> usize {
        self.get_snark_work().len()
    }

    pub fn get_snark_work(&self) -> Vec<CompletedWorks> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.completed_works.clone()))
            .flat_map(|works| works.into_iter())
            .collect()
    }

    pub fn get_user_commands_count(&self) -> usize {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.commands.len()))
            .sum()
    }

    pub fn get_user_commands(&self) -> Vec<CommandSummary> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.commands.clone()))
            .flat_map(|user_commands| user_commands.into_iter())
            .map(|uc| uc.to_command_summary())
            .collect()
    }

    pub fn get_global_slot_since_genesis(&self) -> u64 {
        self.protocol_state.body.consensus_state.global_slot_since_genesis.parse::<u64>().unwrap()
    }

    pub fn get_timestamp(&self) -> u64 {
        self.protocol_state.body.blockchain_state.timestamp.parse::<u64>().unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: ProtocolStateBody,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolStateBody {
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
    pub block_creator: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletedWorks {
    pub fee: String,
    pub prover: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    #[serde(rename = "data", deserialize_with = "deserialize_signed_command")]
    pub signed_command: Option<SignedCommand>, // Directly parse as SignedCommand, or None if absent
    pub status: Vec<Value>,
}

impl Command {
    fn get_status(&self) -> String {
        self.status.first().unwrap().to_string().trim_matches('"').to_string()
    }

    fn get_nonce(&self) -> usize {
        self.signed_command
            .as_ref()
            .map(|sc| sc.payload.common.nonce.parse::<usize>().unwrap())
            .unwrap()
    }

    fn get_sender(&self) -> String {
        match self.signed_command.as_ref().unwrap().payload.body.clone() {
            Body::Payment(p) => p.source_pk.to_string(),
            Body::StakeDelegation(StakeDelegationType::SetDelegate(sd)) => sd.delegator.to_string(),
        }
    }

    fn get_type(&self) -> String {
        match self.signed_command.as_ref().unwrap().payload.body.clone() {
            Body::Payment(_) => "Payment".to_string(),
            Body::StakeDelegation(StakeDelegationType::SetDelegate(_)) => "Stake Delegation".to_string(),
        }
    }

    fn get_receiver(&self) -> String {
        match self.signed_command.as_ref().unwrap().payload.body.clone() {
            Body::Payment(p) => p.receiver_pk.to_string(),
            Body::StakeDelegation(StakeDelegationType::SetDelegate(sd)) => sd.new_delegate.to_string(),
        }
    }

    fn get_fee(&self) -> f64 {
        self.signed_command.as_ref().unwrap().payload.common.fee.parse::<f64>().unwrap()
    }

    fn get_amount_nanomina(&self) -> u64 {
        match self.signed_command.as_ref().unwrap().payload.body.clone() {
            Body::Payment(p) => p.amount.parse::<u64>().unwrap(),
            Body::StakeDelegation(StakeDelegationType::SetDelegate(_)) => 0,
        }
    }

    fn get_memo(&self) -> String {
        self.signed_command.as_ref().unwrap().payload.common.memo.to_string()
    }

    fn get_fee_payer(&self) -> String {
        self.signed_command.as_ref().unwrap().payload.common.fee_payer_pk.to_string()
    }

    pub fn to_command_summary(&self) -> CommandSummary {
        CommandSummary {
            memo: self.get_memo(),
            fee_payer: self.get_fee_payer(),
            sender: self.get_sender(),
            receiver: self.get_receiver(),
            status: self.get_status(),
            txn_type: self.get_type(),
            nonce: self.get_nonce(),
            fee_nanomina: (self.get_fee() * 1_000_000_000f64) as u64,
            amount_nanomina: self.get_amount_nanomina(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandSummary {
    pub memo: String,
    pub fee_payer: String,
    pub sender: String,
    pub receiver: String,
    pub status: String,
    pub txn_type: String,
    pub nonce: usize,
    pub fee_nanomina: u64,
    pub amount_nanomina: u64,
}

fn deserialize_signed_command<'de, D>(deserializer: D) -> Result<Option<SignedCommand>, D::Error>
where
    D: Deserializer<'de>,
{
    struct CommandDataVisitor;

    impl<'de> Visitor<'de> for CommandDataVisitor {
        type Value = Option<SignedCommand>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a list where the second item is a SignedCommand")
        }

        fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: SeqAccess<'de>,
        {
            // Skip the first element
            let _ = seq.next_element::<Value>()?;
            // Attempt to parse the second element as SignedCommand
            let second: Option<SignedCommand> = seq.next_element()?;
            Ok(second)
        }
    }

    deserializer.deserialize_seq(CommandDataVisitor)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedCommand {
    pub payload: Payload,
    pub signer: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub common: Common,
    #[serde(deserialize_with = "deserialize_body")]
    pub body: Body,
}

fn deserialize_body<'de, D>(deserializer: D) -> Result<Body, D::Error>
where
    D: Deserializer<'de>,
{
    struct BodyVisitor;

    impl<'de> Visitor<'de> for BodyVisitor {
        type Value = Body;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array with a command type string and a map of values or nested arrays")
        }

        fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: SeqAccess<'de>,
        {
            // Parse the command type as a string
            let command_type: String = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

            // Handle different command types based on structure
            match command_type.as_str() {
                "Payment" => {
                    let payment: Payment = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                    Ok(Body::Payment(payment))
                }
                "Stake_delegation" => {
                    // Parse additional nested array for Stake_delegation
                    let nested: Vec<Value> = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                    // Check if nested is an array and matches ["Set_delegate", { ... }]
                    // Assume structure ["Set_delegate", { "delegator": ..., "new_delegate": ... }]
                    let set_delegate = nested[1].clone();
                    let set_delegate: SetDelegate = sonic_rs::from_value(&set_delegate).map_err(serde::de::Error::custom)?;
                    Ok(Body::StakeDelegation(StakeDelegationType::SetDelegate(set_delegate)))
                }
                _ => Err(serde::de::Error::unknown_variant(&command_type, &["Payment", "Stake_delegation"])),
            }
        }
    }

    deserializer.deserialize_seq(BodyVisitor)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Body {
    Payment(Payment),
    StakeDelegation(StakeDelegationType),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub source_pk: String,
    pub receiver_pk: String,
    pub token_id: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum StakeDelegationType {
    #[serde(rename = "Set_delegate")]
    SetDelegate(SetDelegate),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetDelegate {
    pub delegator: String,
    pub new_delegate: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Common {
    pub fee: String,
    pub fee_token: String,
    pub fee_payer_pk: String,
    pub nonce: String,
    pub valid_until: String,
    pub memo: String,
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

        // Test block creator
        assert_eq!(&mainnet_block.get_block_creator(), "B62qjJ2eGwj1mmB6XThCV2m9JxUqJGXLqwyirxTbzBanzs2ThazD1Gy");

        let user_commands = mainnet_block.get_user_commands();
        let first_user_command = user_commands.first().unwrap();

        // Test Fee
        assert_eq!(first_user_command.fee_nanomina, 10_000_000);

        // Test memo
        assert_eq!(&first_user_command.memo, "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH");

        // Test memo
        assert_eq!(first_user_command.nonce, 265);

        // Test sender
        assert_eq!(&first_user_command.sender, "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");

        // Test receiver
        assert_eq!(&first_user_command.receiver, "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM");

        // test status
        assert_eq!(&first_user_command.status, "Applied");

        // test
        assert_eq!(first_user_command.amount_nanomina, 1000);
    }

    #[test]
    fn test_mainnet_block_with_stake_delegation() {
        // Path to your test JSON file
        let path = Path::new("./src/stream/test_data/misc_blocks/mainnet-199999-3NKDFcMG4gbdeSwEYzoAURSHv4uRabTFbTY7W6JpECN2rHmwYE8j.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(mainnet_block.get_user_commands_count(), 23);

        let user_commands = mainnet_block.get_user_commands();
        let fifth_user_command = user_commands.get(4).unwrap();

        // Test Fee
        assert_eq!(fifth_user_command.fee_nanomina, 10_100_000);

        // Test memo
        assert_eq!(&fifth_user_command.memo, "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH");

        // Test memo
        assert_eq!(fifth_user_command.nonce, 0);

        // Test sender
        assert_eq!(&fifth_user_command.sender, "B62qj2PMFaL2bmZQsWMfr2eiMxNErwUrZYKvt8JHgany2G3CvF6RGoc");

        // Test receiver
        assert_eq!(&fifth_user_command.receiver, "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6");

        // test status
        assert_eq!(&fifth_user_command.status, "Applied");

        // test
        assert_eq!(fifth_user_command.amount_nanomina, 0);
    }
}
