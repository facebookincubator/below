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

pub enum DFormat {
    Title(&'static str, &'static str),
    Field(&'static str, &'static str),
    CSVTitle(&'static str, &'static str),
    CSVField(&'static str, &'static str),
}

/// A convenience function for a single field string generation.
/// This function will generate all styled and unstyled field string. It
/// will apply all the decorator, attribute, etc for styled flavor. For
/// non styled flavor, it's nothing but a call to the display trait.
fn generate_from_attr(f: &syn::Field, a: &BelowAttr, link: bool) -> Tstream {
    let aggr_type = if let Some(aggr) = a.field.as_ref().unwrap().aggr.clone() {
        let aggr_vec: Vec<&str> = aggr.split(':').collect();
        if aggr_vec.len() != 2 {
            unimplemented!("Expect \"Type: a + b\" format for aggregator");
        }
        Some(aggr_vec[0].to_string().parse::<Tstream>().unwrap())
    } else {
        None
    };
    let name = if link {
        format!("self.get_{}(model)", &f.ident.clone().unwrap())
            .parse::<Tstream>()
            .unwrap()
    } else if aggr_type.is_some() {
        format!("Self::get_{}(input)", &f.ident.clone().unwrap())
            .parse::<Tstream>()
            .unwrap()
    } else {
        format!("self.{}", &f.ident.clone().unwrap())
            .parse::<Tstream>()
            .unwrap()
    };

    let fn_name = format!("get_{}_str_styled", &f.ident.clone().unwrap())
        .parse::<Tstream>()
        .unwrap();
    let fn_name_raw = format!("get_{}_str", &f.ident.clone().unwrap())
        .parse::<Tstream>()
        .unwrap();
    let view = a.view.as_ref().unwrap();
    let prefix = view
        .prefix
        .clone()
        .unwrap_or_else(|| "\"\"".parse::<Tstream>().unwrap());
    let depth = view
        .depth
        .clone()
        .unwrap_or_else(|| "0".parse::<Tstream>().unwrap());
    let width = view.width.clone().unwrap_or(0);
    let unit = view.unit.clone().unwrap_or_else(|| "".into());
    let decored_val = if let Some(decor) = view.decorator.as_ref() {
        decor.replace("$", "v").parse::<Tstream>().unwrap()
    } else {
        quote! {v}
    };

    let mut pattern = "{".to_string();
    if let Some(p) = view.precision.clone() {
        pattern.push_str(format!(":.{}", p).as_str());
    }
    pattern.push_str("}{}");
    let value = match f.ty {
        syn::Type::Path(ref ty_path) if ty_path.path.segments[0].ident == "Option" => {
            let none_mark = &view.none_mark;
            quote! {
                match #name.clone() {
                    Some(v) => format!(#pattern, &#decored_val, #unit),
                    _ => format!("{}", &#none_mark),
                }
            }
        }
        _ => {
            quote! {
                {
                    let v = &#name;
                    format!(#pattern, &#decored_val, #unit)
                }
            }
        }
    };

    if link {
        let link = a.field.as_ref().unwrap().link.as_ref().unwrap();
        let link = link.split('$').collect::<Vec<&str>>();
        let m_type = if link.len() == 2 {
            link[0].parse::<Tstream>().unwrap()
        } else {
            unimplemented!("Expecting a model for link");
        };

        quote! {
            pub fn #fn_name(&self, model: &#m_type) -> String {
                let value = #value;
                let prefix = #prefix;
                let depth = #depth;
                let unit = #unit;
                let width = if #width == 0 {
                    value.len()
                } else if #width >=  depth {
                    #width - depth
                } else {
                    0
                };

                format!(
                    "{:>depth$.depth$}{:width$.width$}",
                    prefix,
                    value,
                    depth = depth,
                    width = width
                )
            }

            pub fn #fn_name_raw(&self, model: &#m_type) -> String {
                #value
            }
        }
    } else {
        let args = if let Some(at) = aggr_type {
            quote! {input: &#at}
        } else {
            quote! {&self}
        };

        quote! {
            pub fn #fn_name(#args) -> String {
                let value = #value;
                let prefix = #prefix;
                let depth = #depth;
                let unit = #unit;
                let width = if #width == 0 {
                    value.len()
                } else if #width >=  depth {
                    #width - depth
                } else {
                    0
                };

                format!(
                    "{:>depth$.depth$}{:width$.width$}",
                    prefix,
                    value,
                    depth = depth,
                    width = width
                )
            }

            pub fn #fn_name_raw(#args) -> String {
                #value
            }
        }
    }
}

/// Generate get_{}_str function for each direct field.
pub fn gen_get_str_per_dir_field(fields: &syn::FieldsNamed) -> Tstream {
    let per_field_str = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_none() || a.field.as_ref().unwrap().link.is_none())
        .filter(|(_, a)| a.view.is_some())
        .map(|(f, a)| generate_from_attr(f, &a, false));

    quote! {
        #(#per_field_str)*
    }
}

/// Generate get_{}_str function for each linked field.
pub fn gen_get_str_per_link_field(fields: &syn::FieldsNamed) -> Tstream {
    let direct_field = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().link.is_some())
        .filter(|(_, a)| a.view.is_some())
        .map(|(f, a)| generate_from_attr(f, &a, true));

    let follow_field = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && a.field.as_ref().unwrap().link.is_some())
        .filter(|(_, a)| a.view.is_none())
        .map(|(f, a)| {
            let name = &f.ident.clone().unwrap();
            let link = a.field.unwrap().link.unwrap();
            let link = link.replace("?", ".as_ref().unwrap_or(&Default::default())");
            let link = link.split('$').collect::<Vec<&str>>();

            if link.len() == 2 {
                let m_type = link[0].parse::<Tstream>().unwrap();
                let (link, link_raw) = if link[1].ends_with('&') {
                    let mut link = link[1].to_string();
                    link.pop();
                    let target = format!("{}_str_styled", link).parse::<Tstream>().unwrap();
                    let target_raw = format!("{}_str", link).parse::<Tstream>().unwrap();
                    (
                        quote! {
                            model.#target(model)
                        },
                        quote! {
                            model.#target_raw(model)
                        },
                    )
                } else {
                    let target = format!("{}_str_styled", link[1])
                        .parse::<Tstream>()
                        .unwrap();
                    let target_raw = format!("{}_str", link[1]).parse::<Tstream>().unwrap();
                    (
                        quote! {
                            model.#target()
                        },
                        quote! {
                            model.#target_raw()
                        },
                    )
                };
                let fn_name = Ident::new(&format!("get_{}_str_styled", name), Span::call_site());
                let fn_name_raw = Ident::new(&format!("get_{}_str", name), Span::call_site());

                quote! {
                    pub fn #fn_name(&self, model: &#m_type) -> String {
                        &self.#name;
                        #link
                    }

                    pub fn #fn_name_raw(&self, model: &#m_type) -> String {
                        &self.#name;
                        #link_raw
                    }
                }
            } else {
                unimplemented!("Expecting a model for link");
            }
        });

    quote! {
        #(#direct_field)*
        #(#follow_field)*
    }
}

fn unified_line_generation(fields: &syn::FieldsNamed, dformat: DFormat) -> Tstream {
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

    let aggr_type = fields
        .named
        .iter()
        .map(|f| parse_attribute(&f.attrs, &f.ident.clone().unwrap()))
        .filter(|a| a.field.is_some() && a.field.as_ref().unwrap().aggr.is_some())
        .find_map(|a| {
            let aggr = a.field.as_ref().unwrap().aggr.clone().unwrap();
            let aggr_vec: Vec<&str> = aggr.split(':').collect();
            if aggr_vec.len() != 2 {
                unimplemented!("Expect \"Type: a + b\" format for aggregator");
            }
            Some(aggr_vec[0].to_string())
        });

    let (fn_name, ext, sep, is_title, styled_string) = match dformat {
        DFormat::Title(e, "|") => (
            "get_title_pipe".parse::<Tstream>().unwrap(),
            e,
            "|",
            true,
            false,
        ),
        DFormat::Title(e, s) => (
            "get_title_line".parse::<Tstream>().unwrap(),
            e,
            s,
            true,
            false,
        ),
        DFormat::Field(e, s) => (
            "get_field_line".parse::<Tstream>().unwrap(),
            e,
            s,
            false,
            true,
        ),
        DFormat::CSVTitle(e, s) => (
            "get_csv_title".parse::<Tstream>().unwrap(),
            e,
            s,
            true,
            false,
        ),
        DFormat::CSVField(e, s) => (
            "get_csv_field".parse::<Tstream>().unwrap(),
            e,
            s,
            false,
            false,
        ),
    };

    let rep = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && !a.field.as_ref().unwrap().no_show)
        .map(|(f, a)| {
            let field_attr = a.field.as_ref().unwrap();
            let name = &f.ident.clone().unwrap();
            let fn_name = format!("get_{}_{}", &name, ext).parse::<Tstream>().unwrap();
            if field_attr.link.is_some() && (!is_title || field_attr.title.is_none()) {
                if styled_string {
                    quote! {
                        res.append_plain(&self.#fn_name(model));
                        res.append_plain(#sep);
                    }
                } else {
                    quote! {
                        res.push_str(&self.#fn_name(model));
                        res.push_str(#sep);
                    }
                }
            } else if field_attr.aggr.is_some() {
                let args = if ext.starts_with("title") {
                    quote! {}
                } else {
                    quote! {input}
                };
                if styled_string {
                    quote! {
                        res.append_plain(&Self::#fn_name(#args));
                        res.append_plain(#sep);
                    }
                } else {
                    quote! {
                        res.push_str(&Self::#fn_name(#args));
                        res.push_str(#sep);
                    }
                }
            } else {
                if styled_string {
                    quote! {
                        res.append_plain(&self.#fn_name());
                        res.append_plain(#sep);
                    }
                } else {
                    quote! {
                        res.push_str(&self.#fn_name());
                        res.push_str(#sep);
                    }
                }
            }
        });

    let mut args = "&self".to_string();

    if let Some(model_type) = model_type {
        args.push_str(&format!(", model: &{}", model_type));
    }

    if let Some(aggr_type) = aggr_type {
        if !ext.starts_with("title") {
            args.push_str(&format!(", input: &{}", aggr_type));
        }
    }

    let args = args.parse::<Tstream>().unwrap();

    if styled_string {
        quote! {
            pub fn #fn_name(#args) -> StyledString {
                let mut res = StyledString::new();
                #(#rep)*
                res
            }
        }
    } else {
        quote! {
            pub fn #fn_name(#args) -> String {
                let mut res = String::new();
                #(#rep)*
                res
            }
        }
    }
}

pub fn gen_title_line(fields: &syn::FieldsNamed) -> Tstream {
    unified_line_generation(fields, DFormat::Title("title_styled", " "))
}

pub fn gen_field_line(fields: &syn::FieldsNamed) -> Tstream {
    unified_line_generation(fields, DFormat::Field("str_styled", " "))
}

pub fn gen_csv_title(fields: &syn::FieldsNamed) -> Tstream {
    unified_line_generation(fields, DFormat::CSVTitle("title", ","))
}

pub fn gen_csv_field(fields: &syn::FieldsNamed) -> Tstream {
    unified_line_generation(fields, DFormat::CSVField("str", ","))
}

// This function will help us easier parse title line into the TabView vector by split with '|'
pub fn gen_title_pipe(fields: &syn::FieldsNamed) -> Tstream {
    unified_line_generation(fields, DFormat::Title("title_styled", "|"))
}

pub fn gen_interleave(fields: &syn::FieldsNamed) -> Tstream {
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

    let aggr_type = fields
        .named
        .iter()
        .map(|f| parse_attribute(&f.attrs, &f.ident.clone().unwrap()))
        .filter(|a| a.field.is_some() && a.field.as_ref().unwrap().aggr.is_some())
        .find_map(|a| {
            let aggr = a.field.as_ref().unwrap().aggr.clone().unwrap();
            let aggr_vec: Vec<&str> = aggr.split(':').collect();
            if aggr_vec.len() != 2 {
                unimplemented!("Expect \"Type: a + b\" format for aggregator");
            }
            Some(aggr_vec[0].to_string())
        });

    let fn_name = "get_interleave_line".parse::<Tstream>().unwrap();

    let rep = iter_field_attr!(fields)
        .filter(|(_, a)| a.field.is_some() && !a.field.as_ref().unwrap().no_show)
        .map(|(f, a)| {
            let field_attr = a.field.as_ref().unwrap();
            let name = &f.ident.clone().unwrap();
            let title_name = format!("get_{}_title_styled", &name)
                .parse::<Tstream>()
                .unwrap();
            let value_name = format!("get_{}_str_styled", &name)
                .parse::<Tstream>()
                .unwrap();
            if field_attr.link.is_some() {
                if field_attr.title.is_none() {
                    quote! {
                        let mut line = StyledString::new();
                        line.append_plain(self.#title_name(model));
                        line.append_plain(sep);
                        line.append_plain(self.#value_name(model));
                        res.push(line);
                    }
                } else {
                    quote! {
                        let mut line = StyledString::new();
                        line.append_plain(self.#title_name());
                        line.append_plain(sep);
                        line.append_plain(self.#value_name(model));
                        res.push(line);
                    }
                }
            } else if field_attr.aggr.is_some() {
                quote! {
                    let mut line = StyledString::new();
                    line.append_plain(Self::#title_name());
                    line.append_plain(sep);
                    line.append_plain(Self::#value_name(input));
                    res.push(line);
                }
            } else {
                quote! {
                    let mut line = StyledString::new();
                    line.append_plain(self.#title_name());
                    line.append_plain(sep);
                    line.append_plain(self.#value_name());
                    res.push(line);
                }
            }
        });

    let mut args = "&self, sep: &str".to_string();
    if let Some(model_type) = model_type {
        args.push_str(&format!(", model: &{}", model_type));
    }
    if let Some(aggr_type) = aggr_type {
        args.push_str(&format!(", input: &{}", aggr_type));
    }
    let args = args.parse::<Tstream>().unwrap();

    quote! {
        pub fn #fn_name(#args) -> Vec<StyledString> {
            let mut res: Vec<StyledString> = Vec::new();
            #(#rep)*
            res
        }
    }
}
