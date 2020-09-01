#![allow(unused)]
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

use crate::*;
use field::Field;

/// Per field fn generator
pub struct Function;

impl Function {
    /// Generate "get" function for THIS field.
    /// A "get" function will return a reference of current field. If current field
    /// is a link, it will follow the link. Please check "gen_get_fn_*" for generated
    /// code
    pub fn gen_get_fn(field: &Field) -> Tstream {
        let fn_name = field.build_fn_name("value");
        if field.is_blink() {
            Self::gen_get_fn_linked_no_aggr(field, fn_name)
        } else if field.is_aggr() {
            Self::gen_get_fn_direct_aggr(field, fn_name)
        } else {
            Self::gen_get_fn_direct_no_aggr(field, fn_name)
        }
    }

    /// Generate "get" function for direct field.
    /// Generated code for `cpu: Option<f64>`:
    /// ```ignore
    /// fn get_cpu_value(&self) -> Option<f64> {
    ///     self.cpu.clone()
    /// }
    /// ```
    fn gen_get_fn_direct_no_aggr(field: &Field, fn_name: Tstream) -> Tstream {
        // We have to borrow the field here since the quote! macro cannot parse self.*
        let field_name = &field.name;
        let field_type = &field.field_type;
        quote! {
            pub fn #fn_name(&self) -> #field_type {
                self.#field_name.clone()
            }
        }
    }

    /// Generate "get" function for aggregated direct field.
    /// Generated code for `total: Option<f64>` that decorated by
    /// `CgroupModel: io_total?.rbytes_per_sec? + io_total?.?`:
    /// ```ignore
    /// fn get_total_value(&self, model: &CgroupModel) -> Option<f64> {
    ///     Some(model
    ///         .io_total
    ///         .as_ref()
    ///         .unwrap_or(&Default::default())
    ///         .rbytes_per_sec
    ///         .as_ref()
    ///         .unwrap_or(&Default::default())
    ///         + model
    ///             .io_total
    ///             .as_ref()
    ///             .unwrap_or(&Default::default())
    ///             .wbytes_per_sec
    ///             .as_ref()
    ///             .unwrap_or(&Default::default()))
    /// }
    /// ```
    fn gen_get_fn_direct_aggr(field: &Field, fn_name: Tstream) -> Tstream {
        let field_type = &field.field_type;
        // use unwrap on the self.aggr_* should be safe since the caller should make sure
        // it is an aggr field before generate.
        let aggr_val = field
            .aggr_val
            .as_ref()
            .unwrap()
            .parse::<Tstream>()
            .expect("Failed to parse aggr_val.");
        let args = field.get_common_args();

        if field.is_option() {
            quote! {
                pub fn #fn_name(#args) -> #field_type {
                    Some(#aggr_val)
                }
            }
        } else {
            quote! {
                pub fn #fn_name(#args) -> #field_type {
                    #aggr_val
                }
            }
        }
    }

    /// Generate "get" function for linked field
    /// For example:
    /// ```ignore
    /// #[blink("CgroupModel$pressure?.")]
    /// cpu_some_pressure: Option<f64>
    /// ```
    /// Will be generated to
    /// ```ignore
    /// pub fn get_cpu_some_pressure_value(&self, model: &CgroupModel) -> &Option<f64> {
    ///     model.pressure.unwrap_or(&Default::default()).get_cpu_some_pressure()
    /// }
    /// ```
    fn gen_get_fn_linked_no_aggr(field: &Field, fn_name: Tstream) -> Tstream {
        let field_type = &field.field_type;
        let blink_type = field
            .blink_type
            .as_ref()
            .unwrap()
            .parse::<Tstream>()
            .expect("Failed to parse blink_type.");

        let blink_val = field.build_fn_interface("value");

        quote! {
            pub fn #fn_name(&self, model: &#blink_type) -> #field_type {
                #blink_val.clone()
            }
        }
    }

    /// Generate title fns
    /// * For direct field
    ///   ```ignore
    ///   #[bttr(title = "Title", width = 10)]
    ///   field: String
    ///   ```
    ///   Will generate:
    ///   ```ignore
    ///   pub fn get_field_title(&self) -> &'static str {
    ///       "Title"
    ///   }
    ///
    ///   pub fn get_field_title_styled(&self) -> &'static str {
    ///       "Title     "
    ///   }
    ///   ```
    /// * For linked field
    ///   ```ignore
    ///   #[blink("Model$")]
    ///   field: String
    ///   ```
    ///   Will generate:
    ///   ```ignore
    ///   pub fn get_field_title(&self, model: &Model) -> &'static str {
    ///       model.get_field_title()
    ///   }
    ///
    ///   pub fn get_field_title_styled(&self, model: &Model) -> &'static str {
    ///       model.get_field_title_styled()
    ///   }
    ///   ```
    pub fn gen_get_title_fn(field: &Field) -> Tstream {
        let fn_name = field.build_fn_name("title");
        let fn_name_styled = field.build_fn_name("title_styled");

        // Parse title field from BelowAttr
        // * Title has value
        //   `format!("{:w$.w$}", &title, w = width)`
        //   width is one of the following, order by priority:
        //     * title_width
        //     * width
        //     * title.len()
        // * Title doesn't have value
        //     * For blink: {self.blink_prefix}_title
        //     * All others: None
        let title = field.field_attr.title.as_ref();
        let args = field.get_common_args();

        let mut styled_return_type = quote! {&'static str};

        let (title, title_styled) = if let Some(t) = title {
            let width = field
                .view_attr
                .title_width
                .unwrap_or_else(|| field.view_attr.width.unwrap_or_else(|| 0));

            let title_str = format!("\"{}\"", t).parse::<Tstream>().unwrap();

            // For linked string, we use the longer one of the current title and linked title.
            let styled_title_str = if field.is_blink() {
                let linked_title = field.build_fn_interface("title_styled");
                styled_return_type = quote! {String};
                quote! {
                    format!("{:w$.w$}", #title_str, w = if #width == 0 {
                        std::cmp::max(#title_str.len(), #linked_title.len())
                    } else {
                        #width
                    })
                }
            } else {
                format!("\"{:w$.w$}\"", t, w = width)
                    .parse::<Tstream>()
                    .unwrap()
            };

            (title_str, styled_title_str)
        } else if field.is_blink() {
            let title_str = field.build_fn_interface("title");

            let title_str_styled =
                if field.view_attr.width.is_none() && field.view_attr.title_width.is_none() {
                    field.build_fn_interface("title_styled")
                } else {
                    let width = field
                        .view_attr
                        .title_width
                        .unwrap_or_else(|| field.view_attr.width.unwrap_or_else(|| 0));
                    quote! {
                        format!("\"{:w$.w$}\"", #title_str, w = #width)
                    }
                };
            (title_str, title_str_styled)
        } else {
            // When calling title function for field that does not have title.
            return quote! {
                pub fn #fn_name(#args) -> &'static str {
                    "unknown"
                }

                pub fn #fn_name_styled(#args) -> &'static str {
                    "unknown"
                }
            };
        };

        quote! {
            pub fn #fn_name(#args) -> &'static str {
                #title
            }

            pub fn #fn_name_styled(#args) -> #styled_return_type {
                #title_styled
            }
        }
    }
}
