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

use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Ident, Token};

/// Adapted from strum_macros
pub fn get_metadata<'a, T: Parse + Spanned>(
    name: &str,
    it: impl IntoIterator<Item = &'a Attribute>,
) -> syn::Result<Vec<T>> {
    it.into_iter()
        .filter(|attr| attr.path.is_ident(name))
        .try_fold(Vec::new(), |mut vec, attr| {
            vec.extend(attr.parse_args_with(Punctuated::<T, Token![,]>::parse_terminated)?);
            Ok(vec)
        })
}

/// Adapted from strum_macros
pub fn occurrence_error<T: ToTokens>(fst: T, snd: T, attr: &str) -> syn::Error {
    let mut e = syn::Error::new_spanned(
        snd,
        format!("Found multiple occurrences of queriable({})", attr),
    );
    e.combine(syn::Error::new_spanned(fst, "first one here"));
    e
}

/// Extract the bracketed type of Option.
pub fn parse_option(ty: &syn::Type) -> Option<syn::Type> {
    let ty_path = match ty {
        syn::Type::Path(ty_path) => ty_path,
        _ => return None,
    };
    // Reverse match parts from ::std::option::Option
    const OPTION_PATH: &[&str] = &["std", "option", "Option"];
    let segs = &ty_path.path.segments;
    if segs
        .iter()
        .rev()
        .zip(OPTION_PATH.iter().rev())
        .any(|(seg, part)| seg.ident != part)
    {
        return None;
    }
    let angle_args = match &segs.last()?.arguments {
        syn::PathArguments::AngleBracketed(angle_bracketed) => &angle_bracketed.args,
        _ => return None,
    };
    if angle_args.len() != 1 {
        return None;
    }
    match &angle_args[0] {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    }
}

/// Simplistic implementation of snake case to camel case conversion for ident.
/// For example, "thp_fault_alloc" => "ThpFaultAlloc".
pub fn to_camelcase(snake: &Ident) -> Ident {
    let mut res = String::new();
    // The next char should be upper case (convert if necessary). The first char
    // should always be upper case.
    let mut next_upper = true;
    for c in snake.to_string().chars() {
        if c == '_' {
            next_upper = true;
        } else {
            if c.is_ascii_lowercase() && next_upper {
                res.push(c.to_ascii_uppercase());
            } else {
                res.push(c);
            }
            next_upper = false;
        }
    }
    Ident::new(&res, snake.span())
}

/// Simplistic implementation of camel case to snake case conversion for ident.
/// For example, "ThpFaultAlloc" => "thp_fault_alloc".
pub fn to_snakecase(camel: &Ident) -> Ident {
    let mut res = String::new();
    let mut was_lower = false;
    for c in camel.to_string().chars() {
        if c.is_ascii_uppercase() {
            if was_lower {
                res.push('_');
            }
            res.push(c.to_ascii_lowercase());
        } else {
            was_lower = c.is_ascii_lowercase();
            res.push(c);
        }
    }
    Ident::new(&res, camel.span())
}
