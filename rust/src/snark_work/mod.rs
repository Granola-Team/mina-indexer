pub mod store;

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::public_key::PublicKey,
    protocol::serialization_types::snark_work as mina_rs,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnarkWorkSummary {
    pub fee: u64,
    pub prover: PublicKey,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnarkWorkTotal {
    pub total_fees: u64,
    pub prover: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnarkWork {
    pub fee: u64,
    pub proofs: serde_json::Value,
    pub prover: PublicKey,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnarkWorkSummaryWithStateHash {
    pub fee: u64,
    pub prover: PublicKey,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnarkWorkWithStateHash {
    pub fee: u64,
    pub proofs: serde_json::Value,
    pub prover: PublicKey,
    pub state_hash: String,
}

impl SnarkWorkSummary {
    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .completed_works()
            .into_iter()
            .map(Self::from)
            .collect()
    }

    pub fn contains_pk(&self, pk: &PublicKey) -> bool {
        &self.prover == pk
    }
}

impl SnarkWorkSummaryWithStateHash {
    pub fn from(snark: mina_rs::TransactionSnarkWork, state_hash: &BlockHash) -> Self {
        let snark: SnarkWorkSummary = snark.into();
        Self {
            fee: snark.fee,
            prover: snark.prover,
            state_hash: state_hash.to_string(),
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .completed_works()
            .into_iter()
            .map(|snark| Self::from(snark, &block.state_hash()))
            .collect()
    }

    pub fn contains_pk(&self, pk: &PublicKey) -> bool {
        &self.prover == pk
    }
}

// mina-rs conversions

impl From<mina_rs::TransactionSnarkWork> for SnarkWorkSummary {
    fn from(value: mina_rs::TransactionSnarkWork) -> Self {
        Self {
            fee: value.fee.inner().inner(),
            prover: value.prover.into(),
        }
    }
}

impl From<SnarkWorkSummaryWithStateHash> for SnarkWorkSummary {
    fn from(value: SnarkWorkSummaryWithStateHash) -> Self {
        Self {
            fee: value.fee,
            prover: value.prover,
        }
    }
}

// json conversions

impl From<SnarkWorkSummary> for serde_json::Value {
    fn from(value: SnarkWorkSummary) -> Self {
        use serde_json::*;

        let mut obj = Map::new();
        obj.insert("fee".into(), Value::Number(Number::from(value.fee)));
        obj.insert("prover".into(), Value::String(value.prover.into()));
        Value::Object(obj)
    }
}

impl From<SnarkWorkSummaryWithStateHash> for serde_json::Value {
    fn from(value: SnarkWorkSummaryWithStateHash) -> Self {
        use serde_json::*;

        let mut obj = Map::new();
        obj.insert("fee".into(), Value::Number(Number::from(value.fee)));
        obj.insert("prover".into(), Value::String(value.prover.into()));
        obj.insert("state_hash".into(), Value::String(value.state_hash));
        Value::Object(obj)
    }
}

impl std::fmt::Debug for SnarkWorkSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json: serde_json::Value = self.clone().into();
        write!(f, "{}", serde_json::to_string_pretty(&json).unwrap())
    }
}

impl std::fmt::Debug for SnarkWorkSummaryWithStateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json: serde_json::Value = self.clone().into();
        write!(f, "{}", serde_json::to_string_pretty(&json).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal::{prelude::ToPrimitive, Decimal};
    use rust_decimal_macros::dec;
    use serde_json::*;
    use std::path::PathBuf;

    #[test]
    fn from_precomputed() -> anyhow::Result<()> {
        fn convert_snark_work(value: Value) -> Value {
            match value {
                Value::String(s) => {
                    if let Ok(num) = s.parse::<Decimal>() {
                        Value::Number(Number::from((num * dec!(1_000_000_000)).to_u64().unwrap()))
                    } else {
                        Value::String(s)
                    }
                }
                Value::Object(mut obj) => {
                    obj.iter_mut()
                        .for_each(|(_, v)| *v = convert_snark_work(v.clone()));
                    Value::Object(obj)
                }
                x => x,
            }
        }
        fn remove_proofs(value: Value) -> Value {
            if let Value::Object(mut obj) = value {
                obj.remove("proofs");
                Value::Object(obj)
            } else {
                value
            }
        }
        fn add_state_hash(value: Value, state_hash: &BlockHash) -> Value {
            if let Value::Object(mut obj) = value {
                obj.insert("state_hash".into(), Value::String(state_hash.0.clone()));
                Value::Object(obj)
            } else {
                Value::Null
            }
        }

        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw
        let path: PathBuf = "./tests/data/non_sequential_blocks/mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw.json".into();
        let contents = std::fs::read(path.clone())?;
        let block = PrecomputedBlock::parse_file(&path)?;

        if let Value::Array(arr) = from_slice::<Value>(&contents)?["staged_ledger_diff"]["diff"][0]
            ["completed_works"]
            .clone()
        {
            // no proofs
            let completed_works_no_proofs_json = Value::Array(
                arr.clone()
                    .into_iter()
                    .map(|x| remove_proofs(convert_snark_work(x)))
                    .collect(),
            );
            let completed_works_no_proofs = SnarkWorkSummary::from_precomputed(&block);
            let completed_works_no_proofs: Value = completed_works_no_proofs.into();
            assert_eq!(completed_works_no_proofs_json, completed_works_no_proofs);

            // state hash
            let completed_works_state_hash_json = Value::Array(
                arr.into_iter()
                    .map(|x| {
                        remove_proofs(add_state_hash(convert_snark_work(x), &block.state_hash()))
                    })
                    .collect(),
            );
            let completed_works_state_hash =
                SnarkWorkSummaryWithStateHash::from_precomputed(&block);
            let completed_works_state_hash: Value = completed_works_state_hash.into();
            assert_eq!(completed_works_state_hash_json, completed_works_state_hash);
        } else {
            panic!("Expected SNARK work object")
        }

        // mainnet-111-3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK
        let path: PathBuf = "./tests/data/non_sequential_blocks/mainnet-111-3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK.json".into();
        let contents = std::fs::read(path.clone())?;
        let contents = String::from_utf8_lossy(&contents);
        let block = PrecomputedBlock::parse_file(&path)?;

        if let Value::Array(arr) = from_str::<Value>(&contents)?["staged_ledger_diff"]["diff"][1]
            ["completed_works"]
            .clone()
        {
            // no proofs
            let completed_works_no_proofs_json = Value::Array(
                arr.clone()
                    .into_iter()
                    .map(|x| remove_proofs(convert_snark_work(x)))
                    .collect(),
            );
            let completed_works_no_proofs = SnarkWorkSummary::from_precomputed(&block);
            let completed_works_no_proofs: Value = completed_works_no_proofs.into();
            assert_eq!(completed_works_no_proofs_json, completed_works_no_proofs);

            // state hash
            let completed_works_state_hash_json = Value::Array(
                arr.into_iter()
                    .map(|x| {
                        remove_proofs(add_state_hash(convert_snark_work(x), &block.state_hash()))
                    })
                    .collect(),
            );
            let completed_works_state_hash =
                SnarkWorkSummaryWithStateHash::from_precomputed(&block);
            let completed_works_state_hash: Value = completed_works_state_hash.into();
            assert_eq!(completed_works_state_hash_json, completed_works_state_hash);
        } else {
            panic!("Expected SNARK work object")
        }

        Ok(())
    }
}
