// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

use crate::protocol::bin_prot::{
    de::{Enum, LooselyTyped, MapAccess, SeqAccess},
    error::{Error, Result},
    to_writer,
    value::layout::{BinProtRule, Polyvar, Summand, TaggedPolyvar},
    Deserializer as DS, ReadBinProtExt,
};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{de::Visitor, Deserialize, Serialize};
use std::{convert::TryInto, io::Read};

impl<'de, R: Read> DS<R, LooselyTyped> {
    /// The loose deserializer version of deserialize
    /// Only implemented on the LooselyTyped variant of the Deserializer
    pub fn deserialize_loose<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.mode.layout_iter.next() {
            Some(rule) => {
                match rule {
                    BinProtRule::Unit => {
                        self.rdr.bin_read_unit()?;
                        visitor.visit_unit()
                    }
                    BinProtRule::Record(fields) => {
                        // Grab the field names from the rule to pass to the map access
                        visitor.visit_map(MapAccess::new(
                            self,
                            fields.into_iter().map(|f| f.field_name).rev().collect(),
                        ))
                    }
                    BinProtRule::Tuple(items) => {
                        visitor.visit_seq(SeqAccess::new(self, items.len()))
                    }
                    BinProtRule::Bool => visitor.visit_bool(self.rdr.bin_read_bool()?),
                    BinProtRule::Sum(summands) => {
                        // read the enum variant index.
                        // We need this to select which variant layout to use
                        // when deserializing the variants data
                        let index = self.rdr.bin_read_variant_index()?;
                        let variant_rules = summands[index as usize].ctor_args.clone();
                        self.mode
                            .layout_iter
                            .push(vec![BinProtRule::Tuple(variant_rules)]);
                        visitor.visit_enum(ValueEnum::new(
                            self,
                            VariantType::Sum(summands[index as usize].clone()),
                        ))
                    }
                    BinProtRule::Polyvar(summands) => {
                        let tag = self.rdr.bin_read_polyvar_tag()?;
                        let (index, variant) = summands
                            .into_iter()
                            .enumerate()
                            .find_map(|(i, v)| {
                                match v {
                                    Polyvar::Tagged(t) => {
                                        // return the first tagged variant where the tag matches
                                        if t.hash == tag {
                                            Some((i, t))
                                        } else {
                                            None
                                        }
                                    }
                                    Polyvar::Inherited(_) => unimplemented!(), /* don't know how
                                                                                * to handle these
                                                                                * yet */
                                }
                            })
                            .ok_or(Error::UnknownPolyvarTag(tag))?;
                        self.mode
                            .layout_iter
                            .push(vec![BinProtRule::Tuple(variant.clone().polyvar_args)]);
                        visitor.visit_enum(ValueEnum::new(
                            self,
                            VariantType::Polyvar(index as u8, variant),
                        ))
                    }
                    BinProtRule::Option(some_rule) => {
                        let index = self.rdr.bin_read_variant_index()?; // 0 or 1
                        match index {
                            0 => visitor.visit_none(),
                            1 => {
                                self.mode.layout_iter.push(vec![*some_rule]);
                                visitor.visit_some(self)
                            }
                            _ => Err(Error::InvalidOptionByte { got: index }),
                        }
                    }
                    BinProtRule::String => visitor.visit_bytes(&self.rdr.bin_read_bytes()?),
                    BinProtRule::Float => visitor.visit_f64(self.rdr.read_f64::<LittleEndian>()?),
                    BinProtRule::Char => {
                        let c = self.rdr.read_u8()?;
                        visitor.visit_char(c as char)
                    }
                    BinProtRule::List(element_rule) => {
                        // read the length
                        let len = self.rdr.bin_read_nat0()?;
                        // request the iterator repeats the list elements the current number of
                        // times
                        self.mode.layout_iter.push_n(*element_rule, len);
                        // read the elements
                        visitor.visit_seq(SeqAccess::new_list(self, len))
                    }
                    BinProtRule::Int
                    | BinProtRule::Int32
                    | BinProtRule::Int64
                    | BinProtRule::NativeInt => visitor.visit_i64(self.rdr.bin_read_integer()?),
                    BinProtRule::Vec(_, _)
                    | BinProtRule::Nat0
                    | BinProtRule::Hashtable(_)
                    | BinProtRule::TypeVar(_)
                    | BinProtRule::Bigstring
                    | BinProtRule::SelfReference(_)
                    | BinProtRule::TypeClosure(_, _)
                    | BinProtRule::TypeAbstraction(_, _)
                    | BinProtRule::Reference(_) => Err(Error::UnimplementedRule), /* Don't know how to implement these yet */
                    BinProtRule::Custom(_) => {
                        // the traverse function should never produce this
                        Err(Error::LayoutIteratorError)
                    }
                    BinProtRule::CustomForPath(path, rules) => {
                        // here is where custom deser methods can be looked up by path
                        match path.as_str() {
                            // These vector types will be handled like any other sequence
                            "Pickles_type.Vector.Vector2" // the missing 's' on 'types' here is intention due to a bug in layout producing code
                            | "Pickles_types.Vector.Vector2" // in case it gets fixed :P
                            | "Pickles_types.Vector.Vector4"
                            | "Pickles_types.Vector.Vector8"
                            | "Pickles_types.Vector.Vector17"
                            | "Pickles_types.Vector.Vector18" => {
                                let element_rule = rules.first().unwrap();
                                let len = match path.as_str() {
                                    "Pickles_type.Vector.Vector2"
                                    | "Pickles_types.Vector.Vector2" => 2,
                                    "Pickles_types.Vector.Vector4" => 4,
                                    "Pickles_types.Vector.Vector8" => 8,
                                    "Pickles_types.Vector.Vector17" => 17,
                                    "Pickles_types.Vector.Vector18" => 18,
                                    _ => unreachable!()
                                };
                                self.mode.layout_iter.push(vec![BinProtRule::Unit]); // zero byte terminator, will be read last
                                self.mode.layout_iter.push_n(element_rule.clone(), len);
                                visitor.visit_seq(SeqAccess::new(self, len + 1))
                            }
                            "Ledger_hash0" // these are all BigInt (32 bytes)
                            | "State_hash"
                            | "Pending_coinbase.Stack_hash"
                            | "State_body_hash"
                            | "Pending_coinbase.Hash_builder"
                            | "Snark_params.Make_inner_curve_scalar"
                            | "Snark_params.Tick"
                            | "Epoch_seed"
                            | "Zexe_backend.Zexe_backend_common.Stable.Field"
                            | "Pending_coinbase.Coinbase_stack" => {
                                // force it to read a 32 element long tuple of u8/chars
                                self.mode.layout_iter.push_n(BinProtRule::Char, 32);
                                visitor.visit_seq(SeqAccess::new(self, 32))
                            }
                            _ => Err(Error::UnknownCustomType { typ: path })
                        }
                    }
                }
            }
            None => Err(Error::UnexpectedEndOfLayout),
        }
    }
}

pub enum VariantType {
    Sum(Summand),
    Polyvar(u8, TaggedPolyvar),
}

// for accessing enums when using the loosely typed method
// to deserialize into a Value
pub struct ValueEnum<'a, R: Read, Mode> {
    de: &'a mut DS<R, Mode>,
    variant: VariantType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EnumData {
    Sum {
        index: u8,
        name: String,
        len: usize,
    },
    Polyvar {
        index: u8,
        tag: u32,
        name: String,
        len: usize,
    },
}

impl<'a, R: Read, Mode> ValueEnum<'a, R, Mode> {
    fn new(de: &'a mut DS<R, Mode>, variant: VariantType) -> Self {
        Self { de, variant }
    }
}

impl<'de, 'a, R: Read> serde::de::EnumAccess<'de> for ValueEnum<'a, R, LooselyTyped> {
    type Error = Error;
    type Variant = Enum<'a, R, LooselyTyped>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        // bit of a hack here. visit_enum in the visitor is expecting to be able to
        // deserialize the enum details (e.g. variant index and name) from the stream.
        // Since in this case it comes from the layout file we need to serialize this
        // data and then return the deserializer to be handled by visit_enum

        let (index, enum_data) = match self.variant {
            VariantType::Sum(summand) => (
                summand.index as u8,
                EnumData::Sum {
                    index: summand.index.try_into().unwrap(),
                    name: summand.ctor_name,
                    len: summand.ctor_args.len(),
                },
            ),
            VariantType::Polyvar(index, polyvar) => (
                index,
                EnumData::Polyvar {
                    index,
                    tag: polyvar.hash,
                    name: polyvar.polyvar_name,
                    len: polyvar.polyvar_args.len(),
                },
            ),
        };

        let mut buf = Vec::<u8>::new();
        to_writer(&mut buf, &enum_data).unwrap();
        let mut de = DS::from_reader(buf.as_slice());
        let v = seed.deserialize(&mut de)?;

        Ok((v, Enum::new(self.de, index)))
    }
}
