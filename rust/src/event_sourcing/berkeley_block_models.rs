use super::{
    block::BlockTrait,
    models::{CommandStatus, CommandSummary, CommandType, CompletedWorksNanomina, ZkAppCommandSummary},
};
use crate::{
    constants::{MAINNET_COINBASE_REWARD, MINA_TOKEN_ID},
    utility::{decode_base58check_to_string, TreeNode},
};
use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Deserializer, Serialize};
use sonic_rs::{JsonValueTrait, Value};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct BerkeleyBlock {
    pub version: u32,
    pub data: Data,
}

impl BlockTrait for BerkeleyBlock {
    fn get_snark_work(&self) -> Vec<CompletedWorksNanomina> {
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

    fn get_user_commands(&self) -> Vec<CommandSummary> {
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

    fn get_coinbase_receiver(&self) -> String {
        self.data.protocol_state.body.consensus_state.coinbase_receiver.to_string()
    }

    fn get_coinbases(&self) -> Vec<Vec<Value>> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| opt_diff.as_ref().map(|diff| diff.coinbase.clone()))
            .collect()
    }
}

impl BerkeleyBlock {
    pub fn get_tokens_used(&self) -> Vec<String> {
        self.data.tokens_used.iter().map(|t| t.0.to_string()).collect()
    }

    pub fn contains_user_tokens(&self) -> bool {
        let mut tokens: HashSet<String> = self.get_tokens_used().into_iter().collect();
        tokens.remove(MINA_TOKEN_ID);
        !tokens.is_empty()
    }

    pub fn get_accounts_created(&self) -> Vec<(String, u64)> {
        self.data
            .accounts_created
            .iter()
            .map(|account_created| {
                let new_account = account_created.0[0].to_string();
                let fee_nanomina = BigDecimal::from_str(&account_created.1).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                (new_account, fee_nanomina.to_u64().expect("Cannot convert to u64"))
            })
            .collect::<Vec<_>>()
    }

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

    pub fn get_zk_app_commands_count(&self) -> usize {
        self.get_zk_app_commands().len()
    }

    pub fn get_zk_app_commands(&self) -> Vec<ZkAppCommandSummary> {
        [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()]
            .iter()
            .filter_map(|opt_diff| {
                opt_diff.as_ref().map(|diff| {
                    diff.commands
                        .iter()
                        .filter(|wrapper| matches!(wrapper.command, Command::ZkappCommand(_)))
                        .cloned()
                        .collect::<Vec<CommandWrapper>>()
                })
            })
            .flatten()
            .map(|w| w.to_zk_app_summary())
            .collect()
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

    pub fn get_global_slot_since_genesis(&self) -> u64 {
        self.data.protocol_state.body.consensus_state.global_slot_since_genesis.parse::<u64>().unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,
    pub accounts_created: Vec<(Vec<String>, String)>,
    pub tokens_used: Vec<(String, Option<(String, String)>)>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletedWorks {
    pub fee: String,
    pub prover: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct CommandWrapper {
    pub command: Command,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    SignedCommand(SignedCommand),
    ZkappCommand(ZkappCommand),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedCommand {
    pub payload: Payload,
    pub signer: String,
    pub signature: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub common: Common,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkappCommand {
    pub fee_payer: FeePayer,
    pub account_updates: Vec<AccountUpdateElt>,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountUpdateElt {
    pub elt: EltBody,
}

pub type AccountUpdateTree = TreeNode<AccountUpdateBody>;

impl AccountUpdateElt {
    pub fn to_tree_node(&self) -> AccountUpdateTree {
        let mut root = TreeNode::new(self.elt.account_update.body.clone());

        if let Some(calls) = &self.elt.calls {
            for call in calls {
                root.add_child(call.to_tree_node());
            }
        }

        root
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EltBody {
    pub account_update: AccountUpdate,
    pub calls: Option<Vec<AccountUpdateElt>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountUpdate {
    pub body: AccountUpdateBody,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeePayer {
    pub body: FeePayerBody,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeePayerBody {
    pub public_key: String,
    pub fee: String,
    pub nonce: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BalanceChange {
    pub magnitude: String,
    pub sgn: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountUpdateBody {
    pub public_key: String,
    pub token_id: String,
    pub balance_change: BalanceChange,
}

impl CommandWrapper {
    pub fn get_status(&self) -> String {
        self.status.to_string()
    }

    fn get_nonce(&self) -> usize {
        match &self.command {
            Command::SignedCommand(signed_command) => signed_command.payload.common.nonce.parse::<usize>().unwrap(),
            Command::ZkappCommand(zk_app_command) => zk_app_command.fee_payer.body.nonce.parse::<usize>().unwrap(),
        }
    }

    fn get_sender(&self) -> String {
        self.get_fee_payer()
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

    fn get_fee(&self) -> u64 {
        match &self.command {
            Command::SignedCommand(signed_command) => {
                let fee_nanomina = BigDecimal::from_str(&signed_command.payload.common.fee).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                fee_nanomina.to_u64().unwrap()
            }
            Command::ZkappCommand(zkapp_command) => {
                let fee_nanomina = BigDecimal::from_str(&zkapp_command.fee_payer.body.fee).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
                fee_nanomina.to_u64().unwrap()
            }
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
            Command::ZkappCommand(zk_app_command) => decode_base58check_to_string(&zk_app_command.memo).unwrap(),
        }
    }

    #[allow(dead_code)]
    fn get_account_updates(&self) -> Option<Vec<AccountUpdateTree>> {
        match &self.command {
            Command::SignedCommand(_) => None,
            Command::ZkappCommand(zk_app_command) => Some(zk_app_command.account_updates.iter().map(|au| au.to_tree_node()).collect::<Vec<_>>()),
        }
    }

    pub fn to_zk_app_commands(commands: Vec<Self>) -> Option<Vec<ZkappCommand>> {
        let zk_app_commands = commands
            .into_iter()
            .filter_map(|wrapper| match &wrapper.command {
                Command::SignedCommand(_) => None,
                Command::ZkappCommand(zk_app_command) => Some(zk_app_command.clone()),
            })
            .collect::<Vec<ZkappCommand>>();
        (!zk_app_commands.is_empty()).then_some(zk_app_commands)
    }

    fn get_fee_payer(&self) -> String {
        match &self.command {
            Command::SignedCommand(signed_command) => signed_command.payload.common.fee_payer_pk.clone(),
            Command::ZkappCommand(zkapp_command) => zkapp_command.fee_payer.body.public_key.to_string(),
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
                fee_nanomina: self.get_fee(),
                amount_nanomina: self.get_amount_nanomina(),
            },
            Command::ZkappCommand(_) => panic!("Cannot convert zk app command to CommandSummary"),
        }
    }

    pub fn to_zk_app_summary(&self) -> ZkAppCommandSummary {
        match &self.command {
            Command::SignedCommand(_) => panic!("Cannot convert signed command to CommandSummary"),
            Command::ZkappCommand(_) => ZkAppCommandSummary {
                memo: self.get_memo(),
                fee_payer: self.get_fee_payer(),
                status: match self.get_status().as_str() {
                    "Applied" => CommandStatus::Applied,
                    _ => CommandStatus::Failed,
                },
                txn_type: CommandType::ZkApp,
                nonce: self.get_nonce(),
                fee_nanomina: self.get_fee(),
                account_updates: self
                    .get_account_updates()
                    .unwrap_or_default()
                    .iter()
                    .map(|update_tree| update_tree.size())
                    .sum::<usize>(),
            },
        }
    }
}

impl<'de> Deserialize<'de> for CommandWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into a generic Value first
        let value: Value = Deserialize::deserialize(deserializer)?;

        // Ensure "data" field exists and is an array
        let data = value
            .get("data")
            .and_then(|v| v.clone().into_array())
            .ok_or_else(|| serde::de::Error::custom("Missing or invalid 'data' field"))?;

        if data.len() != 2 {
            return Err(serde::de::Error::custom("Expected 'data' field to have exactly 2 elements"));
        }

        // Extract status field
        let status = value
            .get("status")
            .and_then(|v| v.clone().into_array())
            .and_then(|arr| arr.first().cloned())
            .map(|v| v.as_str().unwrap().to_string())
            .unwrap();

        // Parse command type and details
        let command_type = data[0]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("First element in 'data' must be a string"))?;
        let details = data[1].clone(); // Clone for deserialization

        // Match and deserialize based on the command type
        match command_type {
            "Signed_command" => {
                let payload = details
                    .get("payload")
                    .ok_or_else(|| serde::de::Error::custom("Missing 'payload' field in Signed_command"))?;
                let signer = details
                    .get("signer")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| serde::de::Error::custom("Missing 'signer' field in Signed_command"))?
                    .to_string();
                let signature = details
                    .get("signature")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| serde::de::Error::custom("Missing 'signature' field in Signed_command"))?
                    .to_string();

                let common = payload
                    .get("common")
                    .ok_or_else(|| serde::de::Error::custom("Missing 'common' field in payload"))?;
                let body = payload.get("body").ok_or_else(|| serde::de::Error::custom("Missing 'body' field in payload"))?;

                let parsed_body = parse_body(body).unwrap();

                let signed_command = SignedCommand {
                    payload: Payload {
                        common: sonic_rs::from_value(common).map_err(serde::de::Error::custom)?,
                        body: parsed_body,
                    },
                    signer,
                    signature,
                    status: status.clone(),
                };

                Ok(CommandWrapper {
                    command: Command::SignedCommand(signed_command),
                    status,
                })
            }
            "Zkapp_command" => {
                let zkapp_command: ZkappCommand = sonic_rs::from_value(&details).map_err(serde::de::Error::custom)?;

                Ok(CommandWrapper {
                    command: Command::ZkappCommand(zkapp_command),
                    status,
                })
            }
            _ => Err(serde::de::Error::custom(format!("Unknown command type: {}", command_type))),
        }
    }
}

fn parse_body(body: &Value) -> Result<Body, sonic_rs::Error> {
    let body_array = body.clone().into_array().ok_or_else(|| serde::de::Error::custom("Expected array for body"))?;

    if body_array.len() != 2 {
        return Err(serde::de::Error::custom("Expected body array to have exactly 2 elements"));
    }

    let body_type = body_array[0]
        .as_str()
        .ok_or_else(|| serde::de::Error::custom("First element in body array must be a string"))?;
    let body_details = &body_array[1];

    match body_type {
        "Stake_delegation" => {
            let delegation_array = body_details
                .clone()
                .into_array()
                .ok_or_else(|| serde::de::Error::custom("Expected array for Stake_delegation details"))?;

            if delegation_array.len() != 2 {
                return Err(serde::de::Error::custom("Expected Stake_delegation array to have exactly 2 elements"));
            }

            if delegation_array[0] != "Set_delegate" {
                return Err(serde::de::Error::custom("Unknown Stake_delegation type"));
            }

            let delegate_info: DelegateInfo = sonic_rs::from_value(&delegation_array[1]).map_err(serde::de::Error::custom)?;
            Ok(Body::StakeDelegation(StakeDelegationType::SetDelegate(delegate_info)))
        }
        "Payment" => {
            let payment: Payment = sonic_rs::from_value(body_details).map_err(serde::de::Error::custom)?;
            Ok(Body::Payment(payment))
        }
        _ => Err(serde::de::Error::unknown_variant(body_type, &["Stake_delegation", "Payment"])),
    }
}

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

    #[test]
    fn test_berkeley_block_410543() {
        // Path to the test JSON file
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-410543-3NLeMXBpXKCpHtY2ugK5RdyQsZp2AUBQNYaJdgJNfu4h83TNvKGj.json")
                .expect("Failed to read test file");

        // Deserialize JSON into BerkeleyBlock struct
        let berkeley_block_res: Result<BerkeleyBlock, sonic_rs::Error> = sonic_rs::from_str(&file_content);

        if let Err(e) = &berkeley_block_res {
            println!("{e} => {}", &file_content[37000..39000]);
        }

        let berkeley_block = berkeley_block_res.unwrap();

        // Test the total number of user commands
        assert_eq!(berkeley_block.get_user_commands_count(), 62, "User commands count should be 62");

        // Extract all user commands
        let user_commands = berkeley_block.get_user_commands();

        // Test specific user command: Payment
        let payment_command = user_commands.iter().find(|cmd| {
            cmd.txn_type == CommandType::Payment
                && cmd.sender == "B62qrb1HQwNoK5YdTKnsZhoS7XCEEY1Bb6DJiMT3fSSbv7SVvgi7Q8t"
                && cmd.receiver == "B62qjcbHzUzpThL4apLCR5pmC3CqoRWUfqwnJxSBc82DxSXt5E6vwbw"
                && cmd.amount_nanomina == 300_000_000
        });
        assert!(
                payment_command.is_some(),
                "Expected a payment command from B62qrb1HQwNoK5YdTKnsZhoS7XCEEY1Bb6DJiMT3fSSbv7SVvgi7Q8t to B62qjcbHzUzpThL4apLCR5pmC3CqoRWUfqwnJxSBc82DxSXt5E6vwbw for 300_000_000"
            );

        // Test specific stake delegation command
        let stake_delegation_command = user_commands.iter().find(|cmd| {
            cmd.txn_type == CommandType::StakeDelegation
                && cmd.sender == "B62qrb1HQwNoK5YdTKnsZhoS7XCEEY1Bb6DJiMT3fSSbv7SVvgi7Q8t"
                && cmd.receiver == "B62qrQiw9JhUumq457sMxicgQ94Z1WD9JChzJu19kBE8Szb5T8tcUAC"
        });
        assert!(
                stake_delegation_command.is_some(),
                "Expected a stake delegation command from B62qrb1HQwNoK5YdTKnsZhoS7XCEEY1Bb6DJiMT3fSSbv7SVvgi7Q8t to B62qrQiw9JhUumq457sMxicgQ94Z1WD9JChzJu19kBE8Szb5T8tcUAC"
            );

        // Test zkApp commands count
        assert_eq!(berkeley_block.get_zk_app_commands_count(), 1, "zkApp commands count should be 1");
    }

    #[test]
    fn test_berkeley_block_410773() {
        // Path to the test JSON file
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-410773-3NLjmPVZ6HRV3CUdB3N8VbgwdNRAyjJibTCc4viKfUrrFuwTZk9s.json")
                .expect("Failed to read test file");

        // Deserialize JSON into BerkeleyBlock struct
        let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).unwrap();

        let zk_app_commands = berkeley_block.get_zk_app_commands();
        assert_eq!(zk_app_commands.len(), 5);

        let first_zkapp_command = zk_app_commands.first().unwrap();
        assert_eq!(first_zkapp_command.fee_payer, "B62qm2aFMwggaVEwAkJB1r77adTBfPkbmJuZkmjzFmsCfAsqrn9kc44");
        assert_eq!(first_zkapp_command.fee_nanomina, 5_000_000);
        assert_eq!(first_zkapp_command.nonce, 200);
        assert_eq!(first_zkapp_command.memo, "Init vote zkapp".to_string());
        assert_eq!(first_zkapp_command.status, CommandStatus::Applied);
        assert_eq!(first_zkapp_command.txn_type, CommandType::ZkApp);
        assert_eq!(first_zkapp_command.account_updates, 1);

        let second_zkapp_command = zk_app_commands.get(1).unwrap();
        assert_eq!(second_zkapp_command.account_updates, 1);

        let third_zkapp_command = zk_app_commands.get(2).unwrap();
        assert_eq!(third_zkapp_command.account_updates, 1);

        let fourth_zkapp_command = zk_app_commands.get(3).unwrap();
        assert_eq!(fourth_zkapp_command.account_updates, 4);

        let fifth_zkapp_command = zk_app_commands.last().unwrap();
        assert_eq!(fifth_zkapp_command.account_updates, 4);

        let accounts_created = berkeley_block.get_accounts_created();
        assert_eq!(accounts_created.len(), 3);

        assert_eq!(
            accounts_created[0],
            ("B62qos6TJeFTeTgzxZ86F2R4YQELqwNf7q7JPUXYNPj4PB1Dfa3THKD".to_string(), 1_000_000_000)
        );
        assert_eq!(
            accounts_created[1],
            ("B62qrKCzoAhAS7vxdTwfE5btgbnFiR9QrBgESA6SSUiwJ7L17oxAdoa".to_string(), 1_000_000_000)
        );
        assert_eq!(
            accounts_created[2],
            ("B62qj4JGH7KPE59RLhX4pyZdt1g6rJSuniWWrvXRRrVrkGAePuKJZnW".to_string(), 1_000_000_000)
        );
    }

    #[test]
    fn test_get_tokens_used() {
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-410773-3NLjmPVZ6HRV3CUdB3N8VbgwdNRAyjJibTCc4viKfUrrFuwTZk9s.json")
                .expect("Failed to read test file");

        let block: BerkeleyBlock = sonic_rs::from_str(&file_content).unwrap();

        // Call `get_tokens_used()`
        let tokens_used = block.get_tokens_used();

        // Assert that the tokens match what we initialized
        assert_eq!(tokens_used, vec![MINA_TOKEN_ID, "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg"]);
        // rust/src/event_sourcing/test_data/berkeley_blocks/mainnet-410543-3NLeMXBpXKCpHtY2ugK5RdyQsZp2AUBQNYaJdgJNfu4h83TNvKGj.json
    }

    #[test]
    fn test_contains_user_tokens() {
        let file_content =
            get_cleaned_pcb("./src/event_sourcing/test_data/berkeley_blocks/mainnet-410543-3NLeMXBpXKCpHtY2ugK5RdyQsZp2AUBQNYaJdgJNfu4h83TNvKGj.json")
                .expect("Failed to read test file");

        let block: BerkeleyBlock = sonic_rs::from_str(&file_content).unwrap();

        // Assert that the tokens match what we initialized
        assert!(!block.contains_user_tokens());
    }
}
