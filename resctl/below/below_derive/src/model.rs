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
use attr_new::*;
use field::Field;
use function::Function;
// use std::collections::BTreeMap;
use std::rc::Rc;

/// A model is a collection of struct fields with parsed attributes
pub struct Model {
    fields: Vec<Rc<Field>>,
    // Denormalization: if any of the fields is a blink, we will have the model type
    // here since all fields should have the same model type.
    blink_type: Option<Tstream>,
    sort_tag_type: Option<Tstream>,
    // class_fns: BTreeMap<String, ClassField>,
    // dfill_struct: Option<Tstream>,
}

enum DfillType {
    Title,
    TitleStyled,
    Field,
    FieldStyled,
}

impl Model {
    /// Generate new model from the fields
    pub fn new_with_members(fields: &syn::FieldsNamed) -> Model {
        let mut model = Model {
            fields: fields
                .named
                .iter()
                .filter_map(|f| {
                    let name = f.ident.clone().expect("Failed unwrap field name.");
                    let attr = parse_attribute(&f.attrs, &name);
                    if attr.field.is_some() || attr.view.is_some() {
                        Some(Rc::new(Field::new_with_attr(name, f.ty.clone(), attr)))
                    } else {
                        None
                    }
                })
                .collect(),
            blink_type: None,
            sort_tag_type: None,
            // class_fns: BTreeMap::new(),
            // dfill_struct: None,
        };

        model.blink_type = model
            .fields
            .iter()
            .find_map(|f| f.blink_type.as_ref().map(|v| v.parse::<Tstream>().unwrap()));

        model.sort_tag_type = model.fields.iter().find_map(|f| f.sort_tag_type.clone());

        model
    }

    /// Generate get fns for each field
    /// More details in Function::gen_get_fn
    pub fn generate_get_fns(&self) -> Tstream {
        let get_fns = self.fields.iter().map(|f| Function::gen_get_fn(&f));
        quote! {
            #(#get_fns)*
        }
    }

    /// Generate get title fns for each field
    /// More details in Function::gen_get_title_fn
    pub fn generate_get_title_fns(&self) -> Tstream {
        let get_title_fns = self.fields.iter().map(|f| Function::gen_get_title_fn(&f));
        quote! {
            #(#get_title_fns)*
        }
    }

    /// Generate get_FIELD_str_impl function for all fields
    pub fn generate_get_str_impl_fns(&self) -> Tstream {
        let get_str_impl_fns = self
            .fields
            .iter()
            .filter(|f| !f.is_blink())
            .map(|f| Function::gen_get_str_impl(&f));
        quote! {
            #(#get_str_impl_fns)*
        }
    }

    /// Generate get_FIELD_str and get_FIELD_str_impl function for all fields
    pub fn generate_get_str_fns(&self) -> Tstream {
        let get_str_fns = self.fields.iter().map(|f| Function::gen_get_str(&f));
        quote! {
            #(#get_str_fns)*
        }
    }

    /// Unified code generation utility
    /// It will generate code like:
    /// ```ignore
    /// fn get_title_line(model: &TestModel) -> String {
    ///    let mut res = String::new();
    ///    res.push_str(&Self::get_usage_pct_title_styled());
    ///    res.push_str(" ");
    ///    res.push_str(&Self::get_mem_high_title_styled(model));
    ////   res.push_str(" ");
    ///    res
    /// }
    /// ```
    fn unified_line_generator_plain<P>(
        &self,
        fn_name: Tstream,
        sep: Tstream,
        sub_fn: &str,
        predicate: P,
    ) -> Tstream
    where
        P: Fn(&Field) -> bool,
    {
        let fields = self.fields.iter().filter(|f| predicate(f)).map(|f| {
            let get_fn = f.build_self_caller(sub_fn);
            quote! {
                res.push_str(&#get_fn);
                res.push_str(#sep);
            }
        });

        let args = match &self.blink_type {
            Some(blink_type) => quote! {&self, model: &#blink_type},
            _ => quote! {&self},
        };

        quote! {
            pub fn #fn_name(#args) -> String {
                let mut res = String::new();
                #(#fields)*
                res
            }
        }
    }

    fn unified_line_generator_styled<P>(
        &self,
        fn_name: Tstream,
        sep: Tstream,
        sub_fn: &str,
        predicate: P,
    ) -> Tstream
    where
        P: Fn(&Field) -> bool,
    {
        let fields = self.fields.iter().filter(|f| predicate(f)).map(|f| {
            let get_fn = f.build_self_caller(sub_fn);
            quote! {
                res.append(#get_fn);
                res.append_plain(#sep);
            }
        });

        let args = match &self.blink_type {
            Some(blink_type) => quote! {&self, model: &#blink_type},
            _ => quote! {&self},
        };

        quote! {
            pub fn #fn_name(#args) -> StyledString {
                let mut res = StyledString::new();
                #(#fields)*
                res
            }
        }
    }

    /// Generate get_title_line fn
    pub fn generate_get_title_line(&self) -> Tstream {
        self.unified_line_generator_plain(
            quote! {get_title_line},
            quote! {" "},
            "title_styled",
            |f| f.field_attr.title.is_some() || f.is_blink(),
        )
    }

    /// Generate get_field_line fn
    pub fn generate_get_field_line(&self) -> Tstream {
        self.unified_line_generator_styled(
            quote! {get_field_line},
            quote! {" "},
            "str_styled",
            |f| f.field_attr.title.is_some() || f.is_blink(),
        )
    }

    /// Generate get_title_pipe fn
    pub fn generate_get_title_pipe(&self) -> Tstream {
        self.unified_line_generator_plain(
            quote! {get_title_pipe},
            quote! {"|"},
            "title_styled",
            |f| f.field_attr.title.is_some() || f.is_blink(),
        )
    }

    /// Generate get_csv_title fn
    pub fn generate_get_csv_title(&self) -> Tstream {
        self.unified_line_generator_plain(quote! {get_csv_title}, quote! {","}, "title", |f| {
            f.field_attr.title.is_some() || f.is_blink()
        })
    }

    /// Generate get_csv_field fn
    pub fn generate_get_csv_field(&self) -> Tstream {
        self.unified_line_generator_plain(quote! {get_csv_field}, quote! {","}, "str", |f| {
            f.field_attr.title.is_some() || f.is_blink()
        })
    }

    /// Generate get_interleave_line fn
    /// Example:
    /// ```ignore
    /// fn get_interleave_line(&self, sep: &str, line_sep: &str, model: &ModelType) -> Vec<StyledString> {
    ///    let mut res: Vec<StyledString> = Vec::new();
    ///    let mut line = StyledString::new();
    ///    line.append_plain(&self.get_usage_pct_title_styled());
    ///    line.append_plain(sep);
    ///    line.append(&self.get_mem_high_title_styled(model));
    ////   res.push(line);
    ///    res
    /// }
    /// ```
    pub fn generate_interleave(&self) -> Tstream {
        let args = match &self.blink_type {
            Some(blink_type) => quote! {&self, sep: &str, model: &#blink_type},
            _ => quote! {&self, sep: &str},
        };

        let fields = self
            .fields
            .iter()
            .filter(|f| f.field_attr.title.is_some() || f.is_blink())
            .map(|f| {
                let title_fn = f.build_self_caller("title_styled");
                let field_fn = f.build_self_caller("str_styled");
                quote! {
                    let mut line = StyledString::new();
                    line.append_plain(#title_fn);
                    line.append_plain(sep);
                    line.append(#field_fn);
                    res.push(line);
                }
            });

        quote! {
            pub fn get_interleave_line(#args) -> Vec<StyledString> {
                let mut res: Vec<StyledString> = Vec::new();
                #(#fields)*
                res
            }
        }
    }

    /// Generate compare function for each field
    pub fn generate_cmp_fns(&self) -> Tstream {
        let cmp_fns = self
            .fields
            .iter()
            .filter(|f| f.field_attr.cmp)
            .map(|f| Function::gen_cmp_fn(f));
        quote! {
            #(#cmp_fns)*
        }
    }

    /// Generate sorting functions
    /// For all fields that decorated by `cmp`, we will automatically generate a cmp_by_FIELD_NAME function. And for
    /// All fields that decorated by `sort_tag`, we will use the associate cmp_by_FIELD_NAME function and generate a sort
    /// function. The generated code will be something like this:
    /// ```ignore
    /// fn sort(&self, tag: TagType, children: Vec<ModelType>, reverse: bool) {
    ///     match tag {
    ///         TagType::Tag1 => children.sort_by(|lhs, rhs| {
    ///             if reverse {
    ///                 self.cmp_by_FIELD_NAME1(&lhs, &rhs)
    ///                     .unwrap_or(std::cmp::Ordering::Equal)
    ///                     .reverse()
    ///             } else {
    ///                 self.#cmp_by_FIELD_NAME1(&lhs, &rhs)
    ///                     .unwrap_or(std::cmp::Ordering::Equal)
    ///             }
    ///         }),
    ///         ...
    ///         _ => ()
    ///     }
    /// }
    /// ```
    /// All fields without a sorting tagged will automatically tagged by TagType::Keep. So when defining sorting tag,
    /// it's mandatory to have a field named Keep. Please note that all fields that are decorated by `sort_tag` will be automatically
    /// set `cmp = true`
    pub fn generate_sort_fn(&self) -> Tstream {
        // No sort tag
        if self.sort_tag_type.is_none() {
            return quote! {};
        }

        let model_type = self.blink_type.clone().unwrap_or(quote! {Self});

        let sort_tag_type = self.sort_tag_type.as_ref().unwrap();

        let match_arms = self
            .fields
            .iter()
            .filter(|f| f.sort_tag_type.is_some())
            .map(|f| {
                let cmp_fn_name = format!("cmp_by_{}", f.name).parse::<Tstream>().unwrap();
                let sort_tag_val = f.sort_tag_val.as_ref().unwrap();
                quote! {
                    #sort_tag_val => children.sort_by(|lhs, rhs| {
                        if reverse {
                            self.#cmp_fn_name(&lhs, &rhs)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .reverse()
                        } else {
                            self.#cmp_fn_name(&lhs, &rhs)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }
                    })
                }
            });

        quote! {
            pub fn sort(&self, tag: #sort_tag_type, children: &mut Vec<&#model_type>, reverse: bool) {
                match tag {
                    #(#match_arms,)*
                    _ => (),
                };
            }
        }
    }

    /// Generate get_sort_tag_vec and has_tag fn
    pub fn generate_sort_util_fns(&self) -> Tstream {
        // No sort tag
        if self.sort_tag_type.is_none() {
            return quote! {};
        }

        let sort_tag_type = self.sort_tag_type.as_ref().unwrap();
        let sort_tags_vec = self.fields.iter().map(|f| {
            if f.sort_tag_val.is_some() {
                f.sort_tag_val.clone().unwrap()
            } else {
                quote! {
                    #sort_tag_type::Keep
                }
            }
        });

        quote! {
            pub fn get_sort_tag_vec() -> Vec<#sort_tag_type> {
                vec![#(#sort_tags_vec,)*]
            }
        }
    }
}
