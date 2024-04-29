// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//!
//! All human-readable values (e.g. base58 encoded hashes and addresses)
//! implement the Base58Checked encoding <https://en.bitcoin.it/wiki/Base58Check_encoding>
//!
//! This adds a unique prefix byte to each type of encoding, so they cannot be
//! confused (e.g. a hash cannot be used as an address). It also adds checksum
//! bytes to the end.
//!
//! See compatible (mainnet):
//!   https://github.com/MinaProtocol/mina/blob/compatible/src/lib/base58_check/version_bytes.ml
//! and berkeley:
//!   https://github.com/MinaProtocol/mina/blob/berkeley/src/lib/base58_check/version_bytes.ml

/// base58 version check byte for coinbase hash
pub const COINBASE_HASH: u8 = 0x01;

/// base58 version check byte for secret box byteswr
pub const SECRET_BOX_BYTESWR: u8 = 0x02;

/// base58 version check byte for fee transfer single
pub const FEE_TRANSFER_SINGLE: u8 = 0x03;

/// base58 version check byte for frontier hash
pub const FRONTIER_HASH: u8 = 0x04;

/// base58 version check byte for ledger hash
pub const LEDGER_HASH: u8 = 0x05;

/// base58 version check byte for lite precomputed
pub const LITE_PRECOMPUTED: u8 = 0x06;

/// base58 version check byte for proof
pub const PROOF: u8 = 0x0a;

/// base58 version check byte for random oracle base
pub const RANDOM_ORACLE_BASE: u8 = 0x0b;

/// base58 version check byte for receipt chain hash
pub const RECEIPT_CHAIN_HASH: u8 = 0x0c;

/// base58 version check byte for epoch seed hash
pub const EPOCH_SEED: u8 = 0x0d;

/// base58 version check byte for aux hash
pub const STAGED_LEDGER_HASH_AUX_HASH: u8 = 0x0e;

/// base58 version check byte for pending coinbase aux hash
pub const STAGED_LEDGER_HASH_PENDING_COINBASE_AUX: u8 = 0x0f;

/// base58 version check byte for state hash
pub const STATE_HASH: u8 = 0x10;

/// base58 version check byte for state body hash
pub const STATE_BODY_HASH: u8 = 0x11;

/// base58 version check byte for v1 (mainnet) txn hash
pub const V1_TXN_HASH: u8 = 0x12;

/// base58 version check byte for v2 (berkeley) txn hash
pub const V2_TXN_HASH: u8 = 0x1d;

/// base58 version check byte for user command
pub const USER_COMMAND: u8 = 0x13;

/// base58 version check byte for user command memo
pub const USER_COMMAND_MEMO: u8 = 0x14;

/// base58 version check byte for vrf output hash
pub const VRF_TRUNCATED_OUTPUT: u8 = 0x15;

/// base58 version check byte for web pipe
pub const WEB_PIPE: u8 = 0x16;

/// base58 version check byte for coinbase stack data
pub const COINBASE_STACK_DATA: u8 = 0x17;

/// base58 version check byte for coinbase stack hash
pub const COINBASE_STACK_HASH: u8 = 0x18;

/// base58 version check byte for pending coinbase hash builder
pub const PENDING_COINBASE_HASH_BUILDER: u8 = 0x19;

/// base58 version check byte for zkapp (aka snapp) command
pub const ZKAPP_COMMAND: u8 = 0x1a;

/// base58 version check byte for verification key
pub const VERIFICATION_KEY: u8 = 0x1b;

/// base58 version check byte for token id key
pub const TOKEN_ID_KEY: u8 = 0x1c;

/// base58 version check byte for private key
pub const PRIVATE_KEY: u8 = 0x5a;

/// base58 version check byte for non-zero curve point compressed
pub const NON_ZERO_CURVE_POINT_COMPRESSED: u8 = 0xcb;

/// base58 version check byte for signature
pub const SIGNATURE: u8 = 0x9a;
