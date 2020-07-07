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
    let per_field_fns = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().tag.is_some())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let mut match_arm = a.field.as_ref().unwrap().tag.clone().unwrap();
            let aggr = a.field.as_ref().unwrap().aggr.is_some();
            let fn_name = format!("get_{}_{}", name, suffix)
                .parse::<Tstream>()
                .unwrap();
            if match_arm.ends_with('&') {
                match_arm.pop();
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                if title && !aggr {
                    quote! {
                        #match_arm => self.#fn_type.push(Box::new(|data, _| data.#fn_name())),
                    }
                } else if aggr && title {
                    quote! {
                        #match_arm => self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name())),
                    }
                } else if aggr {
                    quote! {
                        #match_arm => self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name(model))),
                    }
                } else {
                    quote! {
                        #match_arm => self.#fn_type.push(Box::new(|data, model| data.#fn_name(model))),
                    }
                }
            } else {
                let match_arm = match_arm.parse::<Tstream>().unwrap();
                quote! {
                    #match_arm => self.#fn_type.push(Box::new(|data, _| data.#fn_name())),
                }
            }
        });

    quote! {
        #(#per_field_fns)*
    }
}

fn get_dfill_class_field(field: &str, suffix: &str, fn_type: &Tstream, title: bool) -> Tstream {
    let aggr = field.ends_with('@');
    let link = field.ends_with('&');
    if aggr || link {
        let mut field = field.to_string();
        field.pop();
        let fn_name = format!("get_{}_{}", field, suffix)
            .parse::<Tstream>()
            .unwrap();
        if title && !aggr {
            quote! {
                self.#fn_type.push(Box::new(|data, model| data.#fn_name()));
            }
        } else if aggr && title {
            quote! {
                self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name()));
            }
        } else if aggr {
            quote! {
                self.#fn_type.push(Box::new(|_, model| Self::DataType::#fn_name(model)));
            }
        } else {
            quote! {
                self.#fn_type.push(Box::new(|data, model| data.#fn_name(model)));
            }
        }
    } else {
        let fn_name = format!("get_{}_{}", field, suffix)
            .parse::<Tstream>()
            .unwrap();
        quote! {
            self.#fn_type.push(Box::new(|data, _| data.#fn_name()));
        }
    }
}

fn get_dfill_class_fns(
    fields: &syn::FieldsNamed,
    suffix: &str,
    fn_type: &Tstream,
    title: bool,
) -> Tstream {
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

            let fields_fns_reg = fields_reg
                .iter()
                .map(|field| get_dfill_class_field(field, suffix, fn_type, title));

            if fields.len() > 1 {
                let fields_detail = fields[1].split(',').collect::<Vec<&str>>();
                let fields_fns_detail = fields_detail
                    .iter()
                    .map(|field| get_dfill_class_field(field, suffix, fn_type, title));

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
