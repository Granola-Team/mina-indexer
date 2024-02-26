// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

use crate::{load_json_test_blocks, load_test_blocks};
use lazy_static::lazy_static;
use mina_indexer::protocol::{
    bin_prot, bin_prot::value::layout::BinProtRule,
    serialization_types::external_transition::ExternalTransitionV1,
};
use serde::Deserialize;
use std::collections::HashMap;

pub struct BlockFixture {
    pub bytes: Vec<u8>,
    pub value: bin_prot::Value,
    pub block_name: &'static str,
}

impl BlockFixture {
    pub fn external_transitionv1(&self) -> anyhow::Result<ExternalTransitionV1> {
        Ok(bin_prot::from_reader_strict(self.bytes.as_slice())?)
    }
}

// FIXME: Move layouts into this crate?
pub const BLOCK_LAYOUT: &str = include_str!("../layouts/external_transition.json");

lazy_static! {
    pub static ref BLOCK_RULE: BinProtRule = {
        let mut deserializer = serde_json::Deserializer::from_str(BLOCK_LAYOUT);
        deserializer.disable_recursion_limit();
        let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
        bin_prot::Layout::deserialize(deserializer)
            .unwrap()
            .bin_prot_rule
    };

     pub static ref TEST_BLOCKS: HashMap<String, BlockFixture> = load_test_blocks!(
        "data/block1"
        "data/3NK3P5bJHhqR7xkZBquGGfq3sERUeXNYNma5YXRMjgCNsTJRZpgL.hex"
        "data/3NK6nkk9t23KNHTZ92M77ebpv1nzvFwQLow1DHS4eDNa2bRhtsPd.hex"
        "data/3NKaBJsN1SehD6iJwRwJSFmVzJg5DXSUQVgnMxtH4eer4aF5BrDK.hex"
        "data/3NKf5nZYFZ4qMe1VyysbsnzgA1pm4i93NBt6ytNC7CdU2QbmdRZC.hex"
        "data/3NKCdqMnTzqxLhpG218eKZSUxB1GMqdiiBjbWXrQVrVzz8mTNFrK.hex"
        "data/3NKEWdhLxBuPboanSMXgNxudXqowm9gLNzhyfQMUBM2L52wSjb6z.hex"
        "data/3NLJmFT3nvnatDEZfqZzJ7k9fJYApoe1SPQVUggS24ViZT7z8aNA.hex"
        "data/3NLRkk5T3Gaf8ZPXgoXatdrtpN3hUdzWPWXbrqMo4jVsi3jkiGE5.hex"
        "data/3NLvrNK6rmWnxEkGZo1y4KYjsSTcgVx7gwen2aR2kTWmRDTNoSu8.hex"
        "data/3NK9fHpzfPWhuxFhQ9Dau1X1JWtstB6kGC4xrurSPU1kctMCsU9U.hex"
        "data/3NKapQX5Qe8f4BEZGWxVSWKQvKNnkvPXNLq5KDHCV1qoPzV5Y3Wu.hex"
        "data/3NKjZ5fjms6BMaH4aq7DopPGyMY7PbG6vhRsX5XnYRxih8i9G7dj.hex"
    );

    // Note that GENESIS_BLOCK_MAINNET_JSON has a different json format, so it's not included here
    pub static ref JSON_TEST_BLOCKS: HashMap<String, serde_json::Value> = load_json_test_blocks!(
        "data/mainnet-117896-3NKrv92FYZFHRNUJxiP7VGeRx3MeDY2iffFjUWXTPoXJorsS63ba.json"
        "data/mainnet-117896-3NKjZ5fjms6BMaH4aq7DopPGyMY7PbG6vhRsX5XnYRxih8i9G7dj.json"
        "data/mainnet-116121-3NK6myZRzc3GvS5iydv88on2XTEU2btYrjMVkgtbuoeXASRipSa6.json"
        "data/mainnet-77749-3NK3P5bJHhqR7xkZBquGGfq3sERUeXNYNma5YXRMjgCNsTJRZpgL.json"
        "data/mainnet-77748-3NKaBJsN1SehD6iJwRwJSFmVzJg5DXSUQVgnMxtH4eer4aF5BrDK.json"
        "data/mainnet-113267-3NLenrog9wkiJMoA774T9VraqSUGhCuhbDLj3JKbEzomNdjr78G8.json"
        "data/mainnet-147571-3NKwrze6FvGQCCF6L7Q2JLvwgnsm56hwSny9kUyjbSUr8oqu1MGp.json"
        "data/mainnet-149909-3NLCeY7UwgCryuvk3Wevm9ndMDvWAMjwGBfBJS12MqL1QoTQWEWt.json"
        "data/mainnet-113267-3NKtqqstB6h8SVNQCtspFisjUwCTqoQ6cC1KGvb6kx6n2dqKkiZS.json"
        "data/mainnet-117896-3NLPBDTckSdjcUFcQiE9raJsyzB84KayMPKi4PmwNybnA6J75GoL.json"
    );
}
