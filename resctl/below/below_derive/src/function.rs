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
use field::*;

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

    /// Generate the get_FIELD_str_impl function. Direct field ONLY
    /// Example:
    /// ```ignore
    /// #[bttr(
    ///     title = "Field",
    ///     unit = "/s",
    ///     precision = 2,
    ///     prefix = "-->",
    ///     depth = 2,
    ///     width = 20,
    ///     decorator = "if $ > 0 { $ * 2 } else { 0 }",
    ///     highlight_if = "if $ > 10 {Some(cursive::theme::BaseColor::Red)} else {None}",
    /// )]
    /// field: i32
    /// ```
    /// Will generate:
    /// ```ignore
    /// pub fn get_field_str_impl<P, Q> (
    ///     &self,
    ///     decorator: Option<P>,
    ///     highlight_if: Option<Q>,
    ///     unit: Option<&str>,
    ///     precision: Option<usize>,
    /// ) -> (String, Option<cursive::theme::BaseColor>)
    /// where
    ///     P: Fn(i32) -> Box<dyn std::fmt::Display>,
    ///     Q: Fn(i32) -> Option<cursive::theme::BaseColor>
    /// {
    ///     let precision = precision.map_or(Some(2), |p| Some(p));
    ///     let v = self.get_field_value();
    ///     let value = if let Some(p) = precision {
    ///         if let Some(decor_fn) = decorator {
    ///             format!("{:.precision$}", decor_fn(v), precision = p)
    ///         } else {
    ///             format!("{:.precision$}", if v > 0 { $ * 2 } else { 0 }, precision = p)
    ///         }
    ///     } else {
    ///         if let Some(decor_fn) = decorator {
    ///             format!("{}", decor_fn(v), precision = p)
    ///         } else {
    ///             format!("{}", if v > 0 { $ * 2 } else { 0 }, precision = p)
    ///         }
    ///     }
    ///
    ///     (format!(
    ///         "{}{}",
    ///         value,
    ///         unit.unwrap_or(#unit),
    ///     ),
    ///     if let some(hval_fn) = highlight_if {
    ///         hval_fn(v)
    ///     } else {
    ///         v > 10
    ///     })
    /// }
    ///
    /// pub fn get_field_str_styled_impl<P, Q>(
    ///     &self,
    ///     decorator: Option<P>,
    ///     highlight_if: Option<Q>,
    ///     unit: Option<&str>,
    ///     precision: Option<usize>,
    ///     prefix: Option<&str>,
    ///     depth: Option<usize>,
    ///     width: Option<usize>,
    /// ) -> StyledString
    /// where
    ///     P: Fn(i32) -> Box<dyn std::fmt::Display>,
    ///     Q: Fn(i32) -> Option<cursive::theme::BaseColor>
    /// {
    ///     let (value, highlight) = self.get_field_str_impl(decorator, highlight_if, unit, precision);
    ///     let depth = depth.unwrap_or(#depth);
    ///     let width = width.unwrap_or(#width);
    ///     let width = if width == 0 {
    ///         value.len()
    ///     } else if width >= depth {
    ///         width - depth
    ///     } else {
    ///         0
    ///     };
    ///
    ///     let unhighlighted = format!(
    ///         "{:>depth$.depth$}{:width$.width$}",
    ///         prefix.unwrap_or(#prefix),
    ///         value,
    ///         depth = depth,
    ///         width = width,
    ///     );
    ///
    ///     if highlight {
    ///         StyledString::styled(unhighlighted, cursive::theme::Color::Light(cursive::theme::BaseColor::Red))
    ///     } else {
    ///         StyledString::plain(unhighlighted)
    ///     }
    /// }
    /// ```
    pub fn gen_get_str_impl(field: &Field) -> Tstream {
        let fn_name = field.build_fn_name("str_impl");
        let fn_name_styled = field.build_fn_name("str_styled_impl");

        let unit = field.unit.clone();
        let prefix = field.prefix.clone();
        let depth = field.depth.clone();
        let width = field.width.clone();
        let precision = match field.view_attr.precision {
            Some(v) => quote! {Some(#v)},
            None => quote! {None},
        };

        let get_fn = field.build_fn_interface("value");
        let none_mark = field.view_attr.none_mark.clone();

        let value_type = field.inner_type.as_ref().unwrap_or(&field.field_type);

        let default_decor: Tstream = quote! {
            (|v| {
                let res: Box<dyn std::fmt::Display> = Box::new(v);
                res
            })
        };

        let decor_value = match &field.decor_value {
            Some(d) => d.clone(),
            _ => default_decor,
        };

        let hif_fn = match &field.highlight_if_value {
            Some(h) => h.clone(),
            _ => quote! {
                (|_| None)
            },
        };

        let (args, args_val) = if field.is_aggr() {
            let aggr_type = field
                .blink_type
                .as_ref()
                .unwrap()
                .parse::<Tstream>()
                .unwrap();
            (quote! {model: &#aggr_type}, quote! {model})
        } else {
            (quote! {}, quote! {})
        };

        let value = quote! {
            {let value = if let Some(p) = precision {
                if let Some(decor_fn) = decorator {
                    format!("{:.precision$}", decor_fn(v.clone()), precision = p)
                } else {
                    format!("{:.precision$}", #decor_value(v.clone()), precision = p)
                }
            } else {
                if let Some(decor_fn) = decorator {
                    format!("{}", decor_fn(v.clone()))
                } else {
                    format!("{}", #decor_value(v.clone()))
                }
            };

            format!(
                "{}{}",
                value,
                unit.unwrap_or(#unit)
            )}
        };

        let str_impl_fn = if field.is_option() {
            quote! {
                pub fn #fn_name<P, Q> (
                    &self,
                    decorator: Option<P>,
                    highlight_if: Option<Q>,
                    unit: Option<&str>,
                    precision: Option<usize>,
                    #args
                ) -> (String, Option<cursive::theme::BaseColor>)
                where
                    P: Fn(#value_type) -> Box<dyn std::fmt::Display>,
                    Q: Fn(#value_type) -> Option<cursive::theme::BaseColor>
                {
                    let precision = precision.map_or(#precision, |p| Some(p));
                    if let Some(v) = #get_fn {

                        (#value, if let Some(hif_fn) = highlight_if {
                            hif_fn(v)
                        } else {
                            #hif_fn(v)
                        })
                    } else {
                        (#none_mark.into(), None)
                    }
                }
            }
        } else {
            quote! {
                pub fn #fn_name<P, Q> (
                    &self,
                    decorator: Option<P>,
                    highlight_if: Option<Q>,
                    unit: Option<&str>,
                    precision: Option<usize>,
                    #args
                ) -> (String, Option<cursive::theme::BaseColor>)
                where
                    P: Fn(#value_type) -> Box<dyn std::fmt::Display>,
                    Q: Fn(#value_type) -> Option<cursive::theme::BaseColor>
                {
                    let precision = precision.map_or(#precision, |p| Some(p));
                    let v = #get_fn;
                    (#value, if let Some(hif_fn) = highlight_if {
                        hif_fn(v)
                    } else {
                        #hif_fn(v)
                    })
                }
            }
        };

        quote! {
            #str_impl_fn

            pub fn #fn_name_styled<P, Q>(
                &self,
                decorator: Option<P>,
                highlight_if: Option<Q>,
                unit: Option<&str>,
                precision: Option<usize>,
                prefix: Option<&str>,
                depth: Option<usize>,
                width: Option<usize>,
                #args
            ) -> StyledString
            where
                P: Fn(#value_type) -> Box<dyn std::fmt::Display>,
                Q: Fn(#value_type) -> Option<cursive::theme::BaseColor>
            {
                let (value, highlight_color) = self.#fn_name(decorator, highlight_if, unit, precision, #args_val);
                let depth = depth.unwrap_or(#depth);
                let width = width.unwrap_or(#width);
                let width = if width == 0 {
                    value.len()
                } else if width >= depth {
                    width - depth
                } else {
                    0
                };

                let unhighlighted = (format!(
                    "{:>depth$.depth$}{:width$.width$}",
                    prefix.unwrap_or(#prefix),
                    value,
                    depth = depth,
                    width = width,
                ));
                if let Some(color) = highlight_color {
                    StyledString::styled(unhighlighted, cursive::theme::Color::Light(color))
                } else {
                    StyledString::plain(unhighlighted)
                }
            }
        }
    }

    /// Generate get_FIELD_str and get_FIELD_str_impl function
    pub fn gen_get_str(field: &Field) -> Tstream {
        let fn_name = field.build_fn_name("str");
        let impl_name = field.build_fn_caller("str_impl", "");
        let fn_name_styled = field.build_fn_name("str_styled");
        let impl_name_styled = field.build_fn_caller("str_styled_impl", "");

        if field.is_blink() {
            Self::gen_get_str_blink(field, fn_name, impl_name, fn_name_styled, impl_name_styled)
        } else {
            Self::gen_get_str_direct(field, fn_name, impl_name, fn_name_styled, impl_name_styled)
        }
    }

    /// Generate get_FIELD_str and get_FIELD_str_impl function for direct field:
    /// Example:
    /// ```ignore
    /// #[bttr(
    ///     title = "Field",
    ///     unit = "/s",
    ///     precision = 2,
    ///     prefix = "-->",
    ///     depth = 2,
    ///     width = 20,
    ///     raw = "1 == 1",
    ///     decorator = "if $ > 0 { $ * 2 } else { 0 }"
    /// )]
    /// field: i32
    /// ```
    /// Will Generate:
    /// ```ignore
    /// fn get_field_str(&self) -> String {
    ///     if 1 == 1 {
    ///         self.get_field_str_impl(Some(|v| {Box::new(v)}), None, Some(""), None)
    ///     } else {
    ///         self.get_field_str_impl(None, None, None, None)
    ///     }
    /// }
    /// ```
    fn gen_get_str_direct(
        field: &Field,
        fn_name: Tstream,
        impl_name: Tstream,
        fn_name_styled: Tstream,
        impl_name_styled: Tstream,
    ) -> Tstream {
        let args = field.get_common_args();
        let args_val = if field.is_aggr() {
            quote! {model}
        } else {
            quote! {}
        };

        let value_type = field.inner_type.as_ref().unwrap_or(&field.field_type);
        let decor_val = quote! {None::<Box<dyn Fn(#value_type) -> Box<dyn std::fmt::Display>>>};
        let highligh_val =
            quote! {None::<Box<dyn Fn(#value_type) -> Option<cursive::theme::BaseColor>>>};
        let raw = field.raw.borrow().clone();
        let default_decor: Tstream = quote! {
            (|v| {
                let res: Box<dyn std::fmt::Display> = Box::new(v);
                res
            })
        };
        quote! {
            pub fn #fn_name(#args) -> String {
                if #raw {
                    #impl_name(Some(#default_decor), #highligh_val, Some(""), None, #args_val).0
                } else {
                    #impl_name(#decor_val, #highligh_val, None, None, #args_val).0
                }
            }

            pub fn #fn_name_styled(#args) -> StyledString {
                if #raw {
                    #impl_name_styled(Some(#default_decor), #highligh_val, Some(""), None, None, None, None, #args_val)
                } else {
                    #impl_name_styled(#decor_val, #highligh_val, None, None, None, None, None, #args_val)
                }
            }
        }
    }

    /// Generate get_FIELD_str and get_FIELD_str_impl function for linked field:
    /// Example:
    /// ```ignore
    /// #[bttr(
    ///     title = "Field",
    ///     unit = "/s",
    ///     precision = 2,
    ///     prefix = "-->",
    ///     depth = 2,
    ///     width = 20,
    ///     raw = "1 == 1",
    ///     decorator = "if $ > 0 { $ * 2 } else { 0 }"
    /// )]
    /// #[blink(ModelType$.)]
    /// field: i32
    /// ```
    /// Will Generate:
    /// ```ignore
    /// fn get_field_str(&self, model:&ModelType) -> String {
    ///     if 1 == 1 {
    ///         model.get_field_str_impl(Some(|v| {Box::new(v)}), None, Some(""), Some(2))
    ///     } else {
    ///         model.get_field_str_impl(Some(|v: i32| if v > 0 { v * 2} else { 0 }), None, Some("/s"), Some(2))
    ///     }
    /// }
    /// ```
    fn gen_get_str_blink(
        field: &Field,
        fn_name: Tstream,
        impl_name: Tstream,
        fn_name_styled: Tstream,
        impl_name_styled: Tstream,
    ) -> Tstream {
        let blink_type = field
            .blink_type
            .as_ref()
            .unwrap()
            .parse::<Tstream>()
            .unwrap();
        let unit = field.view_attr.unit.as_ref().map_or_else(
            || quote! {None},
            |_| {
                let v = field.unit.clone();
                quote! {Some(#v)}
            },
        );
        let prefix = field.view_attr.prefix.as_ref().map_or_else(
            || quote! {None},
            |_| {
                let v = field.prefix.clone();
                quote! {Some(#v)}
            },
        );
        let depth = field.view_attr.depth.as_ref().map_or_else(
            || quote! {None},
            |_| {
                let v = field.depth.clone();
                quote! {Some(#v)}
            },
        );
        let width = field.view_attr.width.as_ref().map_or_else(
            || quote! {None},
            |_| {
                let v = field.width.clone();
                quote! {Some(#v)}
            },
        );
        let precision = match field.view_attr.precision {
            Some(v) => quote! {Some(#v)},
            None => quote! {None},
        };

        let value_type = field.inner_type.as_ref().unwrap_or(&field.field_type);

        let decor_value = match &field.decor_value {
            Some(f) => quote! {Some(#f)},
            None => quote! {None::<Box<dyn Fn(#value_type) -> Box<dyn std::fmt::Display>>>},
        };

        let hif_val = match &field.highlight_if_value {
            Some(f) => quote! {Some(#f)},
            None => quote! {None::<Box<dyn Fn(#value_type) -> Option<cursive::theme::BaseColor>>>},
        };

        let args_val = if field.is_aggr() {
            quote! {model}
        } else {
            quote! {}
        };

        let raw = field.raw.borrow().clone();
        let default_decor: Tstream = quote! {
            (|v| {
                let res: Box<dyn std::fmt::Display> = Box::new(v);
                res
            })
        };
        quote! {
            pub fn #fn_name(&self, model: &#blink_type) -> String {
                if #raw {
                    #impl_name(Some(#default_decor), #hif_val, Some(""), #precision, #args_val).0
                } else {
                    #impl_name(#decor_value, #hif_val, #unit, #precision, #args_val).0
                }
            }

            pub fn #fn_name_styled(&self, model: &#blink_type) -> StyledString {
                if #raw {
                    #impl_name_styled(Some(#default_decor), #hif_val, Some(""), #precision, #prefix, #depth, #width, #args_val)
                } else {
                    #impl_name_styled(#decor_value, #hif_val, #unit, #precision, #prefix, #depth, #width, #args_val)
                }
            }
        }
    }

    /// Generate per field cmp function
    /// Example:
    /// ```ignore
    /// pub fn cmp_by_FIELD(left: &Model, right: &Model) -> Option<std::cmp::Ordering> {
    ///     left.get_FIELD().partial_cmp(right.get_field())
    /// }
    /// ```
    pub fn gen_cmp_fn(field: &Field) -> Tstream {
        let args_type = if field.is_blink() || field.is_aggr() {
            field
                .blink_type
                .as_ref()
                .unwrap()
                .parse::<Tstream>()
                .unwrap()
        } else {
            quote! {Self}
        };

        // Aggr field will need to call on self
        let caller = if field.is_aggr() || field.is_blink() {
            Some("self")
        } else {
            None
        };

        let left_caller = field.build_custom_caller(
            "value",
            caller.unwrap_or("left"),
            CallSelf(true),
            Some("left"),
        );
        let right_caller = field.build_custom_caller(
            "value",
            caller.unwrap_or("right"),
            CallSelf(true),
            Some("right"),
        );

        let fn_name = format!("cmp_by_{}", field.name).parse::<Tstream>().unwrap();

        quote! {
            pub fn #fn_name(&self, left: &#args_type, right: &#args_type) -> Option<std::cmp::Ordering> {
                #left_caller.partial_cmp(&#right_caller)
            }
        }
    }
}
