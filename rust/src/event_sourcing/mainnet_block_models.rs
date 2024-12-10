use super::models::{CommandStatus, CommandSummary, CommandType, CompletedWorksNanomina};
use crate::{constants::MAINNET_COINBASE_REWARD, utility::decode_base58check_to_string};
use bigdecimal::{BigDecimal, ToPrimitive};
use core::fmt;
use serde::{
    de::{SeqAccess, Visitor},
    Deserializer,
};
use sonic_rs::{Deserialize, JsonValueTrait, Serialize, Value};
use std::{collections::HashMap, str::FromStr};

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

    pub fn get_excess_block_fees(&self) -> u64 {
        let total_snark_fees = self
            .get_snark_work()
            .iter()
            .map(|ft| (ft.fee.parse::<f64>().unwrap() * 1_000_000_000f64) as u64)
            .sum::<u64>();

        let mut total_fees_paid_into_block_pool = self.get_user_commands().iter().map(|uc| uc.fee_nanomina).sum::<u64>();
        for ftvc in self.get_fee_transfers_via_coinbase().unwrap_or_default().iter() {
            total_fees_paid_into_block_pool += (ftvc.fee * 1_000_000_000f64) as u64;
        }
        total_fees_paid_into_block_pool.saturating_sub(total_snark_fees)
    }

    pub fn get_fee_transfers(&self) -> Vec<FeeTransfer> {
        let excess_block_fees = self.get_excess_block_fees();
        let mut fee_transfers: HashMap<String, u64> = HashMap::new();
        if excess_block_fees > 0 {
            fee_transfers.insert(self.get_coinbase_receiver(), excess_block_fees);
        }
        for completed_work in self.get_snark_work() {
            let fee_nanomina = (completed_work.fee.parse::<f64>().unwrap() * 1_000_000_000f64) as u64;
            *fee_transfers.entry(completed_work.prover).or_insert(0) += fee_nanomina;
        }

        // If the fee for a completed work is higher than the available fees, the remainder
        // is allotted out of the coinbase via a fee transfer via coinbase
        for ftvc in self.get_fee_transfers_via_coinbase().unwrap_or_default().iter() {
            let ftvc_fee_nanomina = (ftvc.fee * 1_000_000_000f64) as u64;

            if let Some(current_fee) = fee_transfers.get_mut(&ftvc.receiver) {
                if *current_fee > ftvc_fee_nanomina {
                    *current_fee -= ftvc_fee_nanomina;
                } else {
                    fee_transfers.remove(&ftvc.receiver);
                }
            }
        }

        fee_transfers.retain(|_, v| *v > 0u64);
        fee_transfers
            .into_iter()
            .map(|(prover, fee_nanomina)| FeeTransfer {
                recipient: prover,
                fee_nanomina,
            })
            .collect()
    }

    pub fn get_internal_command_count(&self) -> usize {
        // Get fee transfers and remove those also in fee transfers via coinbase
        let fee_transfers = self.get_fee_transfers();
        let fee_transfers_via_coinbase = self.get_fee_transfers_via_coinbase().unwrap_or_default();

        fee_transfers.len() + fee_transfers_via_coinbase.len() + 1 // +1 for coinbase
    }

    pub fn get_total_command_count(&self) -> usize {
        self.get_internal_command_count() + self.get_user_commands_count()
    }

    pub fn get_fee_transfers_via_coinbase(&self) -> Option<Vec<FeeTransferViaCoinbase>> {
        // Combine pre and post diff coinbase arrays
        let diffs = [self.get_staged_ledger_pre_diff(), self.get_staged_ledger_post_diff()];

        let fee_transfers = diffs
            .iter()
            .filter_map(|opt_diff| {
                opt_diff.as_ref().and_then(|diff| {
                    if diff.coinbase.first().map_or(false, |v| v == "One" || v == "Two") {
                        // Process the remaining elements
                        Some(
                            diff.coinbase
                                .iter()
                                .filter_map(|v2| {
                                    // Skip non-objects and null values
                                    if !v2.is_object() || v2.is_null() {
                                        return None;
                                    }

                                    // Try to extract "receiver_pk" and "fee"
                                    let receiver = v2.get("receiver_pk")?.as_str()?.to_string();
                                    let fee = v2.get("fee")?.as_str()?.parse::<f64>().ok()?;

                                    Some(FeeTransferViaCoinbase { receiver, fee })
                                })
                                .collect::<Vec<FeeTransferViaCoinbase>>(),
                        )
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect::<Vec<FeeTransferViaCoinbase>>();

        if fee_transfers.is_empty() {
            None
        } else {
            Some(fee_transfers)
        }
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
                    Some(v) if v == "Zero" => None,
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

    pub fn get_aggregated_snark_work(&self) -> Vec<CompletedWorksNanomina> {
        let mut aggregated_snark_work: HashMap<String, BigDecimal> = HashMap::new();

        for completed_work in self.get_snark_work() {
            let fee_decimal = BigDecimal::from_str(&completed_work.fee).expect("Invalid number format") * BigDecimal::from(1_000_000_000);
            *aggregated_snark_work.entry(completed_work.prover.clone()).or_insert(BigDecimal::from(0)) += fee_decimal;
        }

        aggregated_snark_work
            .into_iter()
            .map(|(prover, fee_decimal)| {
                let fee_nanomina = fee_decimal
                    .with_scale(0) // Ensure it's rounded to an integer (nanomina are integers)
                    .to_u64()
                    .unwrap();

                CompletedWorksNanomina { prover, fee_nanomina }
            })
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeeTransferViaCoinbase {
    pub receiver: String,
    pub fee: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FeeTransfer {
    pub recipient: String,
    pub fee_nanomina: u64,
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
        self.status.first().unwrap().as_str().unwrap().to_string()
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

    fn get_type(&self) -> CommandType {
        match self.signed_command.as_ref().unwrap().payload.body.clone() {
            Body::Payment(_) => CommandType::Payment,
            Body::StakeDelegation(StakeDelegationType::SetDelegate(_)) => CommandType::StakeDelegation,
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
        decode_base58check_to_string(&self.signed_command.as_ref().unwrap().payload.common.memo.to_string()).unwrap()
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
            status: match self.get_status().as_str() {
                "Applied" => CommandStatus::Applied,
                _ => CommandStatus::Failed,
            },
            txn_type: self.get_type(),
            nonce: self.get_nonce(),
            fee_nanomina: (self.get_fee() * 1_000_000_000f64) as u64,
            amount_nanomina: self.get_amount_nanomina(),
        }
    }
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
        let path = Path::new("./src/event_sourcing/test_data/misc_blocks/mainnet-185-3NKQ3K2SNp58PEAb8UjpBe5uo3KQKxphURuE9Eq2J8JYBVCD7PSu.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        // Test global_slot_since_genesis
        assert_eq!(mainnet_block.get_global_slot_since_genesis(), 263);

        // Test coinbase reward
        assert_eq!(mainnet_block.get_coinbase_reward_nanomina(), 720_000_000_000);

        // Test user commands count
        assert_eq!(mainnet_block.get_user_commands_count(), 2);

        assert_eq!(mainnet_block.get_internal_command_count(), 2);

        assert_eq!(mainnet_block.get_total_command_count(), 4);

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
        assert_eq!(&first_user_command.memo, "");

        // Test memo
        assert_eq!(first_user_command.nonce, 265);

        // Test sender
        assert_eq!(&first_user_command.sender, "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy");

        // Test receiver
        assert_eq!(&first_user_command.receiver, "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM");

        // test status
        assert_eq!(first_user_command.status, CommandStatus::Applied);

        // test
        assert_eq!(first_user_command.amount_nanomina, 1000);
    }

    #[test]
    fn test_mainnet_block_with_stake_delegation_and_fee_transfers() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/misc_blocks/mainnet-199999-3NKDFcMG4gbdeSwEYzoAURSHv4uRabTFbTY7W6JpECN2rHmwYE8j.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(mainnet_block.get_user_commands_count(), 23);
        assert_eq!(mainnet_block.get_total_command_count(), 35);
        assert_eq!(mainnet_block.get_internal_command_count(), 12);

        let user_commands = mainnet_block.get_user_commands();
        let fifth_user_command = user_commands.get(4).unwrap();
        let second_user_command = user_commands.get(1).unwrap();
        let eleventh_user_command = user_commands.get(10).unwrap();
        assert_eq!(&second_user_command.memo, "memo");
        assert_eq!(&eleventh_user_command.memo, "save_genesis_grant");

        // Test fields of the fifth user command
        assert_eq!(fifth_user_command.fee_nanomina, 10_100_000);
        assert_eq!(&fifth_user_command.memo, "");
        assert_eq!(fifth_user_command.nonce, 0);
        assert_eq!(&fifth_user_command.sender, "B62qj2PMFaL2bmZQsWMfr2eiMxNErwUrZYKvt8JHgany2G3CvF6RGoc");
        assert_eq!(&fifth_user_command.receiver, "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6");
        assert_eq!(fifth_user_command.status, CommandStatus::Applied);
        assert_eq!(fifth_user_command.amount_nanomina, 0);

        // Test fee transfers
        let fee_transfers = mainnet_block.get_fee_transfers();
        let expected_fee_transfers = vec![
            ("B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h", 312),
            ("B62qmwvcwk2vFwAA4DUtRE5QtPDnhJgNQUwxiZmidiqm6QK63v82vKP", 350000),
            ("B62qoHXLV7QT6ezrPai2fLiS2k7eA4dDecW6hkDjXeuKdgjsTDATdNi", 200),
            ("B62qn2Ne2JGRdbHXdfD8wkA6PTWuBjaxUDQ6QuPAmggrcYjTP3HwWkF", 1000000),
            ("B62qpcENWiR5VKkrHscV9cWfPwNs56ExFeb94FDiVz9GeV2mBNpMCkY", 60000),
            ("B62qnucUMHz7Dw2ReNgWhmR5XCvPeQjJWPReuQ8GwPyY4qj1otGBiKr", 960000),
            ("B62qqWzWHjUmJSSB9db6BDpGjFJkRNjjtZorwJdpeASzSPHpRe4CoJS", 1100000),
            ("B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp", 3000000),
            ("B62qp5dXkkj3TkkfPos35rNYxVTKTbm5CqigfgKppA5E7GQHK7H3nNd", 1777776),
            ("B62qkBqSkXgkirtU3n8HJ9YgwHh3vUD6kGJ5ZRkQYGNPeL5xYL2tL1L", 3000000),
            ("B62qmJZD44acynLzxERnhUkbVjFXSqphiRfT6tUwfACYCaCw1xhqoBH", 395151714),
        ];

        println!("{:#?}", fee_transfers);

        // Assert that the number of fee transfers matches
        assert_eq!(fee_transfers.len(), expected_fee_transfers.len());

        // Check each expected fee transfer against the result
        for (expected_recipient, expected_fee) in expected_fee_transfers.iter() {
            assert_eq!(
                fee_transfers
                    .iter()
                    .filter(|ft| &ft.recipient == expected_recipient && &ft.fee_nanomina == expected_fee)
                    .count(),
                1
            )
        }
    }

    #[test]
    fn test_mainnet_block_with_fee_transfer_via_coinbase() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/misc_blocks/mainnet-300000-3NLuJ7pWnSw2iYhjsJFTX1CGTavB3FHcwvP1E6r5Ewxss2qf8udQ.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(mainnet_block.get_user_commands_count(), 30);
        assert_eq!(mainnet_block.get_internal_command_count(), 3);
        assert_eq!(mainnet_block.get_total_command_count(), 33);

        let fee_transfer_via_coinbase = mainnet_block.get_fee_transfers_via_coinbase().unwrap();
        let first_ftva = fee_transfer_via_coinbase.first().unwrap();
        assert_eq!(first_ftva.receiver, "B62qmwvcwk2vFwAA4DUtRE5QtPDnhJgNQUwxiZmidiqm6QK63v82vKP");
        assert_eq!(first_ftva.fee, 0.00005f64);

        // Excess block fees are returned to coinbase receiver in the form of internal command
        let internal_commands = mainnet_block.get_fee_transfers();
        assert_eq!(internal_commands.len(), 1);
        let first_internal_command = internal_commands.first().unwrap();
        assert_eq!(first_internal_command.recipient, mainnet_block.get_coinbase_receiver());
        assert_eq!(first_internal_command.fee_nanomina, 1346000001);
    }

    #[test]
    fn test_mainnet_block_has_two_internal_commands() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        // this block has no compelted works so the user command fees are transferred to the coinbase receiver,
        // for a total of two internal commands
        // 1. coinbase receiver
        // 2. fee transfer of excess fees
        assert_eq!(mainnet_block.get_internal_command_count(), 2);
    }

    #[test]
    fn test_mainnet_block_4556() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/misc_blocks/mainnet-4556-3NKSoUrfAP9zqe6HP3EGWSrzhnpixbez7Hk7EerXjYpCybwKbdme.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        // 6 internal commands
        // 1. coinbase receiver
        // 2. fee transfer via coinbase
        // 3. 3 internal commands
        // 4. A payment to the coinbase receiver out of the excess of fees
        assert_eq!(mainnet_block.get_internal_command_count(), 6);
        assert_eq!(mainnet_block.get_user_commands_count(), 2);
        assert_eq!(mainnet_block.get_total_command_count(), 8);

        assert!(mainnet_block
            .get_fee_transfers_via_coinbase()
            .unwrap()
            .iter()
            .any(|ft| { ft.receiver == *"B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h" && (ft.fee * 1_000_000_000f64) as u64 == 8 }));
        assert!(mainnet_block
            .get_fee_transfers()
            .iter()
            .any(|ft| { ft.recipient == *"B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h" && ft.fee_nanomina == 984 }));
        assert!(mainnet_block
            .get_fee_transfers()
            .iter()
            .any(|ft| { ft.recipient == *"B62qqyBD6cBATkdAa29tNAEJvJfkMTzwfuWyaJz6Ya3dJ76ZixfUter" && ft.fee_nanomina == 5_000_000 }));
        assert!(mainnet_block
            .get_fee_transfers()
            .iter()
            .any(|ft| { ft.recipient == *"B62qr9jmNyuKG9Zhi1jENgPuswFRRDrkin3tP6D76qx8HNpjke5aUMs" && ft.fee_nanomina == 110_000 }));

        // Excess block fees paid to coinbase receiver
        // within one of the actors
        assert_eq!(mainnet_block.get_excess_block_fees(), 14_889_016);

        assert_eq!(mainnet_block.get_aggregated_snark_work().len(), 3);
    }

    #[test]
    fn test_mainnet_block_has_3_internal_commands() {
        // Path to your test JSON file
        let path = Path::new("./src/event_sourcing/test_data/misc_blocks/mainnet-583-3NKQkwAitnSHvvKU2cHLqmRAzaueQMQsXKKxiurQoR7759s555Xu.json");
        let file_content = std::fs::read_to_string(path).expect("Failed to read test file");

        // Deserialize JSON into MainnetBlock struct
        let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse JSON");

        assert_eq!(mainnet_block.get_internal_command_count(), 3);
    }
}
