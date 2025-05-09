// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

#![deny(warnings)]
#![deny(missing_docs)]
#![doc = include_str!("README.md")]
#![allow(clippy::needless_lifetimes)]

mod consts;
mod de;
pub mod error;
#[cfg(feature = "loose_deserialization")]
mod loose_deserializer;
mod polyvar;
mod read_ext;
mod ser;
pub mod value;
mod write_ext;

pub use de::{from_reader, from_reader_strict, Deserializer};
pub use polyvar::{caml_hash_variant, VariantHash};
pub use read_ext::ReadBinProtExt;
pub use ser::{to_writer, Serializer};
#[cfg(feature = "loose_deserialization")]
pub use value::layout::{BinProtRule, Layout};
pub use value::Value;
pub use write_ext::WriteBinProtExt;
