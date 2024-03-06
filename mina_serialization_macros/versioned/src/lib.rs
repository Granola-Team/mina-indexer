// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//!
//! Versioned wrapper types for serialization
//!
//! In the bin-prot Mina wire protocol, each nested type has an associated
//! version. This is to allow for backward compatibility if parts of the wire
//! protocol change. This simple wrapper type ensures that this information
//! is included in the serialized output in an identical way to the Mina
//! reference implementation.

#![deny(warnings)]
#![deny(missing_docs)]

pub mod macros;

use serde::{Deserialize, Serialize};

/// A generic version wrapper around another type
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct Versioned<T, const V: u16> {
    /// Version byte to be encoded first when the whole wrapper is serialized
    pub version: u16,
    /// The wrapped type
    pub t: T,
}

/// A wrapper around version supporting Major and Minor revisions
pub type Versioned2<T, const MAJOR: u16, const MINOR: u16> = Versioned<Versioned<T, MINOR>, MAJOR>;

/// A wrapper around version supporting Major, Minor, and Patch revisions
pub type Versioned3<T, const MAJOR: u16, const MINOR: u16, const PATCH: u16> =
    Versioned2<Versioned<T, PATCH>, MAJOR, MINOR>;

/// A wrapper around version supporting Major, Minor, Patch, and Revision
/// revisions
pub type Versioned4<T, const MAJOR: u16, const MINOR: u16, const PATCH: u16, const REVISION: u16> =
    Versioned3<Versioned<T, REVISION>, MAJOR, MINOR, PATCH>;

impl<T, const V: u16> Default for Versioned<T, V>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            version: V, // version should always be equal to V
            t: Default::default(),
        }
    }
}

impl<T, const V: u16> Versioned<T, V> {
    /// create a new version type of the given const version
    pub fn new(t: T) -> Self {
        Self { version: V, t }
    }

    /// Return the inner type
    pub fn inner(self) -> T {
        self.t
    }

    /// Return the version number
    pub fn version(&self) -> u16 {
        self.version
    }
}

impl<T, const V: u16> From<T> for Versioned<T, V> {
    #[inline]
    fn from(t: T) -> Self {
        Versioned::new(t)
    }
}

impl<T, const V: u16> From<Versioned<T, V>> for (T,) {
    #[inline]
    fn from(t: Versioned<T, V>) -> Self {
        (t.t,)
    }
}

impl<T, const V1: u16, const V2: u16> From<T> for Versioned2<T, V1, V2> {
    #[inline]
    fn from(t: T) -> Self {
        let t: Versioned<T, V2> = t.into();
        t.into()
    }
}

impl<T, const V1: u16, const V2: u16> From<Versioned2<T, V1, V2>> for (T,) {
    #[inline]
    fn from(t: Versioned2<T, V1, V2>) -> Self {
        let (t,): (Versioned<T, V2>,) = t.into();
        t.into()
    }
}

impl<T, const V1: u16, const V2: u16, const V3: u16> From<T> for Versioned3<T, V1, V2, V3> {
    #[inline]
    fn from(t: T) -> Self {
        let t: Versioned2<T, V2, V3> = t.into();
        t.into()
    }
}

impl<T, const V1: u16, const V2: u16, const V3: u16> From<Versioned3<T, V1, V2, V3>> for (T,) {
    #[inline]
    fn from(t: Versioned3<T, V1, V2, V3>) -> Self {
        let (t,): (Versioned2<T, V2, V3>,) = t.into();
        t.into()
    }
}

impl<T, const V1: u16, const V2: u16, const V3: u16, const V4: u16> From<T>
    for Versioned4<T, V1, V2, V3, V4>
{
    #[inline]
    fn from(t: T) -> Self {
        let t: Versioned3<T, V2, V3, V4> = t.into();
        t.into()
    }
}

impl<T, const V1: u16, const V2: u16, const V3: u16, const V4: u16>
    From<Versioned4<T, V1, V2, V3, V4>> for (T,)
{
    #[inline]
    fn from(t: Versioned4<T, V1, V2, V3, V4>) -> Self {
        let (t,): (Versioned3<T, V2, V3, V4>,) = t.into();
        t.into()
    }
}
