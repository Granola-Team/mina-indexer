use super::models::{CommandStatus, CommandSummary, CommandType, CompletedWorksNanomina};
use crate::{constants::MAINNET_COINBASE_REWARD, utility::decode_base58check_to_string};
use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use sonic_rs::{JsonValueTrait, Value};
use std::{collections::HashMap, fmt, str::FromStr};

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
                opt_diff.as_ref().map(|diff| {
                    diff.commands
                        .iter()
                        .filter(|wrapper| matches!(wrapper.command, Command::SignedCommand(_)))
                        .count()
                })
            })
            .sum()
    }

    pub fn get_user_commands(&self) -> Vec<CommandSummary> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff
                    .as_ref()
                    .map(|diff| diff.commands.iter().filter(|wrapper| matches!(wrapper.command, Command::SignedCommand(_))))
            })
            .flat_map(|user_commands| user_commands.into_iter())
            .map(|wrapper| wrapper.to_command_summary())
            .collect()
    }

    pub fn get_zk_app_commands_count(&self) -> usize {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff.as_ref().map(|diff| {
                    diff.commands
                        .iter()
                        .filter(|wrapper| matches!(wrapper.command, Command::ZkappCommand(_)))
                        .count()
                })
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

    pub fn get_snark_work_count(&self) -> usize {
        self.get_snark_work().len()
    }

    pub fn get_snark_work(&self) -> Vec<CompletedWorksNanomina> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.completed_works.clone()))
            .flat_map(|works| {
                works.into_iter().map(|work| {
                    let fee_nanomina = BigDecimal::from_str(&work.fee).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                    CompletedWorksNanomina {
                        fee_nanomina: fee_nanomina.to_u64().unwrap(),
                        prover: work.prover.to_string(),
                    }
                })
            })
            .collect()
    }

    pub fn get_aggregated_snark_work(&self) -> Vec<CompletedWorksNanomina> {
        let mut aggregated_snark_work: HashMap<String, u64> = HashMap::new();

        for completed_work in self.get_snark_work() {
            *aggregated_snark_work.entry(completed_work.prover.clone()).or_insert(0) += completed_work.fee_nanomina;
        }

        aggregated_snark_work
            .into_iter()
            .map(|(prover, fee_nanomina)| CompletedWorksNanomina {
                prover: prover.to_string(),
                fee_nanomina,
            })
            .collect()
    }

    pub fn get_timestamp(&self) -> u64 {
        self.data.protocol_state.body.blockchain_state.timestamp.parse::<u64>().unwrap()
    }

    pub fn get_coinbase_receiver(&self) -> String {
        self.data.protocol_state.body.consensus_state.coinbase_receiver.to_string()
    }

    pub fn get_global_slot_since_genesis(&self) -> u64 {
        self.data.protocol_state.body.consensus_state.global_slot_since_genesis.parse::<u64>().unwrap()
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
    pub body: ProtocolStateBody,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockchainState {
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolStateBody {
    pub consensus_state: ConsensusState,
    pub blockchain_state: BlockchainState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    pub last_vrf_output: String,
    pub coinbase_receiver: String,
    pub supercharge_coinbase: bool,
    pub global_slot_since_genesis: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Diff {
    pub completed_works: Vec<CompletedWorks>,
    pub commands: Vec<CommandWrapper>,
    pub coinbase: Vec<Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CommandWrapper {
    pub command: Command,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletedWorks {
    pub fee: String,
    pub prover: String,
}

#[derive(Serialize, Debug, Clone)]
pub enum Command {
    SignedCommand(SignedCommand),
    ZkappCommand(ZkappCommand),
}

impl CommandWrapper {
    pub fn get_status(&self) -> String {
        self.status.to_string()
    }

    fn get_nonce(&self) -> usize {
        match &self.command {
            Command::SignedCommand(signed_command) => signed_command.payload.common.nonce.parse::<usize>().unwrap(),
            Command::ZkappCommand(_) => todo!("get_nonce not implemented for ZkappCommand"),
        }
    }

    fn get_sender(&self) -> String {
        String::from("Unknown")
    }

    fn get_type(&self) -> CommandType {
        match &self.command {
            Command::SignedCommand(signed_command) => match &signed_command.payload.body {
                Body::Payment(_) => CommandType::Payment,
                Body::StakeDelegation(_) => CommandType::StakeDelegation,
            },
            Command::ZkappCommand(_) => todo!("get_type not implemented for ZkappCommand"),
        }
    }

    fn get_receiver(&self) -> String {
        match &self.command {
            Command::SignedCommand(signed_command) => match &signed_command.payload.body {
                Body::Payment(payment) => payment.receiver_pk.clone(),
                Body::StakeDelegation(StakeDelegationType::SetDelegate(delegate)) => delegate.new_delegate.clone(),
            },
            Command::ZkappCommand(_) => todo!("get_receiver not implemented for ZkappCommand"),
        }
    }

    fn get_fee(&self) -> f64 {
        match &self.command {
            Command::SignedCommand(signed_command) => signed_command.payload.common.fee.parse::<f64>().unwrap(),
            Command::ZkappCommand(_) => todo!("get_fee not implemented for ZkappCommand"),
        }
    }

    fn get_amount_nanomina(&self) -> u64 {
        match &self.command {
            Command::SignedCommand(signed_command) => match &signed_command.payload.body {
                Body::Payment(payment) => payment.amount.parse::<u64>().unwrap(),
                Body::StakeDelegation(_) => 0,
            },
            Command::ZkappCommand(_) => todo!("get_amount_nanomina not implemented for ZkappCommand"),
        }
    }

    fn get_memo(&self) -> String {
        match &self.command {
            Command::SignedCommand(signed_command) => decode_base58check_to_string(&signed_command.payload.common.memo).unwrap(),
            Command::ZkappCommand(_) => todo!("get_memo not implemented for ZkappCommand"),
        }
    }

    fn get_fee_payer(&self) -> String {
        match &self.command {
            Command::SignedCommand(signed_command) => signed_command.payload.common.fee_payer_pk.clone(),
            Command::ZkappCommand(_) => todo!("get_fee_payer not implemented for ZkappCommand"),
        }
    }

    pub fn to_command_summary(&self) -> CommandSummary {
        match &self.command {
            Command::SignedCommand(_) => CommandSummary {
                memo: self.get_memo(),
                fee_payer: self.get_fee_payer(),
                sender: self.get_sender(),
                receiver: self.get_receiver(),
                status: match self.get_status().as_str() {
                    "Applied" => CommandStatus::Applied,
                    _ => CommandStatus::Failed,
                },
                txn_type: self.get_type(),
                nonce: self.get_nonce(),
                fee_nanomina: (self.get_fee() * 1_000_000_000f64) as u64,
                amount_nanomina: self.get_amount_nanomina(),
            },
            Command::ZkappCommand(_) => todo!("to_command_summary not implemented for ZkappCommand"),
        }
    }
}

impl<'de> Deserialize<'de> for CommandWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;

        if !value.is_object() {
            return Err(serde::de::Error::custom("Expected an object for CommandWrapper"));
        }

        // Extract the "data" field
        let data = value
            .get("data")
            .and_then(|v| v.clone().into_array())
            .ok_or_else(|| serde::de::Error::custom("Missing or invalid 'data' field"))?;

        if data.len() != 2 {
            return Err(serde::de::Error::custom("Expected 'data' field to have exactly 2 elements"));
        }

        let command_type = data[0]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("First element in 'data' must be a string"))?;
        let details = &data[1];

        // Deserialize the `Command`
        let command = match command_type {
            "Signed_command" => {
                let signed_command = sonic_rs::from_value::<SignedCommand>(details).map_err(serde::de::Error::custom)?;
                Ok(Command::SignedCommand(signed_command))
            }
            "Zkapp_command" => {
                let zkapp_command = sonic_rs::from_value::<ZkappCommand>(details).map_err(serde::de::Error::custom)?;
                Ok(Command::ZkappCommand(zkapp_command))
            }
            _ => Err(serde::de::Error::custom(format!("Unknown command type: {}", command_type))),
        }?;

        // Extract the "status" field
        let status_arr = value
            .get("status")
            .and_then(|v| v.clone().into_array())
            .ok_or_else(|| serde::de::Error::custom("Missing or invalid 'status' field"))?;

        let status = status_arr.first().unwrap();

        Ok(CommandWrapper {
            command,
            status: status.as_str().unwrap().to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedCommand {
    pub payload: Payload,
    #[serde(skip)]
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub common: Common,
    #[serde(deserialize_with = "deserialize_body")]
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Common {
    pub fee: String,
    pub fee_payer_pk: String,
    pub nonce: String,
    pub valid_until: String,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Body {
    StakeDelegation(StakeDelegationType),
    Payment(Payment),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StakeDelegationType {
    SetDelegate(DelegateInfo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegateInfo {
    pub new_delegate: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub receiver_pk: String,
    pub amount: String,
}

fn deserialize_body<'de, D>(deserializer: D) -> Result<Body, D::Error>
where
    D: Deserializer<'de>,
{
    struct BodyVisitor;

    impl<'de> Visitor<'de> for BodyVisitor {
        type Value = Body;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array with a command type string and a nested structure")
        }

        fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: SeqAccess<'de>,
        {
            // Parse the command type as a string
            let command_type: String = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;

            // Handle specific command types
            match command_type.as_str() {
                "Stake_delegation" => {
                    let nested: Vec<Value> = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;

                    // Assume structure ["Set_delegate", { "new_delegate": ... }]
                    if let Some(set_delegate_array) = nested.get(1) {
                        let delegate_info: DelegateInfo = sonic_rs::from_value(set_delegate_array).map_err(de::Error::custom)?;
                        Ok(Body::StakeDelegation(StakeDelegationType::SetDelegate(delegate_info)))
                    } else {
                        Err(de::Error::invalid_value(de::Unexpected::Seq, &"Expected nested Set_delegate structure"))
                    }
                }
                "Payment" => {
                    // Parse Payment structure
                    let payment: Payment = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    Ok(Body::Payment(payment))
                }
                _ => Err(de::Error::unknown_variant(&command_type, &["Stake_delegation", "Payment"])),
            }
        }
    }

    deserializer.deserialize_seq(BodyVisitor)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkappCommand {}

#[cfg(test)]
mod berkeley_block_tests {
    use super::*;
    use crate::utility::get_cleaned_pcb;
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

        assert_eq!(berkeley_block.get_global_slot_since_genesis(), 8612, "Global slot since genesis should match");

        assert_eq!(berkeley_block.get_snark_work_count(), 0, "snark work count should match");
    }

    #[test]
    fn test_berkeley_block_409021() {
        // Path to your test JSON file
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-409021-3NLWau54pjGtX98RyvEffWyK5NQbqkYfzuzMv1Y2TTUbbKqP7MDk.json").unwrap();

        // Deserialize JSON into BerkeleyBlock struct
        let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(berkeley_block.get_snark_work_count(), 37, "snark work count should match");

        assert!(berkeley_block
            .get_snark_work()
            .iter()
            .any(|work| { work.fee_nanomina == 10_000_000 && &work.prover == "B62qosqzHi58Czax2RXfqPhMDzLogBeDVzSpsRDTCN1xeYUfrVy2F8P" }));

        assert_eq!(berkeley_block.get_aggregated_snark_work().len(), 8, "Aggregated snark work count should match");

        assert!(berkeley_block
            .get_aggregated_snark_work()
            .iter()
            .any(|work| { work.fee_nanomina == 10_000_000 && &work.prover == "B62qosqzHi58Czax2RXfqPhMDzLogBeDVzSpsRDTCN1xeYUfrVy2F8P" }))
    }
}
