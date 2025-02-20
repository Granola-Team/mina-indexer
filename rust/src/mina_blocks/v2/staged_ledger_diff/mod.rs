//! V2 staged ledger diff

pub mod completed_work;

use super::{protocol_state::SupplyAdjustment, AppState, Permissions, Timing, VerificationKey};
use crate::{
    base::{
        amount::Amount, nonce::Nonce, numeric::Numeric, public_key::PublicKey,
        scheduled_time::ScheduledTime, Balance,
    },
    command::{to_mina_format, to_zkapp_json},
    constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
    ledger::{token::TokenAddress, LedgerHash},
    protocol::serialization_types::staged_ledger_diff::{
        TransactionStatus2, TransactionStatusFailedType,
    },
    utility::functions::nanomina_to_mina,
};
use completed_work::CompletedWork;
use mina_serialization_versioned::Versioned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    pub completed_works: Vec<CompletedWork>,
    pub commands: Vec<UserCommand>,
    pub coinbase: Coinbase,
    pub internal_command_statuses: Vec<InternalCommandStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalCommandStatus(pub (StatusKind,));

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Coinbase {
    Zero((CoinbaseKind,)),
    One(CoinbaseKind, Option<CoinbasePayload>),
    Two(
        CoinbaseKind,
        Option<(CoinbasePayload, Option<CoinbasePayload>)>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoinbaseKind {
    Two,
    One,
    Zero,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoinbasePayload {
    pub receiver_pk: PublicKey,
    pub fee: Amount,
}

/// User command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCommand {
    pub data: (UserCommandKind, UserCommandData),
    #[serde(with = "status_format")]
    pub status: Status,
}

impl Eq for UserCommand {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub status: Vec<String>,
    pub failure_data: Option<Vec<Vec<Vec<Value>>>>, // Using serde_json::Value instead of String
}

// Custom serialization module for Status
mod status_format {
    use super::Status;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Status, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;

        if value.is_empty() {
            return Err(serde::de::Error::custom("Status array cannot be empty"));
        }

        let status_str = value[0]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("First element must be a string"))?;

        Ok(Status {
            status: vec![status_str.to_string()],
            failure_data: if value.len() > 1 {
                Some(serde_json::from_value(value[1].clone()).map_err(serde::de::Error::custom)?)
            } else {
                None
            },
        })
    }

    pub fn serialize<S>(status: &Status, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq =
            serializer.serialize_seq(Some(if status.failure_data.is_some() { 2 } else { 1 }))?;

        seq.serialize_element(&status.status[0])?;

        if let Some(ref failure_data) = status.failure_data {
            seq.serialize_element(failure_data)?;
        }

        seq.end()
    }
}

impl Status {
    pub fn applied() -> Self {
        Status {
            status: vec!["Applied".to_string()],
            failure_data: None,
        }
    }

    pub fn failed(failure_data: Vec<Vec<Vec<Value>>>) -> Self {
        Status {
            status: vec!["Failed".to_string()],
            failure_data: Some(failure_data),
        }
    }
}

impl From<Vec<String>> for Status {
    fn from(status: Vec<String>) -> Self {
        Status {
            status,
            failure_data: None,
        }
    }
}

impl From<Status> for TransactionStatus2 {
    fn from(status: Status) -> Self {
        match status.status.first() {
            Some(s) if s == "Applied" => TransactionStatus2::Applied,
            Some(s) if s == "Failed" => TransactionStatus2::Failed(
                status
                    .failure_data
                    .map(|fails| {
                        fails
                            .iter()
                            .map(|outer| {
                                vec![outer
                                    .iter()
                                    .filter_map(|inner| {
                                        if inner.is_empty() {
                                            None
                                        } else if inner.len() == 2
                                            && inner[0].as_str() == Some("Account_app_state_precondition_unsatisfied") {
                                            // Handle numeric parameter case
                                            inner[1].as_i64().map(|n| {
                                                vec![Versioned::new(
                                                    TransactionStatusFailedType::AccountAppStatePreconditionUnsatisfied(n)
                                                )]
                                            })
                                        } else {
                                            // Handle regular failure types
                                            inner[0].as_str().map(|reason| {
                                                vec![Versioned::new(
                                                    TransactionStatusFailedType::from_str(reason)
                                                        .unwrap_or_else(|_| {
                                                            panic!("Invalid failure reason: {}", reason)
                                                        })
                                                )]
                                            })
                                        }
                                    })
                                    .flatten()
                                    .collect()]
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            _ => panic!("Unexpected status value: {:?}", status.status),
        }
    }
}

/// User command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserCommandKind {
    #[serde(rename = "Signed_command")]
    SignedCommand,

    #[serde(rename = "Zkapp_command")]
    ZkappCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserCommandData {
    SignedCommandData(Box<SignedCommandData>),
    ZkappCommandData(ZkappCommandData),
}

impl std::cmp::Eq for UserCommandData {}

/// Signed command

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCommandData {
    pub signer: PublicKey,
    pub payload: SignedCommandPayload,
    pub signature: String,
}

impl std::cmp::Eq for SignedCommandData {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCommandPayload {
    pub common: SignedCommandPayloadCommon,
    pub body: (SignedCommandPayloadKind, SignedCommandPayloadBody),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignedCommandPayloadKind {
    Payment,

    #[serde(rename = "Stake_delegation")]
    StakeDelegation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SignedCommandPayloadBody {
    Payment(PaymentPayload),
    StakeDelegation((SetDelegate, StakeDelegationPayload)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetDelegate {
    #[serde(rename = "Set_delegate")]
    SetDelegate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub receiver_pk: PublicKey,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StakeDelegationPayload {
    pub new_delegate: PublicKey,
}

/// Zkapp command

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappCommandData {
    // base58 encoded memo
    pub memo: String,
    pub fee_payer: FeePayer,
    pub account_updates: Vec<AccountUpdates>,
}

impl Eq for ZkappCommandData {}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FeePayer {
    pub body: FeePayerBody,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FeePayerBody {
    pub public_key: PublicKey,
    pub fee: Amount,
    pub valid_until: Option<ScheduledTime>,
    pub nonce: Nonce,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdates {
    pub elt: Elt,
    pub stack_hash: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Elt {
    pub account_update: AccountUpdate,
    pub account_update_digest: String,
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub body: AccountUpdateBody,
    pub authorization: Authorization,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ProofOrSignature {
    Proof,
    Signature,
}

// see https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/account_update.ml#L24-L28
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authorization {
    #[serde(rename = "None_given")]
    NoneGiven((String,)),
    Either((String,)),
    #[serde(rename = "Proof")]
    Proof_((ProofOrSignature, String)),
    Proof((String,)),
    #[serde(rename = "Signature")]
    Signature_((ProofOrSignature, String)),
    Signature((String,)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountUpdateBody {
    pub public_key: PublicKey,
    pub token_id: TokenAddress,
    pub update: Update,
    pub balance_change: SupplyAdjustment,
    pub increment_nonce: bool,
    pub events: Vec<ZkappEvents>,
    pub actions: Vec<ZkappActions>,
    pub call_data: String,
    pub preconditions: Preconditions,
    pub use_full_commitment: bool,
    pub implicit_account_creation_fee: bool,
    pub may_use_token: (MayUseToken,),
    pub authorization_kind: Authorization,
}

// see https://github.com/MinaProtocol/mina/blob/compatible/src/lib/mina_base/account_update.ml#L136-L147
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MayUseToken {
    No,
    #[serde(rename = "Parents_own_token")]
    ParentsOwnToken,
    #[serde(rename = "Inherit_from_parent")]
    InheritFromParent,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappActions(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappEvents(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Update {
    pub app_state: [UpdateKind; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub delegate: UpdateKind,
    pub verification_key: UpdateVerificationKey,
    pub permissions: UpdatePermissions,
    pub zkapp_uri: UpdateKind,
    pub token_symbol: UpdateKind,
    pub timing: UpdateTiming,
    pub voting_for: UpdateKind,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateKind {
    Keep((String,)),
    Set((String, String)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateVerificationKey {
    Keep((String,)),
    Set((String, VerificationKey)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdatePermissions {
    Keep((String,)),
    Set((String, Permissions)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTiming {
    Keep((String,)),
    Set((String, Timing)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Preconditions {
    pub network: NetworkPreconditions,
    pub account: AccountPreconditions,
    pub valid_while: Precondition<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NetworkPreconditions {
    pub snarked_ledger_hash: Precondition<LedgerHashBounds>,
    pub blockchain_length: Precondition<NumericBoundsU32>,
    pub min_window_density: Precondition<NumericBoundsU32>,
    pub total_currency: Precondition<NumericBoundsU32>,
    pub global_slot_since_genesis: Precondition<NumericBoundsU32>,
    pub staking_epoch_data: StakingEpochDataPreconditions,
    pub next_epoch_data: StakingEpochDataPreconditions,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StakingEpochDataPreconditions {
    pub ledger: LedgerPreconditions,
    pub seed: Precondition<String>,
    pub start_checkpoint: Precondition<String>,
    pub lock_checkpoint: Precondition<String>,
    pub epoch_length: Precondition<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LedgerPreconditions {
    pub hash: Precondition<String>,
    pub total_currency: Precondition<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccountPreconditions {
    pub balance: Precondition<NumericBoundsU64>,
    pub nonce: Precondition<NumericBoundsU32>,
    pub receipt_chain_hash: Precondition<String>,
    pub delegate: Precondition<PublicKey>,
    pub state: [Precondition<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub action_state: Precondition<String>,
    pub proved_state: Precondition<bool>,
    pub is_new: Precondition<bool>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Precondition<T> {
    Ignore((String,)),
    Check((String, T)),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NumericBoundsU32 {
    lower: Numeric<u32>,
    upper: Numeric<u32>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NumericBoundsU64 {
    lower: Numeric<u64>,
    upper: Numeric<u64>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LedgerHashBounds {
    lower: LedgerHash,
    upper: LedgerHash,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Call {
    pub elt: Box<Elt>,
    pub stack_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCommandPayloadCommon {
    // Base58 encoded string
    pub memo: String,
    pub fee: Amount,
    pub fee_payer_pk: PublicKey,
    pub nonce: Nonce,
    pub valid_until: ScheduledTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusKind {
    Applied,
    Failed,
}

impl UserCommandData {
    pub fn to_mina_json(self) -> serde_json::Value {
        use crate::command::signed::SignedCommand;

        match &self {
            UserCommandData::SignedCommandData(_) => {
                let mut json: serde_json::Value = SignedCommand::V2(self).into();
                convert_object("", &mut json);
                to_mina_format(json)
            }
            UserCommandData::ZkappCommandData(data) => to_zkapp_json(data),
        }
    }
}

fn convert_object(key: &str, value: &mut serde_json::Value) {
    use serde_json::Value;

    match value {
        Value::Number(number) => {
            if key == "fee" {
                // fee convert
                let nanomina = number.as_u64().unwrap();
                *value = Value::String(nanomina_to_mina(nanomina));
            } else {
                *value = Value::String(number.to_string());
            }
        }
        Value::Object(obj) => obj
            .iter_mut()
            .for_each(|(k, value)| convert_object(k, value)),
        _ => (),
    }
}

/////////////////
// conversions //
/////////////////

impl<T> From<UpdateKind> for Option<T>
where
    T: From<String>,
{
    fn from(value: UpdateKind) -> Self {
        match value {
            UpdateKind::Keep(_) => None,
            UpdateKind::Set((_, data)) => Some(data.into()),
        }
    }
}

impl From<UpdateVerificationKey> for Option<VerificationKey> {
    fn from(value: UpdateVerificationKey) -> Self {
        match value {
            UpdateVerificationKey::Keep(_) => None,
            UpdateVerificationKey::Set((_, vk)) => Some(vk),
        }
    }
}

impl From<UpdatePermissions> for Option<Permissions> {
    fn from(value: UpdatePermissions) -> Self {
        match value {
            UpdatePermissions::Keep(_) => None,
            UpdatePermissions::Set((_, perm)) => Some(perm),
        }
    }
}

impl From<UpdateTiming> for Option<Timing> {
    fn from(value: UpdateTiming) -> Self {
        match value {
            UpdateTiming::Keep(_) => None,
            UpdateTiming::Set((_, timing)) => Some(timing),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{signed::SignedCommand, to_mina_json, to_zkapp_json, UserCommandWithStatusT},
        mina_blocks::v2::staged_ledger_diff::UserCommandData,
    };
    use std::path::PathBuf;

    #[test]
    fn v2_signed_command_to_mina_json() -> anyhow::Result<()> {
        let block_file = PathBuf::from("./tests/data/hardfork/mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let signed_cmds = precomputed_block
            .commands()
            .into_iter()
            .map(|c| {
                let json: serde_json::Value = SignedCommand::from_user_command(c).into();
                serde_json::to_string_pretty(&to_mina_json(json)).unwrap()
            })
            .collect::<Vec<_>>();

        let expect0 = r#"{
  "payload": {
    "body": [
      "Payment",
      {
        "amount": "1000000000",
        "receiver_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
      }
    ],
    "common": {
      "fee": "0.0011",
      "fee_payer_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
      "memo": "",
      "nonce": "765",
      "valid_until": "4294967295"
    }
  },
  "signature": "7mX5FyaaoRY5a3hKP3kqhm6A4gWo9NtoHMh7irbB3Dt326wm8gyfsEQeHKJgYqQeo7nBgFGNjCD9eC265VrECYZJqYsD5V5R",
  "signer": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32"
}"#;

        assert_eq!(signed_cmds, vec![expect0]);
        Ok(())
    }

    #[test]
    fn zkapp_command_to_mina_json_1() -> anyhow::Result<()> {
        let block_file = PathBuf::from("./tests/data/misc_blocks/mainnet-410535-3NLLmswaSwYVSERiQMdvTdKdBN6TNMgUGmd548zK7e82CaS3tNJK.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let signed_cmds = precomputed_block
            .commands()
            .into_iter()
            .filter_map(|cmd| {
                if !cmd.is_zkapp_command() {
                    // filter out non-zkapp commands
                    return None;
                }

                if let SignedCommand::V2(UserCommandData::ZkappCommandData(data)) = cmd.into() {
                    return Some(serde_json::to_string_pretty(&to_zkapp_json(&data)).unwrap());
                }

                None
            })
            .collect::<Vec<_>>();

        let expect = r#"{
  "account_updates": [],
  "fee_payer": {
    "authorization": "7mXBToH3YVEDek6hpsfzNV3AE89udwKfG4vXKpyKBkhkxdJXvvxdtQRUjMBz1cnPiBVLSPKRgp88tN2ndN85NujFeH3bjQCE",
    "body": {
      "fee": "0.1",
      "nonce": "3",
      "public_key": "B62qkbCH6jLfVEgR36UGyUzzFTPogr2CQb8fPLLFr6DWajMokYEAJvX",
      "valid_until": null
    }
  },
  "memo": "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
}"#;

        assert_eq!(signed_cmds, vec![expect]);
        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn zkapp_command_to_mina_json_2() -> anyhow::Result<()> {
        let block_file = PathBuf::from("./tests/data/misc_blocks/mainnet-397612-3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let json = precomputed_block
            .commands()
            .into_iter()
            .filter_map(|cmd| {
                if !cmd.is_zkapp_command() {
                    // filter out non-zkapp commands
                    return None;
                }

                let cmd: SignedCommand = cmd.into();
                let json: serde_json::Value = cmd.into();

                Some(serde_json::to_string_pretty(&json).unwrap())
            })
            .collect::<Vec<_>>();

        // check the first zkapp json
        let expect = r#"{
  "account_updates": [
    {
      "elt": {
        "account_update": {
          "authorization": [
            "Signature",
            "7mXQ2QQakF4g4DCv8Q9EzCMzGdDpZXR8GdBWd4KMMoyDcoMerEAF1eouCrVByGUZcoXXLCTxkdJdk9Y7u4EoAemCAQuArjGa"
          ],
          "body": {
            "actions": [],
            "authorization_kind": [
              "Signature"
            ],
            "balance_change": {
              "magnitude": "0",
              "sgn": [
                "Pos"
              ]
            },
            "call_data": "0x1450BC0E0E4E32BEF69CCBCC7E238503648E25C1DFA915FAF548AE3AE7377AD1",
            "events": [],
            "implicit_account_creation_fee": false,
            "increment_nonce": true,
            "may_use_token": [
              "No"
            ],
            "preconditions": {
              "account": {
                "action_state": [
                  "Ignore"
                ],
                "balance": [
                  "Ignore"
                ],
                "delegate": [
                  "Ignore"
                ],
                "is_new": [
                  "Ignore"
                ],
                "nonce": [
                  "Check",
                  {
                    "lower": "1",
                    "upper": "1"
                  }
                ],
                "proved_state": [
                  "Ignore"
                ],
                "receipt_chain_hash": [
                  "Ignore"
                ],
                "state": [
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ],
                  [
                    "Ignore"
                  ]
                ]
              },
              "network": {
                "blockchain_length": [
                  "Ignore"
                ],
                "global_slot_since_genesis": [
                  "Ignore"
                ],
                "min_window_density": [
                  "Ignore"
                ],
                "next_epoch_data": {
                  "epoch_length": [
                    "Ignore"
                  ],
                  "ledger": {
                    "hash": [
                      "Ignore"
                    ],
                    "total_currency": [
                      "Ignore"
                    ]
                  },
                  "lock_checkpoint": [
                    "Ignore"
                  ],
                  "seed": [
                    "Ignore"
                  ],
                  "start_checkpoint": [
                    "Ignore"
                  ]
                },
                "snarked_ledger_hash": [
                  "Ignore"
                ],
                "staking_epoch_data": {
                  "epoch_length": [
                    "Ignore"
                  ],
                  "ledger": {
                    "hash": [
                      "Ignore"
                    ],
                    "total_currency": [
                      "Ignore"
                    ]
                  },
                  "lock_checkpoint": [
                    "Ignore"
                  ],
                  "seed": [
                    "Ignore"
                  ],
                  "start_checkpoint": [
                    "Ignore"
                  ]
                },
                "total_currency": [
                  "Ignore"
                ]
              },
              "valid_while": [
                "Ignore"
              ]
            },
            "public_key": "B62qjSHAcwTouw5pxYECuJSFtmG6xup3DeK6f5BWW3BBhvEumW6daEm",
            "token_id": "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
            "update": {
              "app_state": [
                [
                  "Set",
                  "0x1513A94458106D79124BD9251708B62F511581ED00983E90AA7C125FDA08A9F8"
                ],
                [
                  "Set",
                  "0x0000000000000000000000000000000000000000000000000000000000000000"
                ],
                [
                  "Keep"
                ],
                [
                  "Set",
                  "0x1B4F433A35A59849FC94ACB07B73487A0C6D204F99B16E9AC7C6EF786F67BDB6"
                ],
                [
                  "Set",
                  "0x0000000000000000000000000000000000000000000000000000000000000000"
                ],
                [
                  "Keep"
                ],
                [
                  "Keep"
                ],
                [
                  "Keep"
                ]
              ],
              "delegate": [
                "Keep"
              ],
              "permissions": [
                "Keep"
              ],
              "timing": [
                "Keep"
              ],
              "token_symbol": [
                "Keep"
              ],
              "verification_key": [
                "Keep"
              ],
              "voting_for": [
                "Keep"
              ],
              "zkapp_uri": [
                "Keep"
              ]
            },
            "use_full_commitment": false
          }
        },
        "account_update_digest": "0x0BAFE556B3706E6A1E4AF4FDFCEC5CB5A3C66696EC987451229CAB2433AE754A",
        "calls": []
      },
      "stack_hash": "0x24FCD22A629D5A3B078514C990F5EF78843459E0EC0DA4FBDB8E7FA64D8EA8CE"
    }
  ],
  "fee_payer": {
    "authorization": "7mX2MVezL3QtFuuWvw7EEYd1gF2kTkLwPVkScYmiZFc3qg8dJtTdbNe2jmc3zPaioMQ2yXesdRfnYPfDaH7hicQJPC9MxEHQ",
    "body": {
      "fee": "0.005",
      "nonce": "190",
      "public_key": "B62qp4Wa3FxifJZJVeKWZEWUkGnuhbRwRiEogHhJTijUGYvH79CV72H",
      "valid_until": null
    }
  },
  "memo": "E4YrkRobQVqEQff65f7rcUE6x9zgK6XuLx7wzbGEs6E5Fi194KTzd"
}"#;

        assert_eq!(json[0], expect);
        Ok(())
    }

    #[test]
    fn zkapp_command_to_mina_json_compatibility_1() -> anyhow::Result<()> {
        use serde_json::*;

        // indexer zkapp command json
        let block_file = PathBuf::from("./tests/data/misc_blocks/mainnet-410535-3NLLmswaSwYVSERiQMdvTdKdBN6TNMgUGmd548zK7e82CaS3tNJK.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let indexer_json = precomputed_block
            .commands()
            .into_iter()
            .filter_map(|cmd| {
                if !cmd.is_zkapp_command() {
                    // filter out non-zkapp commands
                    return None;
                }

                let cmd: SignedCommand = cmd.into();
                let json: Value = cmd.into();

                Some(json)
            })
            .collect::<Vec<_>>();

        // mina zkapp command json
        // first user command is the only zkapp command
        let contents = std::fs::read(block_file)?;
        let mina_json: Value = from_slice::<Value>(&contents)?["data"]["staged_ledger_diff"]
            ["diff"][0]["commands"][0]["data"][1] // remove the Zkapp_command constructor
            .clone();

        assert_eq!(indexer_json[0], mina_json);
        Ok(())
    }

    #[test]
    fn zkapp_command_to_mina_json_compatibility_2() -> anyhow::Result<()> {
        use serde_json::*;

        // indexer zkapp command json
        let block_file = PathBuf::from("./tests/data/misc_blocks/mainnet-397612-3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK.json");
        let precomputed_block = PrecomputedBlock::parse_file(&block_file, PcbVersion::V2).unwrap();
        let indexer_json = precomputed_block
            .commands()
            .into_iter()
            .filter_map(|cmd| {
                if !cmd.is_zkapp_command() {
                    // filter out non-zkapp commands
                    return None;
                }

                let cmd: SignedCommand = cmd.into();
                let json: Value = cmd.into();

                Some(json)
            })
            .collect::<Vec<_>>();

        // mina zkapp command json
        // 12th "pre-diff" & 6th "post-diff" user commands are the only zkapp commands
        let contents = std::fs::read(block_file)?;
        let mina_zkapp_json_0: Value = from_slice::<Value>(&contents)?["data"]
            ["staged_ledger_diff"]["diff"][0]["commands"][11]["data"][1]
            .clone();
        let mina_zkapp_json_1: Value = from_slice::<Value>(&contents)?["data"]
            ["staged_ledger_diff"]["diff"][1]["commands"][5]["data"][1]
            .clone();

        assert_eq!(indexer_json[0], mina_zkapp_json_0);
        assert_eq!(indexer_json[1], mina_zkapp_json_1);

        Ok(())
    }
}
