// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//! Types and functions related to the Mina consensus state

use crate::protocol::serialization_types::{
    common::{AmountV1, GlobalSlotNumberV1, LengthV1, U32Json, U64Json},
    epoch_data::{EpochDataJson, EpochDataV1},
    global_slot::{GlobalSlotJson, GlobalSlotV1},
    signatures::{PublicKeyJson, PublicKeyV1},
    version_bytes,
};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use mina_serialization_proc_macros::AutoFrom;
use mina_serialization_versioned::{Versioned, Versioned2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Wrapper struct for the output for a VRF
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VrfOutputTruncated(pub Vec<u8>);

/// Wrapper struct for the output for a VRF, with version
pub type VrfOutputTruncatedV1 = Versioned<VrfOutputTruncated, 1>;

/// Wrapper struct for the output for a VRF (base64 encoded json)
#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(VrfOutputTruncated)]
pub struct VrfOutputTruncatedBase64Json(pub Vec<u8>);

impl Serialize for VrfOutputTruncatedBase64Json {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = BASE64_URL_SAFE.encode(self.0.as_slice());
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for VrfOutputTruncatedBase64Json {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(
            BASE64_URL_SAFE
                .decode(s)
                .map_err(<D::Error as serde::de::Error>::custom)?,
        ))
    }
}

/// Wrapper struct for the output for a VRF (base64 encoded json)
#[derive(Clone, Debug, Eq, PartialEq, AutoFrom)]
#[auto_from(VrfOutputTruncated)]
pub struct VrfOutputTruncatedBase58Json(pub Vec<u8>);

impl Serialize for VrfOutputTruncatedBase58Json {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut input = Vec::with_capacity(self.0.len() + 1);
        input.push(self.0.len() as u8);
        input.extend_from_slice(self.0.as_slice());
        let s = bs58::encode(input.as_slice())
            .with_check_version(version_bytes::VRF_TRUNCATED_OUTPUT)
            .into_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for VrfOutputTruncatedBase58Json {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decoded = bs58::decode(s)
            .with_check(Some(version_bytes::VRF_TRUNCATED_OUTPUT))
            .into_vec()
            .map_err(<D::Error as serde::de::Error>::custom)?;
        Ok(Self(decoded.into_iter().skip(2).collect()))
    }
}

/// This structure encapsulates the succinct state of the consensus protocol.
///
/// The stake distribution information is contained by the staking_epoch_data
/// field.
///
/// Due to its succinct nature, Samasika cannot look back into the past to
/// obtain ledger snapshots for the stake distribution. Instead, Samasika
/// implements a novel approach where the future stake distribution snapshot is
/// prepared by the current consensus epoch.
///
/// Samasika prepares the past for the future! This future state is stored in
/// the next_epoch_data field.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ConsensusState {
    /// Height of block
    pub blockchain_length: LengthV1,
    /// Epoch number
    pub epoch_count: LengthV1,
    /// Minimum window density oberved on the chain
    pub min_window_density: LengthV1,
    /// Current sliding window of densities
    pub sub_window_densities: Vec<LengthV1>,
    /// Additional VRS output from leader (for seeding Random Oracle)
    pub last_vrf_output: VrfOutputTruncatedV1,
    /// Total supply of currency
    pub total_currency: AmountV1,
    /// Current global slot number relative to the current hard fork
    pub curr_global_slot: GlobalSlotV1,
    /// Absolute global slot number since genesis
    pub global_slot_since_genesis: GlobalSlotNumberV1,
    /// Epoch data for previous epoch
    pub staking_epoch_data: EpochDataV1,
    /// Epoch data for current epoch
    pub next_epoch_data: EpochDataV1,
    /// If the block has an ancestor in the same checkpoint window
    pub has_ancestor_in_same_checkpoint_window: bool,
    /// Compressed public key of winning account
    pub block_stake_winner: PublicKeyV1,
    /// Compressed public key of the block producer
    pub block_creator: PublicKeyV1,
    /// Compresed public key of account receiving the block reward
    pub coinbase_receiver: PublicKeyV1,
    /// true if block_stake_winner has no locked tokens, false otherwise
    pub supercharge_coinbase: bool,
}

/// V1 protocol version of the consensus state
pub type ConsensusStateV1 = Versioned2<ConsensusState, 1, 1>;

/// json protocol version of the consensus state
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, AutoFrom)]
#[auto_from(ConsensusState)]
pub struct ConsensusStateJson {
    /// Height of block
    pub blockchain_length: U32Json,
    /// Epoch number
    pub epoch_count: U32Json,
    /// Minimum window density oberved on the chain
    pub min_window_density: U32Json,
    /// Current sliding window of densities
    pub sub_window_densities: Vec<U32Json>,
    /// Additional VRS output from leader (for seeding Random Oracle)
    pub last_vrf_output: VrfOutputTruncatedBase64Json,
    /// Total supply of currency
    pub total_currency: U64Json,
    /// Current global slot number relative to the current hard fork
    pub curr_global_slot: GlobalSlotJson,
    /// Absolute global slot number since genesis
    pub global_slot_since_genesis: U32Json,
    /// Epoch data for previous epoch
    pub staking_epoch_data: EpochDataJson,
    /// Epoch data for current epoch
    pub next_epoch_data: EpochDataJson,
    /// If the block has an ancestor in the same checkpoint window
    pub has_ancestor_in_same_checkpoint_window: bool,
    /// Compressed public key of winning account
    pub block_stake_winner: PublicKeyJson,
    /// Compressed public key of the block producer
    pub block_creator: PublicKeyJson,
    /// Compresed public key of account receiving the block reward
    pub coinbase_receiver: PublicKeyJson,
    /// true if block_stake_winner has no locked tokens, false otherwise
    pub supercharge_coinbase: bool,
}
