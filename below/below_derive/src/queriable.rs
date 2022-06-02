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

use crate::helper::{get_metadata, occurrence_error, parse_option, to_camelcase};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    DeriveInput, Field, Ident, Token,
};

mod kw {
    use syn::custom_keyword;

    // struct metadata
    custom_keyword!(field_id_name);

    // field metadata
    custom_keyword!(ignore);
    custom_keyword!(subquery);
    custom_keyword!(preferred_name);
}

pub enum StructMeta {
    FieldIdName { kw: kw::field_id_name, value: Ident },
}

impl Parse for StructMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::field_id_name) {
            let kw = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(StructMeta::FieldIdName { kw, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl Spanned for StructMeta {
    fn span(&self) -> Span {
        match self {
            StructMeta::FieldIdName { kw, .. } => kw.span,
        }
    }
}

pub enum FieldMeta {
    Ignore(kw::ignore),
    Subquery(kw::subquery),
    PreferredName {
        kw: kw::preferred_name,
        value: Ident,
    },
}

impl Parse for FieldMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ignore) {
            Ok(FieldMeta::Ignore(input.parse()?))
        } else if lookahead.peek(kw::subquery) {
            Ok(FieldMeta::Subquery(input.parse()?))
        } else if lookahead.peek(kw::preferred_name) {
            let kw = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(FieldMeta::PreferredName { kw, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl Spanned for FieldMeta {
    fn span(&self) -> Span {
        match self {
            FieldMeta::Ignore(kw) => kw.span,
            FieldMeta::Subquery(kw) => kw.span,
            FieldMeta::PreferredName { kw, .. } => kw.span,
        }
    }
}

#[derive(Clone, Debug)]
struct QueriableStructProps {
    pub field_id_name: Ident,
    pub ident: Ident,
}

fn get_queriable_struct_props(ast: &DeriveInput) -> syn::Result<QueriableStructProps> {
    let mut field_id_name = None;
    let mut field_id_name_kw = None;
    for meta in get_metadata("queriable", &ast.attrs)? {
        match meta {
            StructMeta::FieldIdName { value, kw } => {
                if let Some(fst_kw) = field_id_name_kw {
                    return Err(occurrence_error(fst_kw, kw, "field_id_name"));
                }
                field_id_name_kw = Some(kw);
                field_id_name = Some(value);
            }
        }
    }
    Ok(QueriableStructProps {
        field_id_name: field_id_name
            // Add `FieldId` suffix for default FieldId enum name.
            .unwrap_or_else(|| Ident::new(&format!("{}FieldId", ast.ident), ast.ident.span())),
        ident: ast.ident.clone(),
    })
}

struct QueriableFieldProps {
    pub ignore: bool,
    pub subquery: Option<syn::Type>,
    pub ident: Ident,
    pub variant_name: Ident,
    pub option_type: Option<syn::Type>,
}

fn get_queriable_field_props(field: &Field) -> syn::Result<QueriableFieldProps> {
    let mut ignore = false;
    let mut subquery = None;
    let mut preferred_name = None;
    let mut ignore_kw = None;
    let mut subquery_kw = None;
    let mut preferred_name_kw = None;
    let option_type = parse_option(&field.ty);
    for meta in get_metadata("queriable", &field.attrs)? {
        match meta {
            FieldMeta::Ignore(kw) => {
                if let Some(fst_kw) = ignore_kw {
                    return Err(occurrence_error(fst_kw, kw, "ignore"));
                }
                ignore_kw = Some(kw);
                ignore = true;
            }
            FieldMeta::Subquery(kw) => {
                if let Some(fst_kw) = subquery_kw {
                    return Err(occurrence_error(fst_kw, kw, "subquery"));
                }
                subquery_kw = Some(kw);
                // Extract field if it's wrapped inside Option
                let base_type = option_type.as_ref().unwrap_or(&field.ty);
                // subquery field must implement Queriable
                subquery = Some(syn::parse_quote! {
                    <#base_type as Queriable>::FieldId
                });
            }
            FieldMeta::PreferredName { value, kw } => {
                if let Some(fst_kw) = preferred_name_kw {
                    return Err(occurrence_error(fst_kw, kw, "preferred_name"));
                }
                preferred_name_kw = Some(kw);
                preferred_name = Some(value.clone());
            }
        }
    }
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new(field.span(), "This macro only support named fields"))?;
    let preferred_name = preferred_name.unwrap_or_else(|| ident.clone());
    let variant_name = to_camelcase(&preferred_name);
    Ok(QueriableFieldProps {
        ignore,
        subquery,
        ident,
        variant_name,
        option_type,
    })
}

pub fn queriable_derive_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_props = get_queriable_struct_props(ast)?;
    let input_ident = struct_props.ident;
    let field_id_ident = struct_props.field_id_name;

    let fields = match &ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "This macro only supports struct with named fields.",
            ));
        }
    };

    let mut all_field_props = Vec::new();
    for field in fields {
        let field_props = get_queriable_field_props(&field)?;
        if !field_props.ignore {
            all_field_props.push(field_props);
        }
    }

    let field_id_variants = all_field_props.iter().map(|field_props| {
        let variant_name = &field_props.variant_name;
        match &field_props.subquery {
            Some(subquery_field_id_type) => quote! {
                #variant_name(#subquery_field_id_type),
            },
            None => quote! {
                #variant_name,
            },
        }
    });

    let queriable_match_arms = all_field_props.iter().map(|field_props| {
        let variant_name = &field_props.variant_name;
        let field_ident = &field_props.ident;
        if field_props.subquery.is_some() {
            let query = if field_props.option_type.is_some() {
                quote! { self.#field_ident.as_ref().and_then(|q| q.query(field_id)) }
            } else {
                quote! { self.#field_ident.query(field_id) }
            };
            quote! { Self::FieldId::#variant_name(field_id) => #query, }
        } else {
            let query = if field_props.option_type.is_some() {
                quote! { self.#field_ident.as_ref().map(Field::from) }
            } else {
                quote! { std::option::Option::Some(Field::from(&self.#field_ident)) }
            };
            quote! { Self::FieldId::#variant_name => #query, }
        }
    });

    Ok(quote! {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            ::below_derive::EnumIter,
            ::below_derive::EnumFromStr,
            ::below_derive::EnumToString
        )]
        pub enum #field_id_ident {
            #(#field_id_variants)*
        }

        impl FieldId for #field_id_ident {
            type Queriable = #input_ident;
        }

        impl Queriable for #input_ident {
            type FieldId = #field_id_ident;
            fn query(&self, field_id: &Self::FieldId) -> ::std::option::Option<Field> {
                match field_id {
                    #(#queriable_match_arms)*
                    _ => unreachable!(),
                }
            }
        }
    })
}
