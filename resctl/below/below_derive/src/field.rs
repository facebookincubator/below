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

use std::cell::RefCell;

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
    pub field_attr: BelowFieldAttr,
    // Unwrap the view attr into Field, one less if layer during generation
    pub view_attr: BelowViewAttr,
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
    pub highlight_if_value: Option<Tstream>,
    pub prefix: Tstream,
    pub depth: Tstream,
    pub width: Tstream,
    pub unit: String,
    // Parse dfill related boxed fn handle. More in `parse_dfill_tag`
    pub dfill_tag_title: Option<Tstream>,
    pub dfill_tag_title_styled: Option<Tstream>,
    pub dfill_tag_field: Option<Tstream>,
    pub dfill_tag_field_styled: Option<Tstream>,
    pub raw: RefCell<Tstream>,
}

impl Field {
    /// Generate new field from the attributes
    pub fn new_with_attr(name: syn::Ident, field_type: syn::Type, attr: attr::BelowAttr) -> Field {
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
            highlight_if_value: Self::parse_highlight_if(&view_attr, &inner_type, &field_type),
            dfill_tag_title: None,
            dfill_tag_title_styled: None,
            dfill_tag_field: None,
            dfill_tag_field_styled: None,
            raw: RefCell::new(quote! {false}),
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
        field_attr: &attr::BelowFieldAttr,
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
        view_attr: &attr::BelowViewAttr,
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

    fn parse_highlight_if(
        view_attr: &attr::BelowViewAttr,
        inner_type: &Option<syn::Type>,
        field_type: &syn::Type,
    ) -> Option<Tstream> {
        if let Some(hval) = view_attr.highlight_if.as_ref() {
            let highlight_if_value = hval.replace("$", "v").parse::<Tstream>().unwrap();
            let value_type = inner_type.as_ref().unwrap_or(&field_type);

            Some(quote! {
                (|v: #value_type| {
                    #highlight_if_value
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
