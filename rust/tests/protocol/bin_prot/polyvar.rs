// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

use crate::protocol::bin_prot::polyvar::TestPolyvar::{None, One, Two};
use mina_indexer::protocol::bin_prot::{error::Error, to_writer, Deserializer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename = "Polyvar")]
enum TestPolyvar {
    // Hash repr 870530776_u32
    // ocaml native integer 1741061553_u32
    #[serde(rename = "None")]
    None,
    // Hash repr 3953222_u32
    // ocaml native integer 7906445_u32
    #[serde(rename = "One")]
    One(bool),
    // Hash repr 4203884_u32
    // ocaml native integer 8407769_u32
    #[serde(rename = "Two")]
    Two(TestPolyvar2),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename = "Polyvar")]
enum TestPolyvar2 {
    // Hash repr 925978388_u32
    // ocaml native integer 1851956777_u32
    #[serde(rename = "Some")]
    SomeVariant,
}

#[test]
fn test_polyvar_variant_none() {
    let tag = 1741061553_u32.to_le_bytes();
    let mut de = Deserializer::from_reader(tag.as_slice());
    let result: TestPolyvar = Deserialize::deserialize(&mut de).expect("Failed to deserialize");
    assert_eq!(result, None);

    let mut re_bytes = vec![];
    to_writer(&mut re_bytes, &result).unwrap();
    assert_eq!(tag, re_bytes[..]);
}

#[test]
fn test_polyvar_variant_one() {
    let tag = 7906445_u32.to_le_bytes();
    let value = 0x01_u32.to_le_bytes();
    let data: Vec<u8> = [tag, value].concat();

    let mut de = Deserializer::from_reader(data.as_slice());
    let result: TestPolyvar = Deserialize::deserialize(&mut de).expect("Failed to deserialize");
    assert_eq!(result, One(true));

    let mut re_bytes = vec![];
    to_writer(&mut re_bytes, &result).unwrap();
    assert_eq!(data[..5], re_bytes[..]);
}

#[test]
fn test_polyvar_variant_two() {
    let tag = 8407769_u32.to_le_bytes();
    let value = 1851956777_u32.to_le_bytes();
    let data: Vec<u8> = [tag, value].concat();

    let mut de = Deserializer::from_reader(data.as_slice());
    let result: TestPolyvar = Deserialize::deserialize(&mut de).expect("Failed to deserialize");
    assert_eq!(result, Two(TestPolyvar2::SomeVariant));

    let mut re_bytes = vec![];
    to_writer(&mut re_bytes, &result).unwrap();
    assert_eq!(data, re_bytes[..]);
}

#[test]
fn test_polyvar_unknown_polyvar_tag() {
    let tag = 1234567_u32.to_le_bytes(); // random hash
    let mut de = Deserializer::from_reader(tag.as_slice());
    let result: Result<TestPolyvar, Error> = Deserialize::deserialize(&mut de);
    assert!(result.is_err())
}
