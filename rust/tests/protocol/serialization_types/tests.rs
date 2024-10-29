// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(test, feature = "browser"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use crate::{
    block_path_test, block_path_test_batch, block_sum_path_test,
    protocol::fixtures::test::TEST_BLOCKS,
};
use mina_indexer::protocol::{
    bin_prot::*,
    serialization_types::{
        blockchain_state::{BlockchainStateV1, NonSnarkStagedLedgerHashV1, StagedLedgerHashV1},
        common::*,
        consensus_state::{ConsensusStateV1, VrfOutputTruncatedV1},
        epoch_data::{EpochDataV1, EpochLedgerV1},
        field_and_curve_elements::{FieldElement, InnerCurveScalar},
        global_slot::GlobalSlotV1,
        protocol_constants::ProtocolConstantsV1,
        protocol_state::*,
        protocol_state_body::ProtocolStateBodyV1,
        protocol_version::ProtocolVersionV1,
        signatures::{PublicKey2V1, PublicKeyV1, SignatureV1},
        staged_ledger_diff::{
            CoinBaseFeeTransferV1, CoinBaseV1, InternalCommandBalanceDataV1, PaymentPayloadV1,
            SignedCommandFeeTokenV1, SignedCommandMemoV1, SignedCommandPayloadBodyV1,
            SignedCommandPayloadCommonV1, SignedCommandV1, TransactionStatusAuxiliaryDataV1,
            TransactionStatusBalanceDataV1, TransactionStatusV1, UserCommandV1,
            UserCommandWithStatusV1,
        },
    },
};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use std::{any::TypeId, str::FromStr};
use wasm_bindgen_test::*;

#[test]
#[wasm_bindgen_test]
fn test_protocol_state() {
    block_path_test_batch! {
        ProtocolStateV1 => "t/protocol_state"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_previous_state_hash() {
    block_path_test_batch! {
        HashV1 => "t/protocol_state/t/t/previous_state_hash"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body() {
    block_path_test_batch! {
        ProtocolStateBodyV1 => "t/protocol_state/t/t/body"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_genesis_state_hash() {
    block_path_test_batch! {
        HashV1 => "t/protocol_state/t/t/body/t/t/genesis_state_hash"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_blockchain_state() {
    block_path_test_batch! {
        HashV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/snarked_ledger_hash"
        HashV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/genesis_ledger_hash"
        TokenIdV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/snarked_next_available_token"
        BlockTimeV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/timestamp"
        BlockchainStateV1 => "t/protocol_state/t/t/body/t/t/blockchain_state"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_blockchain_state_staged_ledger_hash() {
    block_path_test_batch! {
        HashV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash/t/t/non_snark/t/ledger_hash"
        ByteVecV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash/t/t/non_snark/t/aux_hash"
        ByteVecV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash/t/t/non_snark/t/pending_coinbase_aux"
        NonSnarkStagedLedgerHashV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash/t/t/non_snark"
        Hash2V1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash/t/t/pending_coinbase_hash"
        StagedLedgerHashV1 => "t/protocol_state/t/t/body/t/t/blockchain_state/t/t/staged_ledger_hash"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_consensus_state() {
    block_path_test_batch! {
        LengthV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/blockchain_length"
        LengthV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/epoch_count"
        LengthV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/min_window_density"
        Vec<LengthV1> => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/sub_window_densities"
        VrfOutputTruncatedV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/last_vrf_output"
        AmountV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/total_currency"
        GlobalSlotV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/curr_global_slot"
        GlobalSlotNumberV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/global_slot_since_genesis"
        EpochDataV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/staking_epoch_data"
        EpochDataV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/next_epoch_data"
        bool => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/has_ancestor_in_same_checkpoint_window"
        PublicKeyV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/block_stake_winner"
        ConsensusStateV1 => "t/protocol_state/t/t/body/t/t/consensus_state"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_consensus_state_staking_epoch_data() {
    block_path_test_batch! {
        EpochLedgerV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/staking_epoch_data/t/t/ledger"
        HashV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/staking_epoch_data/t/t/seed"
        EpochDataV1 => "t/protocol_state/t/t/body/t/t/consensus_state/t/t/staking_epoch_data"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_protocol_state_body_constants() {
    block_path_test_batch! {
        LengthV1 => "t/protocol_state/t/t/body/t/t/constants/t/t/k"
        LengthV1 => "t/protocol_state/t/t/body/t/t/constants/t/t/slots_per_epoch"
        LengthV1 => "t/protocol_state/t/t/body/t/t/constants/t/t/slots_per_sub_window"
        DeltaV1 => "t/protocol_state/t/t/body/t/t/constants/t/t/delta"
        BlockTimeV1 => "t/protocol_state/t/t/body/t/t/constants/t/t/genesis_state_timestamp"
        ProtocolConstantsV1 => "t/protocol_state/t/t/body/t/t/constants"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands() {
    block_path_test_batch! {
        UserCommandWithStatusV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0"
        Vec<UserCommandWithStatusV1> => "t/staged_ledger_diff/t/diff/t/0/t/t/commands"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_data() {
    block_path_test_batch! {
        SignedCommandV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/[sum]"
        UserCommandV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_data_payload_common() {
    block_path_test_batch! {
        AmountV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/fee"
        SignedCommandFeeTokenV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/fee_token"
        PublicKeyV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/fee_payer_pk"
        ExtendedU32 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/nonce"
        i32 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/valid_until/t/t"
        ExtendedU32 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/valid_until"
        SignedCommandMemoV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common/t/t/t/memo"
        SignedCommandPayloadCommonV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/common"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_data_payload_body() {
    block_path_test_batch! {
       PublicKeyV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0/t/t/source_pk"
       PublicKeyV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0/t/t/receiver_pk"
       u64 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0/t/t/token_id/t/t/t"
       ExtendedU64_3 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0/t/t/token_id"
       AmountV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0/t/t/amount"
       PaymentPayloadV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body/t/t/0"
       SignedCommandPayloadBodyV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/payload/t/t/body"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_data_signer() {
    block_path_test_batch! {
        PublicKey2V1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/signer"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_data_signature() {
    block_path_test_batch! {
        FieldElement => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/signature/t/t/0"
        InnerCurveScalar => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/signature/t/t/1"
    }

    block_path_test_batch! {
        (FieldElement, InnerCurveScalar) => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/signature/t/t"
        SignatureV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/data/t/t/0/t/t/signature"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_commands_status() {
    block_path_test_batch! {
        TransactionStatusAuxiliaryDataV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/status/t/0"
        TransactionStatusBalanceDataV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/status/t/1"
        TransactionStatusAuxiliaryDataV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/status/t/[sum]/0"
        TransactionStatusBalanceDataV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/status/t/[sum]/1"
        TransactionStatusV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/commands/0/t/status"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_coinbase() {
    block_path_test_batch! {
        CoinBaseV1 => "t/staged_ledger_diff/t/diff/t/0/t/t/coinbase"
    }
    block_sum_path_test!(
        "t/staged_ledger_diff/t/diff/t/0/t/t/coinbase/t/[sum]",
        Option<CoinBaseFeeTransferV1>,
        // other variant (dummy)
        // replace this with the actual types
        // once CoinBase::Zero and CoinBase::Two are implemented,
        DummyEmptyVariant,
    );
}

#[test]
#[wasm_bindgen_test]
fn test_staged_ledger_diff_diff_internal_command_balances() {
    block_path_test_batch! {
        Vec<InternalCommandBalanceDataV1> => "t/staged_ledger_diff/t/diff/t/0/t/t/internal_command_balances"
    }
}

#[test]
#[wasm_bindgen_test]
fn test_all_block_subtypes() {
    ////////////////////////////////////////////////////////////////
    // Here is where to add calls to test_in_block for every type
    // that has a strongly typed implementation to test
    ////////////////////////////////////////////////////////////////
    block_path_test_batch! {
        ProtocolVersionV1 => "t/current_protocol_version"
        Option<ProtocolVersionV1> => "t/proposed_protocol_version_opt"
        HashV1 => "t/protocol_state/t/t/previous_state_hash"
    }
}

#[test]
#[wasm_bindgen_test]
fn smoke_test_roundtrip_block1() {
    let block = TEST_BLOCKS.get("block1").expect("Failed to load block1");

    // test we can correctly index a known field
    assert_eq!(
        block.value["t"]["protocol_state"]["t"]["t"]["previous_state_hash"]["t"],
        Value::Tuple(
            [
                30, 76, 197, 215, 115, 43, 42, 245, 198, 30, 253, 134, 49, 117, 82, 71, 182, 181,
                180, 95, 18, 250, 46, 1, 25, 3, 78, 193, 57, 152, 116, 49
            ]
            .iter()
            .map(|c| Value::Char(*c))
            .collect()
        )
    );

    // check roundtrip
    test_roundtrip(&block.value, block.bytes.as_slice());
}

pub(crate) fn select_path(block: &Value, path: impl AsRef<str>) -> &Value {
    // pull out the bin_prot::Value corresponding to the path
    // will panic if the path is invalid
    let path_ref = path.as_ref();
    if path_ref.is_empty() {
        return block;
    }
    let mut val = block;
    for p in path_ref.split('/') {
        if p == "[sum]" {
            match val {
                Value::Sum {
                    ref value, index, ..
                } => {
                    println!("Unpacking sum type index {index} for {path_ref}");
                    val = value;
                }
                _ => panic!("Sum expected"),
            }
        } else {
            val = match usize::from_str(p) {
                Ok(index) => &val[index],
                _ => &val[p],
            };
        }
    }
    val
}

fn test_in_block_ensure_empty(block: &Value, paths: &[&str]) {
    for path in paths {
        let val = select_path(block, path);

        let mut bytes = vec![];
        to_writer(&mut bytes, val)
            .map_err(|err| {
                format!(
                    "Failed writing bin-prot encoded data, err: {err}\npath: {path}\ndata: {:?}",
                    val
                )
            })
            .unwrap();
        assert_eq!(bytes.len(), 0, "path: {}\ndata: {:#?}", path, val);
    }
}

fn test_in_block<'a, T: Serialize + Deserialize<'a>>(block: &Value, paths: &[&str]) {
    for path in paths {
        let val = select_path(block, path);

        // write to binary then deserialize into T
        let mut bytes = vec![];
        to_writer(&mut bytes, val)
            .map_err(|err| {
                format!(
                    "Failed writing bin-prot encoded data, err:{err}\npath: {path}\ndata: {:?}",
                    val
                )
            })
            .unwrap();
        let re_val: T = from_reader_strict(bytes.as_slice())
            .map_err(|err| {
                format!(
                    "Could not deserialize type, err:{err}\npath: {}\nbytes({}): {:?}\ndata: {:?}",
                    path,
                    bytes.len(),
                    bytes,
                    val
                )
            })
            .unwrap();
        // serialize back to binary and ensure it matches
        let mut re_bytes = vec![];
        to_writer(&mut re_bytes, &re_val)
            .map_err(|err| {
                format!(
                    "Failed writing bin-prot encoded data, err: {err}\npath: {path}\ndata: {:?}",
                    val
                )
            })
            .unwrap();

        assert_eq!(bytes, re_bytes, "path: {}\ndata: {:?}", path, val);
    }
}

fn test_roundtrip<T>(val: &T, bytes: &[u8])
where
    T: Serialize,
{
    let mut output = vec![];
    to_writer(&mut output, val).expect("Failed writing bin-prot encoded data");
    assert_eq!(bytes, output)
}

// This is introduced to support `block_sum_path_test`
// match a given path to CoinBase::Zero which is an empty variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DummyEmptyVariant;
