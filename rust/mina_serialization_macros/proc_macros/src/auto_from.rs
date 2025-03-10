// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//!
//! Utilities for implementing AutoFrom

use crate::*;
use syn::{punctuated::Punctuated, AttrStyle, Attribute, Field, Fields, Token, Variant};

/// Parse the attributed target type(s)
pub fn parse_types_from_attr(attributes: &[Attribute]) -> Vec<proc_macro2::TokenStream> {
    let mut target_types: Vec<proc_macro2::TokenStream> = Vec::new();
    for attr in attributes {
        if let AttrStyle::Outer = attr.style {
            if let Some(attr_id) = attr.path().get_ident() {
                if &attr_id.to_string() == "auto_from" {
                    if let Ok(args) = attr.parse_args::<proc_macro2::TokenStream>() {
                        target_types.push(args.clone());
                    }
                }
            }
        };
    }
    target_types
}

/// Codegen for structs with named fields
pub fn auto_from_for_struct_with_named_fields(
    ident: &proc_macro2::Ident,
    target_types: &[proc_macro2::TokenStream],
    named: Punctuated<Field, Token![,]>,
) -> Option<TokenStream> {
    let mut field_idents = Vec::new();
    let mut vec_field_idents = Vec::new();
    let mut option_field_idents = Vec::new();
    let mut boxed_field_idents = Vec::new();
    'outer: for f in named {
        if let Some(ident) = f.ident {
            if let syn::Type::Path(type_path) = f.ty {
                for seg in type_path.path.segments {
                    match seg.ident.to_string().as_str() {
                        "Vec" => {
                            vec_field_idents.push(ident);
                            continue 'outer;
                        }
                        "Option" => {
                            option_field_idents.push(ident);
                            continue 'outer;
                        }
                        "Box" => {
                            boxed_field_idents.push(ident);
                            continue 'outer;
                        }
                        _ => {}
                    };
                }
            }
            field_idents.push(ident);
        }
    }
    if field_idents.is_empty()
        && vec_field_idents.is_empty()
        && option_field_idents.is_empty()
        && boxed_field_idents.is_empty()
    {
        return None;
    }
    let mut output = TokenStream::default();

    for target_type in target_types {
        let ts: TokenStream = quote! {
            impl ::std::convert::From<#ident> for #target_type {
                fn from(item: #ident) -> Self {
                    Self {
                        #(#field_idents: item.#field_idents.into(),) *
                        #(#vec_field_idents: item.#vec_field_idents.into_iter().map(::std::convert::Into::into).collect(),) *
                        #(#option_field_idents: item.#option_field_idents.map(::std::convert::Into::into),) *
                        #(#boxed_field_idents: ::std::boxed::Box::new((*item.#boxed_field_idents).into()),) *
                    }
                }
            }

            impl ::std::convert::From<#target_type> for #ident {
                fn from(item: #target_type) -> Self {
                    Self {
                        #(#field_idents: item.#field_idents.into(),) *
                        #(#vec_field_idents: item.#vec_field_idents.into_iter().map(::std::convert::Into::into).collect(),) *
                        #(#option_field_idents: item.#option_field_idents.map(::std::convert::Into::into),) *
                        #(#boxed_field_idents: ::std::boxed::Box::new((*item.#boxed_field_idents).into()),) *
                    }
                }
            }
        }
        .into();
        output.extend(ts);
        output.extend(impl_from_for_versioned(ident, target_type));
    }

    Some(output)
}

/// Codegen for structs with unnamed fields
pub fn auto_from_for_struct_with_unnamed_fields(
    ident: &proc_macro2::Ident,
    target_types: &[proc_macro2::TokenStream],
    named: Punctuated<Field, Token![,]>,
) -> Option<TokenStream> {
    let mut pos_token_stream: Vec<proc_macro2::TokenStream> = Vec::new();
    'outer: for f in named {
        if f.ident.is_none() {
            let pos = proc_macro2::Literal::usize_unsuffixed(pos_token_stream.len());

            if let syn::Type::Path(type_path) = f.ty {
                for seg in type_path.path.segments {
                    match seg.ident.to_string().as_str() {
                        "Vec" => {
                            pos_token_stream.push(
                                quote! {item.#pos.into_iter().map(::std::convert::Into::into).collect()}
                            );
                            continue 'outer;
                        }
                        "Option" => {
                            pos_token_stream
                                .push(quote! {item.#pos.map(::std::convert::Into::into)});
                            continue 'outer;
                        }
                        "Box" => {
                            pos_token_stream
                                .push(quote! {::std::boxed::Box::new((*item.#pos).into())});
                            continue 'outer;
                        }
                        _ => {}
                    };
                }
            }

            pos_token_stream.push(quote! {item.#pos.into()});
        }
    }
    if pos_token_stream.is_empty() {
        return None;
    }
    let mut output = TokenStream::default();

    for target_type in target_types {
        let ts: TokenStream = quote! {
            impl ::std::convert::From<#ident> for #target_type {
                fn from(item: #ident) -> Self {
                    Self (
                        #(#pos_token_stream,) *
                    )
                }
            }

            impl ::std::convert::From<#target_type> for #ident {
                fn from(item: #target_type) -> Self {
                    Self (
                        #(#pos_token_stream,) *
                    )
                }
            }
        }
        .into();
        output.extend(ts);
        output.extend(impl_from_for_versioned(ident, target_type));
    }

    Some(output)
}

/// Codegen for enums
pub fn auto_from_for_enum(
    ident: &proc_macro2::Ident,
    target_types: &[proc_macro2::TokenStream],
    variants: Punctuated<Variant, Token![,]>,
) -> Option<TokenStream> {
    let mut variant_token_streams: Vec<proc_macro2::TokenStream> =
        Vec::with_capacity(variants.len());
    for v in variants {
        let ident = &v.ident;
        match v.fields.len() {
            0 => variant_token_streams.push(quote! {
                Other::#ident => Self::#ident
            }),
            fields_len => match v.fields {
                Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                    let mut lhs_idents = Vec::with_capacity(fields_len);
                    let mut rhs_convert = Vec::with_capacity(fields_len);
                    'outer_unnamed: for (i, f) in unnamed.into_iter().enumerate() {
                        let ident = proc_macro2::Ident::new(
                            &format!("v{i}"),
                            proc_macro2::Span::call_site(),
                        );
                        lhs_idents.push(ident.clone());
                        if let syn::Type::Path(type_path) = f.ty {
                            for seg in type_path.path.segments {
                                match seg.ident.to_string().as_str() {
                                    "Vec" => {
                                        rhs_convert.push(quote! {#ident.into_iter().map(::std::convert::Into::into).collect()});
                                        continue 'outer_unnamed;
                                    }
                                    "Option" => {
                                        rhs_convert
                                            .push(quote! {#ident.map(::std::convert::Into::into)});
                                        continue 'outer_unnamed;
                                    }
                                    "Box" => {
                                        rhs_convert.push(
                                            quote! {::std::boxed::Box::new((*#ident).into())},
                                        );
                                        continue 'outer_unnamed;
                                    }
                                    _ => {}
                                };
                            }
                        }

                        rhs_convert.push(quote! {#ident.into()});
                    }
                    variant_token_streams.push(quote! {
                        Other::#ident(#(#lhs_idents,) *) => Self::#ident(#(#rhs_convert,) *)
                    });
                }
                Fields::Named(FieldsNamed { named, .. }) => {
                    let mut lhs_idents = Vec::with_capacity(fields_len);
                    let mut rhs_convert = Vec::with_capacity(fields_len);
                    'outer_named: for f in named {
                        if let Some(ident) = f.ident {
                            lhs_idents.push(ident.clone());
                            if let syn::Type::Path(type_path) = f.ty {
                                for seg in type_path.path.segments {
                                    match seg.ident.to_string().as_str() {
                                        "Vec" => {
                                            rhs_convert.push(quote! {#ident: #ident.into_iter().map(::std::convert::Into::into).collect()});
                                            continue 'outer_named;
                                        }
                                        "Option" => {
                                            rhs_convert.push(
                                                    quote! {#ident: #ident.map(::std::convert::Into::into)},
                                                );
                                            continue 'outer_named;
                                        }
                                        "Box" => {
                                            rhs_convert.push(
                                                quote! {#ident: ::std::boxed::Box::new((*#ident).into())},
                                            );
                                            continue 'outer_named;
                                        }
                                        _ => {}
                                    };
                                }
                            }
                            rhs_convert.push(quote! {#ident: #ident.into()});
                        }
                    }
                    variant_token_streams.push(quote! {
                        Other::#ident{#(#lhs_idents,) *} => Self::#ident{#(#rhs_convert,) *}
                    });
                }
                _ => {}
            },
        }
    }

    if variant_token_streams.is_empty() {
        return None;
    }
    let mut output = TokenStream::default();
    for target_type in target_types {
        let ts: TokenStream = quote! {
            impl ::std::convert::From<#ident> for #target_type {
                fn from(item: #ident) -> Self {
                    use #ident as Other;
                    match item {
                        #(#variant_token_streams,) *
                    }
                }
            }

            impl ::std::convert::From<#target_type> for #ident {
                fn from(item: #target_type) -> Self {
                    use #target_type as Other;
                    match item {
                        #(#variant_token_streams,) *
                    }
                }
            }
        }
        .into();
        output.extend(ts);
        output.extend(impl_from_for_versioned(ident, target_type));
    }

    Some(output)
}

/// Codegen for versioned target type(s)
fn impl_from_for_versioned(
    ident: &proc_macro2::Ident,
    target_type: &proc_macro2::TokenStream,
) -> TokenStream {
    quote! {
        impl<const V: u16> ::std::convert::From<#ident> for ::mina_serialization_versioned::Versioned<#target_type, V> {
            #[inline]
            fn from(t: #ident) -> Self {
                let t: #target_type = t.into();
                t.into()
            }
        }

        impl<const V: u16> ::std::convert::From<::mina_serialization_versioned::Versioned<#target_type, V>> for #ident {
            #[inline]
            fn from(t: ::mina_serialization_versioned::Versioned<#target_type, V>) -> Self {
                let (t,): (#target_type,) = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16> ::std::convert::From<#ident> for ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2> {
            #[inline]
            fn from(t: #ident) -> Self {
                let t: #target_type = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16> ::std::convert::From<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>> for #ident {
            #[inline]
            fn from(t: ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>) -> Self {
                let (t,): (#target_type,) = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16, const V3: u16> ::std::convert::From<#ident> for ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3> {
            #[inline]
            fn from(t: #ident) -> Self {
                let t: #target_type = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16, const V3: u16> ::std::convert::From<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3>> for #ident {
            #[inline]
            fn from(t: ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3>) -> Self {
                let (t,): (#target_type,) = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16, const V3: u16, const V4: u16> ::std::convert::From<#ident> for ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3>, V4> {
            #[inline]
            fn from(t: #ident) -> Self {
                let t: #target_type = t.into();
                t.into()
            }
        }

        impl<const V1: u16, const V2: u16, const V3: u16, const V4: u16> ::std::convert::From<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3>, V4>> for #ident {
            #[inline]
            fn from(t: ::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<::mina_serialization_versioned::Versioned<#target_type, V1>, V2>, V3>, V4>) -> Self {
                let (t,): (#target_type,) = t.into();
                t.into()
            }
        }
    }.into()
}
