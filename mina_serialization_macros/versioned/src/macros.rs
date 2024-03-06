// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//!
//! Macros that help implement common traits for versioned wrapper types.
//! Helper macros for type conversions

/// Macro that implements [From] trait for a struct and its versioned wrapper
#[macro_export]
macro_rules! impl_from_versioned {
    ($t:ty) => {
        impl<const V: u16> From<::mina_serialization_versioned::Versioned<$t, V>> for $t {
            fn from(t: ::mina_serialization_versioned::Versioned<$t, V>) -> Self {
                let (t,) = t.into();
                t
            }
        }

        impl<const V1: u16, const V2: u16>
            From<
                ::mina_serialization_versioned::Versioned<
                    ::mina_serialization_versioned::Versioned<$t, V1>,
                    V2,
                >,
            > for $t
        {
            fn from(
                t: ::mina_serialization_versioned::Versioned<
                    ::mina_serialization_versioned::Versioned<$t, V1>,
                    V2,
                >,
            ) -> Self {
                let (t,) = t.into();
                t
            }
        }
    };
}

/// Macro that implements [From] trait for a newtype struct and its versioned
/// wrapper
#[macro_export]
macro_rules! impl_from_for_newtype {
    ($t:ty, $tv:ty) => {
        impl From<$t> for $tv {
            fn from(t: $t) -> Self {
                t.0.into()
            }
        }

        impl From<$tv> for $t {
            fn from(t: $tv) -> Self {
                let (t,) = t.into();
                Self(t)
            }
        }
    };
}

/// Macro that implements [From] trait for 2-way conversion
/// between the original type and the target json type, via an
/// intermediate versioned type that is convertible from and to
/// both original and target types.
/// It also implements BinProtSerializationType and JsonSerializationType
/// for convenience
#[macro_export]
macro_rules! impl_from_with_proxy {
    ($t:ty, $t_versioned:ty, $t_json:ty) => {
        impl From<$t> for $t_json {
            fn from(t: $t) -> Self {
                let intermidate: $t_versioned = t.into();
                intermidate.into()
            }
        }

        impl From<$t_json> for $t {
            fn from(t: $t_json) -> Self {
                let intermidate: $t_versioned = t.into();
                intermidate.into()
            }
        }
    };
}

/// Macro that implements [From] trait for 2-way conversion
/// between the original type and the versioned target type, using the
/// intermediate type that is convertible from and to both
/// original and target types
#[macro_export]
macro_rules! impl_from_for_versioned_with_proxy {
    ($t:ty, $ti:ty, $t2:ty) => {
        impl From<$t> for $t2 {
            fn from(t: $t) -> Self {
                let intermidate: $ti = t.into();
                intermidate.into()
            }
        }

        impl From<$t2> for $t {
            fn from(t: $t2) -> Self {
                let (intermidate,): ($ti,) = t.into();
                intermidate.into()
            }
        }
    };
}

/// Macro that implements [From] trait for 2-way conversion
/// between the original type and the generic target type, using the
/// intermediate type that is convertible from and to both
/// original and target types
#[macro_export]
macro_rules! impl_from_for_generic_with_proxy {
    ($t:ty, $ti:ty, $t2:ty) => {
        impl From<$t> for $t2 {
            fn from(t: $t) -> Self {
                let intermidate: $ti = t.into();
                intermidate.into()
            }
        }

        impl From<$t2> for $t {
            fn from(t: $t2) -> Self {
                let (intermidate,): ($ti,) = t.into();
                intermidate.into()
            }
        }
    };
}

/// Implements [std::str::FromStr] and [std::fmt::Display] by implementing
/// [TryFrom] between given type and string types via its corresponding
/// json serialization type which is convertible from / to json with single
/// unnamed string field.
#[macro_export]
macro_rules! impl_strconv_via_json {
    ($ty:ty, $ty_json:ty) => {
        impl TryFrom<&str> for $ty {
            type Error = serde_json::error::Error;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                let json_string = serde_json::to_string(s)?;
                let json: $ty_json = serde_json::from_str(&json_string)?;
                Ok(json.into())
            }
        }

        impl ::std::str::FromStr for $ty {
            type Err = serde_json::error::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.try_into()
            }
        }

        impl TryFrom<$ty> for String {
            type Error = serde_json::error::Error;
            fn try_from(h: $ty) -> Result<Self, Self::Error> {
                let h: $ty_json = h.into();
                let json_string = serde_json::to_string(&h)?;
                let json: String = serde_json::from_str(&json_string)?;
                Ok(json)
            }
        }

        impl TryFrom<&$ty> for String {
            type Error = serde_json::error::Error;
            fn try_from(h: &$ty) -> Result<Self, Self::Error> {
                h.clone().try_into()
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                let s: String = self.try_into().map_err(|_| ::std::fmt::Error::default())?;
                write!(f, "{s}")
            }
        }
    };
}

/// Implement list tagged enum json serde format for the given enum,
/// with another convertible enum which is externally tagged and
/// extra serde options
#[macro_export]
macro_rules! impl_mina_enum_json_serde_with_option {
    ($t:ty, $tp:ty, $ser_expand_array:expr) => {
        impl ::serde::Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let e: $tp = self.clone().into();
                let v =
                    ::serde_json::to_value(e).map_err(<S::Error as ::serde::ser::Error>::custom)?;
                if v.is_string() {
                    let list_tagged_array = ::serde_json::json!([v]);
                    return serializer.serialize_some(&list_tagged_array);
                } else if let Some(m) = v.as_object() {
                    if m.len() != 1 {
                        panic!("Bad enum: {:?}", self);
                    }
                    for (k, v) in m {
                        if $ser_expand_array {
                            if let Some(array) = v.as_array() {
                                let mut list_tagged_array = ::serde_json::json!([k]);
                                if let Some(list_tagged_array) = list_tagged_array.as_array_mut() {
                                    for i in array {
                                        list_tagged_array.push(i.clone());
                                    }
                                }
                                return serializer.serialize_some(&list_tagged_array);
                            }
                        }
                        let list_tagged_array = ::serde_json::json!([k, v]);
                        return serializer.serialize_some(&list_tagged_array);
                    }
                }
                serializer.serialize_some(&v)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let v = ::serde_json::Value::deserialize(deserializer)?;
                if let Some(array) = v.as_array() {
                    let e: $tp = match array.len() {
                        0 => panic!("Non-empty array expected"),
                        1 => ::serde_json::from_value(array[0].clone())
                            .map_err(<D::Error as ::serde::de::Error>::custom)?,
                        2 => {
                            let key: String = ::serde_json::from_value(array[0].clone())
                                .map_err(<D::Error as serde::de::Error>::custom)?;
                            ::serde_json::from_value(::serde_json::json!({key: array[1]}))
                                .map_err(<D::Error as serde::de::Error>::custom)?
                        }
                        _ => {
                            let key: String = ::serde_json::from_value(array[0].clone())
                                .map_err(<D::Error as ::serde::de::Error>::custom)?;
                            ::serde_json::from_value(::serde_json::json!({key: array[1..]}))
                                .map_err(<D::Error as ::serde::de::Error>::custom)?
                        }
                    };
                    Ok(e.into())
                } else {
                    panic!("Array expected")
                }
            }
        }
    };
}

/// Implement list tagged enum json serde format for the given enum,
/// with another convertible enum which is externally tagged
#[macro_export]
macro_rules! impl_mina_enum_json_serde {
    ($t:ty, $tp:ty) => {
        impl_mina_enum_json_serde_with_option!($t, $tp, true);
    };
}
