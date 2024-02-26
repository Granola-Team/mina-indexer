use crate::protocol::fixtures::test::{BlockFixture, BLOCK_RULE};
use mina_indexer::protocol::bin_prot::Deserializer;
use serde::Deserialize;
use std::borrow::Borrow;

#[macro_export]
macro_rules! load_json_test_blocks {
    ( $( $lt:literal $(,)?) * ) => {
        {
            let mut temp_map = HashMap::new();
            $(
                let file_name = $lt.split('/').last().unwrap().into();
                let mut block: serde_json::Value = serde_json::from_slice(include_bytes!($lt)).map_err(|err|format!("Error loading {}: {err}", $lt)).unwrap();
                // Remove scheduled_time field as it's not part of block
                if let Some(block_mut) = block.as_object_mut() {
                    block_mut.remove("scheduled_time");
                }
                temp_map.insert(file_name, block);
            )*
            temp_map
        }
    };
}

#[macro_export]
macro_rules! load_test_blocks {
    ( $( $lt:literal $(,)?) * ) => {
        {
            let mut temp_map = HashMap::new();
            $(
                let file_name = $lt.split('/').last().unwrap().into();
                let bytes = include_bytes!($lt);
                let block = $crate::protocol::macros::load_test_block($lt, bytes);
                temp_map.insert(file_name, block);
            )*
            temp_map
        }
    };
}

#[macro_export]
macro_rules! block_path_test {
    ($typ:ty, $path:expr) => {
        for block in $crate::protocol::test_fixtures::TEST_BLOCKS.values() {
            let start = std::time::Instant::now();
            test_in_block::<$typ>(&block.value, &[$path]);
            println!(
                "block {} duration {:?}",
                block.block_name,
                std::time::Instant::now() - start,
            );
        }
    };
}

#[macro_export]
macro_rules! block_path_test_batch {
    ($($typ:ty => $path:expr) *)  => {
        $(
            block_path_test!($typ, $path);
        )*
    };
}

#[macro_export]
macro_rules! block_sum_path_test {
    ($path:expr, $($typ:ty,)*) => {
        for block in TEST_BLOCKS.values() {
            println!("Testing block {}", block.block_name);
            let mut success = 0;
            $(
                if TypeId::of::<$typ>() == TypeId::of::<DummyEmptyVariant>() {
                    if std::panic::catch_unwind(|| test_in_block_ensure_empty(&block.value, &[$path])).is_ok() {
                        success += 1;
                    }
                } else if std::panic::catch_unwind(|| test_in_block::<$typ>(&block.value, &[$path])).is_ok() {
                    success += 1;
                }
            )*
            assert_eq!(success, 1, "Failing block: {}", block.block_name);
        }
    };
}

/// Macro that lets one use jane street's bin prot expect test cases blocks
/// and assert using serde-bin-prot's APIs.
#[macro_export]
macro_rules! bin_prot_test {
    ($($(..) * $($expected:literal) * -> $typ:expr),*)  => {
        $(
        let mut output = vec![];
        mina_indexer::protocol::bin_prot::to_writer(&mut output, &$typ).expect("Failed writing bin-prot encoded data");
        assert_eq!(vec![$($expected,)*], output.into_iter().rev().collect::<Vec<u8>>());
        )*
    };
}

pub fn load_test_block(block_name: &'static str, bytes: &'static [u8]) -> BlockFixture {
    let mut de = Deserializer::from_reader(bytes).with_layout(&BLOCK_RULE);
    match Deserialize::deserialize(&mut de) {
        Ok(value) => BlockFixture {
            bytes: bytes.into(),
            value,
            block_name,
        },
        Err(_) => load_test_block_hex(
            block_name,
            String::from_utf8(bytes.into())
                .expect("Failed to decode hex encoded block")
                .borrow(),
        ),
    }
}

pub fn load_test_block_hex(block_name: &'static str, hex_str: &str) -> BlockFixture {
    let bytes = hex::decode(hex_str).expect("Failed to decode hex encoded block");
    let mut de = Deserializer::from_reader(bytes.as_slice()).with_layout(&BLOCK_RULE);
    let value = Deserialize::deserialize(&mut de).expect("Failed to deserialize test block");
    BlockFixture {
        bytes,
        value,
        block_name,
    }
}
