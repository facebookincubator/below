// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Macros for implementing Queriable FieldId enum methods (thus named qenum).
//!
//! This mod is mostly inspired by the strum_macros crate, particularly
//! EnumString (for EnumFromStr), strum_macros:ToString (for EnumToString), and
//! EnumIter (for EnumIter). However, this mod extends those macros by defining
//! special behaviors with enum variants that contains a single unnamed field.
//! These variants are mainly used by Queriable::FieldId to access fields of
//! sub-models, making the enum a mapping of the tree structure of its
//! corresponding Queriable.

use crate::helper::to_snakecase;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, DeriveInput, LitStr};

fn get_variants(
    ast: &DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>> {
    match &ast.data {
        syn::Data::Enum(syn::DataEnum { variants, .. }) => Ok(variants),
        _ => Err(syn::Error::new(
            Span::call_site(),
            "This macro only supports enum.",
        )),
    }
}

fn variant_constraint_error(span: Span) -> syn::Error {
    syn::Error::new(
        span,
        "This macro only supports unit variant or variant with one unnamed field.",
    )
}

pub fn enum_to_string_derive_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &ast.ident;

    let variant_to_string_arms = get_variants(ast)?
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let snake = to_snakecase(variant_name);
            let snake_str = LitStr::new(&snake.to_string(), snake.span());
            match &variant.fields {
                syn::Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => Ok(quote! {
                    Self::#variant_name(nested) => format!(
                        "{}.{}",
                        #snake_str,
                        nested.to_string()
                    ),
                }),
                syn::Fields::Unit => Ok(quote! {
                    Self::#variant_name => #snake_str.to_owned(),
                }),
                _ => Err(variant_constraint_error(variant.span())),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl ::std::string::ToString for #enum_name {
            fn to_string(&self) -> ::std::string::String {
                match self {
                    #(#variant_to_string_arms)*
                }
            }
        }
    })
}

pub fn enum_from_str_derive_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &ast.ident;

    let variant_from_str_arms = get_variants(ast)?
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let snake = to_snakecase(variant_name);
            let snake_str = LitStr::new(&snake.to_string(), snake.span());
            match &variant.fields {
                syn::Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                    let nested_type = &unnamed.unnamed[0].ty;
                    Ok(quote! {
                        _ if s.starts_with(concat!(#snake_str, ".")) => {
                            <#nested_type>::from_str(unsafe {
                                s.get_unchecked(concat!(#snake_str, ".").len()..)
                            }).map(Self::#variant_name)
                        }
                    })
                }
                syn::Fields::Unit => Ok(quote! {
                    #snake_str => Ok(Self::#variant_name),
                }),
                _ => Err(variant_constraint_error(variant.span())),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl ::std::str::FromStr for #enum_name {
            type Err = ::anyhow::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                match s {
                    #(#variant_from_str_arms)*
                    _ => Err(::anyhow::anyhow!(
                        "Unable to find a variant of the given enum matching string `{}`.",
                        s,
                    )),
                }
            }
        }

    })
}

pub fn enum_iter_derive_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &ast.ident;
    let variants = get_variants(ast)?;

    let unit_variants = variants.iter().filter(|variant| match &variant.fields {
        syn::Fields::Unit => true,
        _ => false,
    });

    // Iterate over all enums by chaining them together. Unit variants are
    // wrapped in std::iter::once. Basically a DFS over the enum tree.
    let all_variant_iter_chain = variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            match &variant.fields {
                syn::Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                    let nested_type = &unnamed.unnamed[0].ty;
                    Ok(quote! {
                        .chain(<#nested_type>::all_variant_iter().map(Self::#variant_name))
                    })
                }
                syn::Fields::Unit => Ok(quote! {
                    .chain(::std::iter::once(Self::#variant_name))
                }),
                _ => Err(variant_constraint_error(variant.span())),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl EnumIter for #enum_name {
            fn unit_variant_iter() -> Box<dyn Iterator<Item = Self>> {
                const UNIT_VARIANTS: &[#enum_name] = &[#(#enum_name::#unit_variants),*];
                Box::new(UNIT_VARIANTS.iter().cloned())
            }

            fn all_variant_iter() -> Box<dyn Iterator<Item = Self>> {
                Box::new(
                    ::std::iter::empty()
                    #(#all_variant_iter_chain)*
                )
            }
        }
    })
}
