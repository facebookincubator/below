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
    // sort_tag_type: Option<Tstream>,
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
            // sort_tag_type: None,
            // class_fns: BTreeMap::new(),
            // dfill_struct: None,
        };

        model.blink_type = model
            .fields
            .iter()
            .find_map(|f| f.blink_type.as_ref().map(|v| v.parse::<Tstream>().unwrap()));

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
}
