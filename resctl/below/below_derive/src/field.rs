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
use attr::*;

/// Struct to indicate if a function is called on `self` object.
pub struct CallSelf(pub bool);

impl CallSelf {
    pub fn is_true(&self) -> bool {
        let CallSelf(res) = self;
        *res
    }
}

/// A field is an instance of a struct field name with its parsed decoration attributes
pub struct Field {
    // name of the field
    pub name: syn::Ident,
    pub field_type: syn::Type,
    // name of the inner type of option field
    pub inner_type: Option<syn::Type>,
    // Unwrap the field attr into Field, one less if layer during generation
    pub field_attr: attr_new::BelowFieldAttr,
    // Unwrap the view attr into Field, one less if layer during generation
    pub view_attr: attr_new::BelowViewAttr,
    // Generated expr of aggregated field, more details in the comment of `parse_blink`
    pub aggr_val: Option<String>,
    // The linked model type, more details in the comment of `parse_blink`
    pub blink_type: Option<String>,
    // The linked field prefix, more details in the comment of `parse_blink`
    pub blink_prefix: Option<String>,
    // The sort tag enum type, more in the comment of `parse_sort_tag`
    pub sort_tag_type: Option<Tstream>,
    // The sort tag enum value, more in the comment of `parse_sort_tag`
    pub sort_tag_val: Option<Tstream>,
    // Parsed display related values. More in `parse_decor`
    pub decor_value: Option<Tstream>,
    pub prefix: Tstream,
    pub depth: Tstream,
    pub width: Tstream,
    pub unit: String,
    // Parse dfill related boxed fn handle. More in `parse_dfill_tag`
    pub dfill_tag_title: Option<Tstream>,
    pub dfill_tag_title_styled: Option<Tstream>,
    pub dfill_tag_field: Option<Tstream>,
    pub dfill_tag_field_styled: Option<Tstream>,
}

impl Field {
    /// Generate new field from the attributes
    pub fn new_with_attr(
        name: syn::Ident,
        field_type: syn::Type,
        attr: attr_new::BelowAttr,
    ) -> Field {
        let inner_type = Self::parse_option(&field_type);
        let is_option = inner_type.is_some();
        let field_attr = attr.field.unwrap_or_default();
        let view_attr = attr.view.unwrap_or_default();
        let width = view_attr.width.unwrap_or(0);
        let (sort_tag_type, sort_tag_val) = Self::parse_sort_tag(&field_attr, &name);

        Field {
            name: name.clone(),
            field_type: field_type.clone(),
            inner_type: inner_type.clone(),
            field_attr: field_attr.clone(),
            view_attr: view_attr.clone(),
            aggr_val: Self::parse_aggr_val(&field_attr.link, is_option),
            blink_type: Self::parse_blink_type(&field_attr.link),
            blink_prefix: Self::parse_blink_prefix(&field_attr.link, &name),
            sort_tag_type,
            sort_tag_val,
            decor_value: Self::parse_decor(&view_attr, &inner_type, &field_type),
            prefix: view_attr.prefix.clone().unwrap_or_else(|| quote! {""}),
            depth: view_attr.depth.clone().unwrap_or_else(|| quote! {0}),
            width: quote! {#width},
            unit: view_attr.unit.clone().unwrap_or_else(|| "".into()),
            dfill_tag_title: None,
            dfill_tag_title_styled: None,
            dfill_tag_field: None,
            dfill_tag_field_styled: None,
        }
        .build_dfill_tags()
    }

    // Parse `blink_type`, `blink_prefix` and `aggr_val` from `BelowAttr`
    // `blink_type`, `blink_prefix`, and `aggr_val` are parsed from the BelowFieldAttr::link,
    // which is in form of "Type$call_path". For example:
    // `CgroupModel$cpu?.get_cpu`
    // will be parsed to:
    // ```ignore
    // blink_type: "CgroupModel"
    // blink_prefix: "model.cpu.unwrap_or(&Default::default()).get_cpu"
    // ```
    // Multi blinks will aggregate the blink value, all of the blink val should have same type.
    // ```ignore
    // #[blink(CgroupModel$cpu?.get_system_usage)]
    // #[blink(CgroupModel$cpu?.get_user_usage)]
    // cpu_total
    // ```
    // Will generate:
    // ```ignore
    // blink_type: "CgroupModel"
    // blink_prefix: None
    // aggr_value:
    //     model.cpu.unwrap_or(&Default::default()).get_system_usage_value().unwrap_or_default()
    //      + model.cpu.unwrap_or(&Default::default()).get_user_usage_value().unwrap_or_default()
    // ```
    //
    // ## Char replacing:
    // * "?": indicates the marked value is an Option, will be replaced to `unwrap_or(&Default::default())`
    //
    // ## Note (single blink ONLY):
    // For convenience, will use current field name for prefix if the prefix is omitted. For example:
    // ```ignore
    // #[blink("CgroupModel$pressure?.")]
    // cpu_some_pressure
    // ```
    // is equal to
    // ```ignore
    // #[blink("CgroupModel$pressure?.get_cpu_some_pressure")]
    // cpu_some_pressure
    // ```
    fn get_blink_vec(blink: &[String]) -> Vec<String> {
        if blink.is_empty() {
            return vec![];
        }

        // "CgroupModel$cpu?.get_cpu" to
        // "CgroupModel$cpu.as_ref().unwrap_or(&Default::default()).get_cpu"
        let link = blink[0].replace("?", ".as_ref().unwrap_or(&Default::default())");
        let link_vec = link
            .split('$')
            .map(|v| v.to_string())
            .collect::<Vec<String>>();

        if link_vec.len() != 2 {
            unimplemented!("Link format error, expect \"ModelType$get_field_name\".");
        }

        link_vec
    }

    fn parse_blink_type(blink: &[String]) -> Option<String> {
        if blink.is_empty() {
            return None;
        }

        let link_vec = Self::get_blink_vec(blink);

        // ["CgroupModel", "cpu.as_ref().unwrap_or(...).get_cpu"]
        Some(link_vec[0].trim().to_string())
    }

    fn parse_blink_prefix(blink: &[String], name: &syn::Ident) -> Option<String> {
        if blink.len() != 1 {
            return None;
        }

        let link_vec = Self::get_blink_vec(blink);
        let mut blink_prefix = link_vec[1].trim().to_string();

        // Handle omitted field name replacement.
        if blink_prefix.is_empty() || blink_prefix.ends_with('.') {
            blink_prefix.push_str(&format!("get_{}", name));
        }
        Some(format!("model.{}", blink_prefix))
    }

    fn parse_aggr_val(blink: &[String], is_option: bool) -> Option<String> {
        if blink.len() < 2 {
            return None;
        }

        Some(
            blink
                .iter()
                .map(|link| {
                    let link = link.replace("?", ".as_ref().unwrap_or(&Default::default())");
                    let link_vec = link.split('$').collect::<Vec<&str>>();
                    let mut handle = format!("model.{}_value()", link_vec[1].trim());
                    if is_option {
                        handle.push_str(".unwrap_or_default()")
                    }
                    handle
                })
                .collect::<Vec<String>>()
                .join("+"),
        )
    }

    /// Convenience function to check if a field is a blink field
    pub fn is_blink(&self) -> bool {
        self.blink_type.is_some() && self.blink_prefix.is_some()
    }

    /// Convenience function to check if a field is an aggregated field.
    pub fn is_aggr(&self) -> bool {
        self.blink_type.is_some() && self.aggr_val.is_some()
    }

    /// Parse the sort tag string into type and value
    /// Example: ProcessTag::Cmdline
    /// => sort_type = ProcessTag,
    /// => sort_val = ProcessTag::Cmdline,
    fn parse_sort_tag(
        field_attr: &attr_new::BelowFieldAttr,
        name: &syn::Ident,
    ) -> (Option<Tstream>, Option<Tstream>) {
        if let Some(sort_tag) = field_attr.sort_tag.as_ref() {
            let sort_tag_path = sort_tag.split(':').collect::<Vec<&str>>();
            if sort_tag_path.len() < 2 {
                unimplemented!("{}: wrong sort tag format, expect EnumType::EnumVal", name);
            }
            let sort_tag_type = Some(sort_tag_path[0].parse().unwrap());
            let sort_tag_val = Some(sort_tag.parse().unwrap());
            (sort_tag_type, sort_tag_val)
        } else {
            (None, None)
        }
    }

    /// If a field's type is option, we will parse the inner type of the field
    fn parse_option(field_type: &syn::Type) -> Option<syn::Type> {
        match field_type {
            syn::Type::Path(ref ty_path) if ty_path.path.segments[0].ident == "Option" => {
                match ty_path.path.segments[0].arguments {
                    syn::PathArguments::AngleBracketed(ref angle_args) => {
                        match angle_args.args[0] {
                            syn::GenericArgument::Type(ref ty) => Some(ty.clone()),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Convenience function to check if a field is an Option
    pub fn is_option(&self) -> bool {
        self.inner_type.is_some()
    }

    /// Convenience pre-processing function for parsing the view related attribute
    /// decor_value: user input or empty. Replaced $ with v
    ///   "v" here represents the parsed value after get. Possible code:
    /// ```ignore
    /// // field is an option
    /// match self.get_field() {
    ///     Some(v) => ...
    ///     _ => format!("{}", none_mark)
    /// }
    ///
    /// // field is not an option
    /// let v = self.get_field()
    /// ```
    fn parse_decor(
        view_attr: &attr_new::BelowViewAttr,
        inner_type: &Option<syn::Type>,
        field_type: &syn::Type,
    ) -> Option<Tstream> {
        if let Some(decor) = view_attr.decorator.as_ref() {
            let decor_value = decor.replace("$", "v").parse::<Tstream>().unwrap();
            let value_type = inner_type.as_ref().unwrap_or(&field_type);

            Some(quote! {
                (|v: #value_type| {
                    let res = #decor_value;
                    let res: Box<dyn std::fmt::Display> = Box::new(res);
                    res
                })
            })
        } else {
            None
        }
    }

    /// Unified function argument convention
    pub fn get_common_args(&self) -> Tstream {
        if self.is_blink() || self.is_aggr() {
            let blink_type = self
                .blink_type
                .as_ref()
                .unwrap()
                .parse::<Tstream>()
                .unwrap();
            quote! {&self, model: &#blink_type}
        } else {
            quote! {&self}
        }
    }

    /// Unified function caller argument convention
    pub fn get_common_args_value(&self, call_self: CallSelf, arg: Option<&str>) -> Tstream {
        if (self.is_blink() && call_self.is_true()) || self.is_aggr() {
            let arg = arg.unwrap_or("model").parse::<Tstream>().unwrap();
            quote! {#arg}
        } else {
            quote! {}
        }
    }

    /// Convenience function for build function interface
    /// This function will enforce all generated function following same pattern
    /// Example: get_field_value
    pub fn build_fn_name(&self, sub_fn: &str) -> Tstream {
        format!("get_{}_{}", self.name, sub_fn).parse().unwrap()
    }

    /// Convenience function for build function interface
    /// This function will enforce all generated function following same pattern
    /// Example: self.get_field_value(model)
    pub fn build_fn_interface(&self, sub_fn: &str) -> Tstream {
        self.build_fn_caller(
            sub_fn,
            &format!("({})", &self.get_common_args_value(CallSelf(false), None)),
        )
    }

    /// Convenience function for build function interface
    /// Example: model.get_field_value()
    pub fn build_fn_caller(&self, sub_fn: &str, args: &str) -> Tstream {
        if self.is_blink() {
            format!("{}_{}{}", self.blink_prefix.as_ref().unwrap(), sub_fn, args)
                .parse()
                .unwrap()
        } else {
            format!("self.{}{}", self.build_fn_name(sub_fn), args)
                .parse()
                .unwrap()
        }
    }

    /// Convenience function for build function interface on self
    pub fn build_self_caller(&self, sub_fn: &str) -> Tstream {
        self.build_custom_caller(sub_fn, "self", CallSelf(true), None)
    }

    /// Convenience function for build function interface on caller
    /// `arg` here will be used for replace unified item with special item
    pub fn build_custom_caller(
        &self,
        sub_fn: &str,
        caller: &str,
        call_self: CallSelf,
        arg: Option<&str>,
    ) -> Tstream {
        format!(
            "{}.{}({})",
            caller,
            self.build_fn_name(sub_fn),
            self.get_common_args_value(call_self, arg)
        )
        .parse()
        .unwrap()
    }

    fn is_dfill_tagged(&self) -> bool {
        self.field_attr.tag.is_some() || self.field_attr.class.is_some()
    }

    fn build_dfill_tags(mut self) -> Self {
        if !self.is_dfill_tagged() {
            return self;
        }

        self.dfill_tag_title = Some(
            format!(
                "Box::new(|data, model| {}.into())",
                self.build_custom_caller("title", "data", CallSelf(true), None)
            )
            .parse()
            .unwrap(),
        );
        self.dfill_tag_title_styled = Some(
            format!(
                "Box::new(|data, model| {}.into())",
                self.build_custom_caller("title_styled", "data", CallSelf(true), None)
            )
            .parse()
            .unwrap(),
        );
        self.dfill_tag_field = Some(
            format!(
                "Box::new(|data, model| {})",
                self.build_custom_caller("str", "data", CallSelf(true), None)
            )
            .parse()
            .unwrap(),
        );
        self.dfill_tag_field_styled = Some(
            format!(
                "Box::new(|data, model| {}.source().to_string())",
                self.build_custom_caller("str_styled", "data", CallSelf(true), None)
            )
            .parse()
            .unwrap(),
        );
        self
    }
}

/// Generate get function for direct field.
/// # note
/// This function will not honor decorator.
pub fn gen_get_function_for_direct_field(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_get = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().link.is_none())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let ty = &f.ty;
            let fn_name = Ident::new(&format!("get_{}", name), Span::call_site());

            if let Some(aggr) = a.field.as_ref().unwrap().aggr.clone() {
                let aggr_vec: Vec<&str> = aggr.split(':').collect();
                if aggr_vec.len() != 2 {
                    unimplemented!("Expect \"Type: a + b\" format for aggregator");
                }
                let aggr_type = aggr_vec[0].parse::<Tstream>().unwrap();
                let mut val = String::new();
                aggr_vec[1].split('+').for_each(|item| {
                    let v = item
                        .trim()
                        .to_string()
                        .replace("?", ".as_ref().unwrap_or(&Default::default())");
                    val.push_str(&format!("input.{}+", v));
                });
                val.pop();
                let val = val.parse::<Tstream>().unwrap();
                // Need to verity #ty here.
                quote! {
                    pub fn #fn_name(input: &#aggr_type) -> #ty {
                        Some(#val)
                    }
                }
            } else {
                quote! {
                    pub fn #fn_name<'a>(&'a self) -> &'a #ty {
                        &self.#name
                    }
                }
            }
        });

    quote! {
        #(#per_field_get)*
    }
}

/// Generate title for each field
/// This function will generate both styled and unstyled title. It will evaluate `width`,
/// `title_width`, `title_depth`, and `title_prefix` attributes. `title_width` will override `width`.
///
/// For linked fields, title within current field will always override the destination. The
/// generation function will try to follow the link and use the first title it sees.
pub fn gen_get_title_per_field(fields: &syn::FieldsNamed) -> Tstream {
    let direct_title = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().title.is_some())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let fn_name = format!("get_{}_title", name).parse::<Tstream>().unwrap();
            let fn_name_styled = format!("get_{}_title_styled", name)
                .parse::<Tstream>()
                .unwrap();
            let title = a.field.as_ref().unwrap().title.as_ref().unwrap().clone();
            let aview = a.view.as_ref().unwrap();
            let width = aview
                .title_width
                .unwrap_or_else(|| aview.width.unwrap_or_else(|| title.len()));
            let prefix = aview.title_prefix.clone().unwrap_or_else(|| "".into());
            let depth = aview.title_depth.unwrap_or(0);
            let title_styled = format!("{:>d$.d$}{:w$.w$}", prefix, &title, d = depth, w = width);
            let args = if a.field.as_ref().unwrap().aggr.is_some() {
                quote! {}
            } else {
                quote! {&self}
            };

            quote! {
                pub fn #fn_name(#args) -> &'static str {
                    #title
                }

                pub fn #fn_name_styled(#args) -> &'static str {
                    #title_styled
                }
            }
        });

    let linked_title = iter_field_attr!(fields)
        .filter(|(_, a)| {
            a.field.is_some()
                && a.field.as_ref().unwrap().title.is_none()
                && a.field.as_ref().unwrap().link.is_some()
        })
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let link = a.field.unwrap().link.unwrap();
            let link = link.replace("?", ".as_ref().unwrap_or(&Default::default())");
            let link = link.split('$').collect::<Vec<&str>>();

            if link.len() == 2 {
                let m_type = link[0].parse::<Tstream>().unwrap();
                let (link, link_styled) = if link[1].ends_with('&') {
                    let mut link = link[1].to_string();
                    link.pop();
                    let target = format!("{}_title", &link).parse::<Tstream>().unwrap();
                    let target_styled = format!("{}_title_styled", &link)
                        .parse::<Tstream>()
                        .unwrap();
                    (
                        quote! {
                            model.#target(model)
                        },
                        quote! {
                            model.#target_styled(model)
                        },
                    )
                } else {
                    let target = format!("{}_title", &link[1]).parse::<Tstream>().unwrap();
                    let target_styled = format!("{}_title_styled", &link[1])
                        .parse::<Tstream>()
                        .unwrap();
                    (
                        quote! {
                            model.#target()
                        },
                        quote! {
                            model.#target_styled()
                        },
                    )
                };
                let fn_name = Ident::new(&format!("get_{}_title", name), Span::call_site());
                let fn_name_styled =
                    Ident::new(&format!("get_{}_title_styled", name), Span::call_site());

                quote! {
                    pub fn #fn_name(&self, model: &#m_type) -> &'static str {
                        #link
                    }

                    pub fn #fn_name_styled(&self, model: &#m_type) -> &'static str {
                        #link_styled
                    }
                }
            } else {
                unimplemented!("Expecting a model for link");
            }
        });

    quote! {
        #(#direct_title)*
        #(#linked_title)*
    }
}

/// Generate get function for linked field.
/// The get function will call the get function of the destination or next hop.
pub fn gen_get_function_for_linked_field(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_get = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().link.is_some())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let ty = &f.ty;
            let link = a.field.unwrap().link.unwrap();
            let link = link.replace("?", ".as_ref().unwrap_or(&Default::default())");
            let link = link.split('$').collect::<Vec<&str>>();

            if link.len() == 2 {
                let m_type = link[0].parse::<Tstream>().unwrap();
                let link = if link[1].ends_with('&') {
                    let mut link = link[1].to_string();
                    link.pop();
                    let target = link.parse::<Tstream>().unwrap();
                    quote! {
                        model.#target(model)
                    }
                } else {
                    let target = link[1].parse::<Tstream>().unwrap();
                    quote! {
                        model.#target()
                    }
                };
                let fn_name = Ident::new(&format!("get_{}", name), Span::call_site());

                quote! {
                    pub fn #fn_name(&self, model: &#m_type) -> #ty {
                        &self.#name;
                        #link.clone()
                    }
                }
            } else {
                unimplemented!("Expecting a model for link");
            }
        });

    quote! {
        #(#per_field_get)*
    }
}

/// Generate comparison functions
/// Nothing but a partial compare on direct field. For linked field,
/// the argument will take two model and compare base on the get function.
pub fn gen_cmp_fns(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_cmp = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().cmp)
        .map(|(f, a)| {
            let field = a.field.as_ref().unwrap();
            let name = &f.ident.clone().unwrap();
            let fn_name = format!("cmp_by_{}", &name).parse::<Tstream>().unwrap();
            if field.link.is_some() {
                let link = a.field.unwrap().link.unwrap();
                let link = link.replace("?", ".as_ref().unwrap_or(&Default::default())");
                let link = link.split('$').collect::<Vec<&str>>();
                if link.len() == 2 {
                    let m_type = link[0].parse::<Tstream>().unwrap();
                    if link[1].ends_with('&') {
                        let mut link = link[1].to_string();
                        link.pop();
                        let target = link.parse::<Tstream>().unwrap();
                        quote! {
                            pub fn #fn_name(left: &#m_type, right: &#m_type) -> Option<std::cmp::Ordering> {
                                left.#target(model).partial_cmp(&right.#target(model))
                            }
                        }
                    } else {
                        let target = link[1].parse::<Tstream>().unwrap();
                        quote! {
                            pub fn #fn_name(left: &#m_type, right: &#m_type) -> Option<std::cmp::Ordering> {
                                left.#target().partial_cmp(&right.#target())
                            }
                        }
                    }
                } else {
                    unimplemented!("Expecting a model for link");
                }
            } else if field.aggr.is_some() {
                let aggr = field.aggr.clone().unwrap();
                let aggr_vec: Vec<&str> = aggr.split(':').collect();
                if aggr_vec.len() != 2 {
                    unimplemented!("Expect \"Type: a + b\" format for aggregator");
                }
                let aggr_type = aggr_vec[0].parse::<Tstream>().unwrap();
                let call_name = format!("get_{}", name).parse::<Tstream>().unwrap();
                quote! {
                    pub fn #fn_name(left: &#aggr_type, right: &#aggr_type) -> Option<std::cmp::Ordering> {
                        Self::#call_name(left).partial_cmp(&Self::#call_name(right))
                    }
                }
            } else {
                let call_name = format!("get_{}", name).parse::<Tstream>().unwrap();
                quote! {
                    pub fn #fn_name(left: &Self, right: &Self) -> Option<std::cmp::Ordering> {
                        left.#call_name().partial_cmp(&right.#call_name())
                    }
                }
            }
        });

    quote! {
        #(#per_field_cmp)*
    }
}

/// Generate sorting functions
/// For all fields that decorated by `cmp`, we will automatically generate a cmp_by_FIELD_NAME function. And for
/// All fields that decorated by `sort_tag`, we will use the associate cmp_by_FIELD_NAME function and generate a sort
/// function. The generated code will be something like this:
/// ```
/// fn sort(&self, tag: TagType, children: Vec<ModelType>, reverse: bool) {
///     match tag {
///         TagType::Tag1 => children.sort_by(|lhs, rhs| {
///             if reverse {
///                 Self::cmp_by_FIELD_NAME1(&lhs, &rhs)
///                     .unwrap_or(std::cmp::Ordering::Equal)
///                     .reverse()
///             } else {
///                 Self::#cmp_by_FIELD_NAME1(&lhs, &rhs)
///                     .unwrap_or(std::cmp::Ordering::Equal)
///             }
///         }),
///         ...
///         _ => ()
///     }
/// }
/// ```
/// All fields without a sorting tag will automatically tagged by TagType::Keep. So when defining sorting tag,
/// it's mandatory to have a field named Keep. Please note that all fields that are decorated by `sort_tag` will be automatically
/// set `cmp = true`
pub fn gen_tag_sort_fn(fields: &syn::FieldsNamed) -> Tstream {
    let enum_type = fields
        .named
        .iter()
        .map(|f| parse_attribute(&f.attrs, &f.ident.clone().unwrap()))
        .filter(|a| a.field.is_some() && a.field.as_ref().unwrap().sort_tag.is_some())
        .find_map(|a| {
            let enum_val = a.field.as_ref().unwrap().sort_tag.as_ref().unwrap();
            let enum_val_path = enum_val.split(':').collect::<Vec<&str>>();
            Some(enum_val_path[0].to_string())
        });

    if enum_type == None {
        return quote! {};
    }

    let model_type = fields
        .named
        .iter()
        .map(|f| parse_attribute(&f.attrs, &f.ident.clone().unwrap()))
        .filter(|a| a.field.is_some() && a.field.as_ref().unwrap().link.is_some())
        .find_map(|a| {
            let link = a.field.unwrap().link.unwrap();
            let link = link.split('$').collect::<Vec<&str>>();
            Some(link[0].to_string())
        });

    let model_type = if let Some(mt) = model_type {
        mt.parse::<Tstream>().unwrap()
    } else {
        "Self".parse::<Tstream>().unwrap()
    };

    let enum_type = enum_type.unwrap().parse::<Tstream>().unwrap();

    let tag_items_vec = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some())
        .map(|(_, a)| {
            let enum_val = if let Some(val) = a.field.as_ref().unwrap().sort_tag.as_ref() {
                val.to_string()
            } else {
                format!("{}::Keep", &enum_type)
            };

            enum_val.parse::<Tstream>().unwrap()
        });

    // We clone the tag_items_vec here is because quote!{get_sort_tag_vec} will consume
    // the iterator.
    let tag_items_has_tag = tag_items_vec.clone();

    let match_arms = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().sort_tag.is_some())
        .map(|(f, a)| {
            let enum_val = a
                .field
                .unwrap()
                .sort_tag
                .unwrap()
                .parse::<Tstream>()
                .unwrap();
            let name = &f.ident.clone().unwrap();
            let cmp_fn_name = format!("cmp_by_{}", name).parse::<Tstream>().unwrap();
            quote! {
                #enum_val => children.sort_by(|lhs, rhs| {
                    if reverse {
                        Self::#cmp_fn_name(&lhs, &rhs)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .reverse()
                    } else {
                        Self::#cmp_fn_name(&lhs, &rhs)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                })
            }
        });

    quote! {
        pub fn sort(tag: #enum_type, children: &mut Vec<&#model_type>, reverse: bool) {
            match tag {
                #(#match_arms,)*
                _ => (),
            };
        }

        pub fn has_tag(tag: #enum_type) -> bool {
            match tag {
                #(#tag_items_has_tag => true,)*
                _ => false
            }
        }

        pub fn get_sort_tag_vec() -> Vec<#enum_type> {
            vec![#(#tag_items_vec,)*]
        }
    }
}

fn get_dfill_field_fns(
    fields: &syn::FieldsNamed,
    suffix: &str,
    fn_type: &Tstream,
    title: bool,
) -> Tstream {
    let styled_str_extension = if suffix == "str_styled" && !title {
        quote! {.source().to_string()}
    } else {
        quote! {}
    };

    let per_field_fns = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().tag.is_some())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let mut match_arm = a.field.as_ref().unwrap().tag.clone().unwrap();
            let aggr = a.field.as_ref().unwrap().aggr.is_some();
            let fn_name = format!("get_{}_{}", name, suffix)
                .parse::<Tstream>()
                .unwrap();
            // If such field does not have a title, we will treat it as a pure link field and
            // generate link related functions.
            // TODO: Put all tag and sort_tag into independent struct of attribute. This will
            // reduce the decorator size a lot. Because we can reuse the original view attr instead
            // of writing new one.T67969117
            let link_title = a.field.as_ref().unwrap().title.is_none();
            if match_arm.ends_with('&') {
                match_arm.pop();
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                match (title, link_title, aggr) {
                    (true, false, true) => quote! {
                        #match_arm => self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name())),
                    },
                    (true, false, false) => quote! {
                        #match_arm => self.#fn_type.push(Box::new(|data, _| data.#fn_name())),
                    },
                    (_, _, true) => quote! {
                        #match_arm => self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name(model)#styled_str_extension)),
                    },
                    _ => quote! {
                        #match_arm => self.#fn_type.push(Box::new(|data, model| data.#fn_name(model)#styled_str_extension)),
                    }
                }
            } else {
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                quote! {
                    #match_arm => self.#fn_type.push(Box::new(|data, _| data.#fn_name()#styled_str_extension)),
                }
            }
        });

    quote! {
        #(#per_field_fns)*
    }
}

fn get_dfill_class_field(
    field: &str,
    suffix: &str,
    fn_type: &Tstream,
    title: bool,
    styled_str_extension: &Tstream,
) -> Tstream {
    let aggr = field.ends_with('@');
    let link = field.ends_with('&');
    // We use additional "&" to mark the title link. With T67969117 we can get rid of it.
    let link_title = field.ends_with("&&") || field.ends_with("&@");
    if aggr || link {
        let mut field = field.to_string();
        field.pop();
        if link_title {
            field.pop();
        }
        let fn_name = format!("get_{}_{}", field, suffix)
            .parse::<Tstream>()
            .unwrap();
        match (title, link_title, aggr) {
            (true, false, true) => quote! {
                self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name()));
            },
            (true, false, false) => quote! {
                self.#fn_type.push(Box::new(|data, model| data.#fn_name()));
            },
            (_, _, true) => quote! {
                self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name(model)#styled_str_extension));
            },
            _ => quote! {
                self.#fn_type.push(Box::new(|data, model| data.#fn_name(model)#styled_str_extension));
            },
        }
    } else {
        let fn_name = format!("get_{}_{}", field, suffix)
            .parse::<Tstream>()
            .unwrap();
        quote! {
            self.#fn_type.push(Box::new(|data, _| data.#fn_name()#styled_str_extension));
        }
    }
}

fn get_dfill_class_fns(
    fields: &syn::FieldsNamed,
    suffix: &str,
    fn_type: &Tstream,
    title: bool,
) -> Tstream {
    let styled_str_extension = if suffix == "str_styled" && !title {
        quote! {.source().to_string()}
    } else {
        quote! {}
    };

    let per_class_fns = iter_field_attr!(fields)
        .filter(|(_, a)| a.class.is_some())
        .map(|(f, a)| {
            let mut name: Vec<char> = f.ident.clone().unwrap().to_string().chars().collect();
            name[0] = name[0].to_uppercase().next().unwrap();
            let name = name
                .into_iter()
                .collect::<String>()
                .parse::<Tstream>()
                .unwrap();
            let class_handle = a.class.unwrap();
            let class = class_handle.split('$').collect::<Vec<&str>>();
            if class.len() != 2 {
                unimplemented!("Bad class format, expect: TYPE$field1,field2&,field3");
            }

            let field_type = class[0].parse::<Tstream>().unwrap();
            let fields = class[1].split(':').collect::<Vec<&str>>();
            let fields_reg = fields[0].split(',').collect::<Vec<&str>>();

            let fields_fns_reg = fields_reg.iter().map(|field| {
                get_dfill_class_field(field, suffix, fn_type, title, &styled_str_extension)
            });

            if fields.len() > 1 {
                let fields_detail = fields[1].split(',').collect::<Vec<&str>>();
                let fields_fns_detail = fields_detail.iter().map(|field| {
                    get_dfill_class_field(field, suffix, fn_type, title, &styled_str_extension)
                });

                quote! {
                    #field_type::#name => {
                        #(#fields_fns_reg)*

                        if !self.opts.detail {
                            return ();
                        }
                        #(#fields_fns_detail)*
                    }
                }
            } else {
                quote! {
                    #field_type::#name => {
                        #(#fields_fns_reg)*
                    }
                }
            }
        });

    quote! {
        #(#per_class_fns)*
    }
}

fn get_dfill_cmp_fns(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_fns = iter_field_attr!(fields)
        .filter(|(_, a)| {
            a.field.is_some()
                && a.field.as_ref().unwrap().tag.is_some()
                && a.field.as_ref().unwrap().cmp
        })
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let mut match_arm = a.field.as_ref().unwrap().tag.clone().unwrap();
            let fn_name = format!("cmp_by_{}", name).parse::<Tstream>().unwrap();
            if match_arm.ends_with('&') {
                match_arm.pop();
            }
            let match_arm = match_arm.parse::<Tstream>().unwrap();

            quote! {
                #match_arm => {
                    if reverse {
                        items.sort_by(|lhs, rhs| {
                            Self::DataType::#fn_name(&lhs, &rhs)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .reverse()
                        })
                    } else {
                        items.sort_by(|lhs, rhs| {
                            Self::DataType::#fn_name(&lhs, &rhs)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                    }

                }
            }
        });

    quote! {
        #(#per_field_fns)*
    }
}

fn get_dfill_filters(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_fns = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().tag.is_some())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let mut match_arm = a.field.as_ref().unwrap().tag.clone().unwrap();
            let aggr = a.field.as_ref().unwrap().aggr.is_some();
            let fn_name = format!("get_{}_str", name).parse::<Tstream>().unwrap();
            if match_arm.ends_with('&') {
                match_arm.pop();
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                if aggr {
                    quote! {
                        #match_arm => re.is_match(&Self::DataType::#fn_name(model)),
                    }
                } else {
                    quote! {
                        #match_arm => re.is_match(&self.data.#fn_name(model)),
                    }
                }
            } else {
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                quote! {
                    #match_arm => re.is_match(&self.data.#fn_name()),
                }
            }
        });

    quote! {
        #(#per_field_fns)*
    }
}

pub fn gen_dfill_tag_and_class_fns(fields: &syn::FieldsNamed, struct_name: &syn::Ident) -> Tstream {
    let struct_name = struct_name.to_string();
    let struct_name = struct_name[..struct_name.len() - 4]
        .parse::<Tstream>()
        .unwrap();
    let field_type = fields
        .named
        .iter()
        .map(|f| parse_attribute(&f.attrs, &f.ident.clone().unwrap()))
        .filter(|a| a.field.is_some() && a.field.as_ref().unwrap().tag.is_some())
        .find_map(|a| {
            let tag = a.field.unwrap().tag.unwrap();
            let tag = tag.split("::").collect::<Vec<&str>>();
            Some(tag[0].to_string())
        });

    if field_type.is_none() {
        return quote! {};
    }
    let title_fns = "title_fns".parse::<Tstream>().unwrap();
    let field_fns = "field_fns".parse::<Tstream>().unwrap();
    let tag_title_fns = get_dfill_field_fns(fields, "title", &title_fns, true);
    let tag_title_fns_styled = get_dfill_field_fns(fields, "title_styled", &title_fns, true);
    let tag_field_fns = get_dfill_field_fns(fields, "str", &field_fns, false);
    let tag_field_fns_styled = get_dfill_field_fns(fields, "str_styled", &field_fns, false);

    let class_title_fns = get_dfill_class_fns(fields, "title", &title_fns, true);
    let class_title_fns_styled = get_dfill_class_fns(fields, "title_styled", &title_fns, true);
    let class_field_fns = get_dfill_class_fns(fields, "str", &field_fns, false);
    let class_field_fns_styled = get_dfill_class_fns(fields, "str_styled", &field_fns, false);
    let cmp_fns = get_dfill_cmp_fns(fields);
    let filters = get_dfill_filters(fields);

    let field_type = field_type.unwrap().parse::<Tstream>().unwrap();

    quote! {
        impl Dfill for #struct_name {
            fn build_title_fns(&mut self, opts: &Option<Vec<#field_type>>) {
                opts.as_ref().and_then(|opt| {
                    opt.iter().for_each(|opt| match opt {
                        #tag_title_fns
                        #class_title_fns
                    });
                    Some(())
                });
            }

            fn build_title_fns_styled(&mut self, opts: &Option<Vec<#field_type>>) {
                opts.as_ref().and_then(|opt| {
                    opt.iter().for_each(|opt| match opt {
                        #tag_title_fns_styled
                        #class_title_fns_styled
                    });
                    Some(())
                });
            }

            fn build_field_fns(&mut self, opts: &Option<Vec<#field_type>>) {
                opts.as_ref().and_then(|opt| {
                    opt.iter().for_each(|opt| match opt {
                        #tag_field_fns
                        #class_field_fns
                    });
                    Some(())
                });
            }

            fn build_field_fns_styled(&mut self, opts: &Option<Vec<#field_type>>) {
                opts.as_ref().and_then(|opt| {
                    opt.iter().for_each(|opt| match opt {
                        #tag_field_fns_styled
                        #class_field_fns_styled
                    });
                    Some(())
                });
            }

            fn sort_by(items: &mut Vec<&Self::Model>, tag: &Self::FieldsType, reverse: bool) {
                match tag {
                    #cmp_fns
                    _ => {}
                }
            }

            fn filter_by(&self, model: &Self::Model, tag: &Self::FieldsType, re: &Regex) -> bool {
                match tag {
                    #filters
                    _ => true
                }
            }
        }
    }
}
