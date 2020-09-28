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

//! The whole objective to this procedure macro is to provide a unified way of
//! managing the connection between model and view. This trait will provide
//! solution of the following topic:
//!
//! 1. Data Linking: A virtual link between displayed data and real data.
//! 2. Displaying: Defines how a data is showed including data decoration.
//! 3. Convenient Functions: Title generating, sorting, pattern(TODO), etc.
//!
//! # Attributes
//! ## bttr - below attributes
//! * Display related attributes defines how you want to show the data. If
//!   a field does not have any display or field attribute, `BelowDecor`
//!   will not generate any display related function.
//!
//!     * depth: TokenStream -- A string token stream that produce a
//!       integer which defines the prefix indentation spaces.
//!     * unit: String -- Unit of the field
//!     * prefix: TokenStream -- A string token stream that produce a
//!       displayable represent the prefix symbol.
//!     * width: usize -- Defines the width of the column, apply for
//!       both title and field.
//!     * title_width: usize -- Defines the width of title, will override
//!       width for title.
//!     * none_mark: String -- Default to '?', defines what need show
//!       if the field is None.
//!     * decorator: TokenStream -- Field decorator, apply a function
//!       to the value of this field, will replace $ with the field name.
//!     * precision: usize -- Defines what's the displaying precision when
//!       the value is f64
//!     * highlight_if: Highlight field in COLOR if function returns Some(COLOR).
//!       Will replace `$` with the field name. The function should return
//!       cursive::theme::BaseColor
//!     * raw: TokenStream -- If set, will not generate unit and decorator related
//!       code.
//!
//! * Field related attributes defines how you want to change the field.
//!   If a field does not have any field attribute, `BelowDecor` will not
//!   generate any field related function.
//!
//!     * title: String -- Title of the field
//!     * cmp: bool -- If need to generate compare function for this field.
//!     * gen: bool -- Only generate minimal getter without display code.
//!       Eg.: `"CgroupModel: io_total?.rbytes_per_sec? + io_total?.wbytes_per_sec?"`
//!     * tag: string -- Help to generate dfill trait that map the tag to concrete processing
//!       functions. Eg: tag = "CgroupField::IoDiops"
//!     * class: string -- Mark which class current field belongs to. Dump ONLY.
//!     * class_detail: bool -- If set, current field will show if --detail specified. Dump ONLY
//!
//!
//! ## blink - below link
//! * `Type$call_path` -- Type is the argument type of the generated function.
//!   `Model$cgroup.get_cpu` will generate `model.cgroup.get_cpu()`, the
//!    `model` here is an argument of the generated function. More details can be found in field::parse_blink
//! * Limitation: All link from same struct should have a same starting point, aka, model.
//! * Multi-link will aggregate the link value
//!
//! ## Special characters
//! * `?` --> Means the marked field is an option, it will tell the macro to unwrap a
//!   reference of the field, if the option is None, it will auto use the default value.
//!   Use with `blink`
//!
//! ## Generated functions
//! **If a field has a link, you will need an extra model argument.**
//! * `get_FIELD_NAME_value<'a>(&'a self) -> &'a Type` -- Get a reference of FIELD_NAME. This function won't apply decorator function.
//! * `get_FIELD_NAME_title(&self) -> &'static str` -- Get raw title string. We capture a `&self` here is for chaining of link.
//! * `get_FIELD_NAME_title_styled(&self) -> &'static str` -- Get styled title string. This function will apply width attribute.
//! * `get_title_line() -> String` -- Get all titles in a line, all style applied.
//! * `get_title_pipe() -> String` -- Get all titles in a line, separated by '|', all style applied.
//! * `get_FIELD_NAME_str_styled(&self) -> StyledString` -- Get the field string, all style applied.
//! * `get_FIELD_NAME_str(&self) -> String` -- Get the field string, no style applied, decorator will be hornored.
//! * `get_field_line(&self) -> StyledString` -- Get all fields in a line, all style applied.
//! * `get_csv_field(&self) -> String` -- Get all fields in a line, only apply decorator function. Comma separated.
//! * `get_csv_title(&self) -> String` -- Get all title in a line, comma separated.
//! * `cmp_by_FIELD_NAME(left: &Self, right, &Self) -> Ordering` -- comparison function. If field is a link, type of left and right will be Model
//! * `get_interleave_line(&self, sep: &str) -> Vec<StyledString>` -- Interleave title and value and output
//!    as string. `sep` is a string between title and value, `line_sep` is a string between each pair.
//! * `sort(&self, tag: TagType, children: &mut Vec<ModelType>, reverse: bool)` -- Sort the children array by the sorting tag. If reverse if set
//!    this function will sort in reverse order. TagType will be an enum type that decorate the struct.
//! * `get_sort_tag_vec() -> Vec<TagType>` -- Get all available sorting tags for current struct.
//!
//! # Example
//! ```ignore
//! fn decor_fn(item: &Option<f64>) -> String {
//!    format!("{} MB", item.as_ref().unwrap())
//! }
//!
//! #[derive(BelowDecor)]
//! struct CpuModel {
//!     #[bttr(title = "Usage", unit = "%", width = 15, cmp = true)]
//!     usage_pct: Option<f64>,
//!     #[bttr(title = "User", unit = "%", width = 12, cmp = true)]
//!     #[blink("CpuModel$get_usage_pct")]
//!     user_pct: Option<f64>,
//!     #[bttr(title = "System", unit = "%", none_mark = "0.0", width = 12)]
//!     system_pct: Option<f64>,
//!     #[bttr(
//!         title = "L1 Cache",
//!         decorator = "demacia(&$)",
//!         prefix = "\"-->\"",
//!         depth = "5",
//!         width = 12
//!     )]
//!     cache_usage: Option<f64>,
//!     #[blink("CpuModel$get_usage_pct")]
//!     loopback: Option<f64>,
//!     #[blink("CpuModel$get_loopback&")]
//!     route: Option<f64>,
//!     something_else: Option<f64>,
//! }
//! ```

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input::parse, Data::Struct, DeriveInput, Fields};

type Tstream = proc_macro2::TokenStream;

mod attr;
mod field;
mod function;
mod model;

#[proc_macro_derive(BelowDecor, attributes(bttr, blink))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse::<DeriveInput>(input).unwrap();
    let input_ident = &derive_input.ident;

    let members = match derive_input.data {
        Struct(s) => match s.fields {
            Fields::Named(f) => f,
            _ => unimplemented!("Currently only support named struct"),
        },
        _ => unimplemented!("Currently only support struct"),
    };

    let model = model::Model::new_with_members(&members);
    let get_fns = model.generate_get_fns();
    let get_title_fns = model.generate_get_title_fns();
    let get_title_line = model.generate_get_title_line();
    let get_str_impl_fns = model.generate_get_str_impl_fns();
    let get_str_fns = model.generate_get_str_fns();
    let get_field_line = model.generate_get_field_line();
    let get_field_vec = model.generate_get_field_vec();
    let get_title_pipe = model.generate_get_title_pipe();
    let get_csv_field = model.generate_get_csv_field();
    let get_csv_title = model.generate_get_csv_title();
    let cmp_fns = model.generate_cmp_fns();
    let get_interleave_line = model.generate_interleave();
    let sort_fn = model.generate_sort_fn();
    let sort_util = model.generate_sort_util_fns();
    let get_dfill = model.generate_dfill_fns();

    let token = quote! {
        impl #input_ident {
            #get_fns
            #get_title_fns
            #get_str_impl_fns
            #get_str_fns
            #cmp_fns
            #get_title_line
            #get_title_pipe
            #get_field_line
            #get_field_vec
            #get_csv_field
            #get_csv_title
            #get_interleave_line
            #sort_fn
            #sort_util
        }

        #get_dfill
    };

    token.into()
}
