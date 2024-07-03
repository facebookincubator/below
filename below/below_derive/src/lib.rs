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

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod helper;
mod qenum;
mod queriable;

/// Implements std::fmt::Display for enum, which must only contain unit
/// variants or variants with single unnamed field, e.g. Some(T). Unit variants
/// are converted to their snake case representations. Nested variants works
/// similarly by joining the variant name and field representation with dot ".".
/// For example, None => "none", and Some(None) => "some.none".
#[proc_macro_derive(EnumToString)]
pub fn enum_to_string_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    qenum::enum_to_string_derive_impl(&ast)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements std::str::FromStr for enum, which has same constraints as
/// EnumToString and works in the opposite direction.
#[proc_macro_derive(EnumFromStr)]
pub fn enum_from_str_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    qenum::enum_from_str_derive_impl(&ast)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements the Queriable trait for a model. An enum with variants that map
/// to its fields are created with auto derive above: EnumToString, EnumFromStr.
/// That enum is used as Queriable::FieldId. Subquery fields are accessed by
/// delegating the subquery field_id to the corresponding sub-models.
///
/// Struct attributes:
///
/// #[queriable(field_id_name = MyCgroupModelFieldId)]
///     Alternative name for the created enum. If not provided, It will be
///     `{model_name}FieldId`, i.e. CgroupModelFieldId for struct CgroupModel.
///
/// Field attributes:
///
/// #[queriable(ignore)]
///     Ignore field when implementing Queriable trait.
///
/// #[queriable(subquery)]
///     Mark field for subquery processing, i.e. its value is a Queriable to
///     which we delegate the subquery.
///     For example, a `cpu` field with type CgroupCpuModel annotated with
///     subquery will generate a corresponding variant in the created enum as
///     Cpu(<CgroupCpuModel as Queriable>::FieldId).
///
/// #[queriable(preferred_name = mem)]
///     Name used for generating enum variant instead of the original one. Must
///     be a valid field name for struct (not quoted).
///
/// Example:
///
/// #[derive(::below_derive::Queriable)]
/// #[queriable(field_id_name = MyFooFieldId)]
/// struct Foo {
///     a: u64,
///     b: Option<String>,
///     #[queriable(subquery)]
///     c: Option<Bar>,
///     #[queriable(ignore)]
///     d: f64,
/// }
///
/// Generated code:
///
/// #[derive(
///     Clone,
///     Debug,
///     PartialEq,
///     ::below_derive::EnumFromStr,
///     ::below_derive::EnumToString
///     ::enum_iterator::Sequence
/// )]
/// enum MyFooFieldId {
///     A,
///     B,
///     C(<Bar as Queriable>::FieldId),
/// }
///
/// impl Queriable for Foo {
///     type FieldId = MyFooFieldId;
///     fn query(&self, field_id: &Self::FieldId) -> ::std::option::Option<Field> {
///         match field_id {
///             A => std::option::Option::Some(Field::from(&self.a)),
///             B => self.b.as_ref().map(Field::from),
///             C(field_id) => self.c.query(field_id),
///         }
///     }
/// }
#[proc_macro_derive(Queriable, attributes(queriable))]
pub fn queriable_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    queriable::queriable_derive_impl(&ast)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Shorthand to add #[derive] for all traits necessary for a Queriable model.
/// Example:
///
/// #[below_derive::queriable_derives]
/// struct Foo {
///    ...
/// }
///
/// Generates:
///
/// #[derive(
///     Clone,
///     Debug,
///     Default,
///     PartialEq,
///     Serialize,
///     Deserialize,
///     ::below_derive::Queriable,
/// )]
/// struct Foo {
///   ...
/// }
#[proc_macro_attribute]
pub fn queriable_derives(_: TokenStream, input: TokenStream) -> TokenStream {
    queriable::queriable_derives_impl(input.into()).into()
}
