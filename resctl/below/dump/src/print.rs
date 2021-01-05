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

use super::*;

use model::{Field, FieldId, Queriable, Recursive};
use render::{HasRenderConfig, RenderConfig};

// Dprint trait defines the print fns for different format. Please refer to
// the comment of Dump trait in tmain.rs for the whole picture.
pub trait Dprint
where
    Self: DumpType + Dget,
{
    fn print_title_line<T: Write>(
        &self,
        model: &Self::Model,
        out: &mut T,
        sep: &str,
    ) -> Result<()> {
        let mut line = String::new();
        self.get_title_fns().iter().for_each(|item| {
            line.push_str(&format!("{}{}", item(self.get_data(), &model), sep));
        });

        write!(out, "{}\n", line)?;
        Ok(())
    }

    fn print_field_line<T: Write>(
        &self,
        model: &Self::Model,
        out: &mut T,
        sep: &str,
    ) -> Result<()> {
        let mut line = String::new();
        self.get_field_fns().iter().for_each(|item| {
            line.push_str(&format!("{}{}", item(self.get_data(), &model), sep));
        });

        write!(out, "{}\n", line)?;
        Ok(())
    }

    fn do_print_kv<T: Write>(&self, model: &Self::Model, out: &mut T) -> Result<()> {
        let mut paragraph = String::new();
        self.get_title_fns()
            .iter()
            .zip(self.get_field_fns().iter())
            .for_each(|(title, field)| {
                paragraph.push_str(&format!(
                    "{}: {}\n",
                    title(self.get_data(), &model),
                    field(self.get_data(), &model),
                ));
            });

        write!(out, "{}\n", paragraph)?;
        Ok(())
    }

    fn do_print_json(&self, model: &Self::Model) -> Value {
        let mut res = json!({});
        self.get_title_fns()
            .iter()
            .zip(self.get_field_fns().iter())
            .for_each(|(title, field)| {
                res[title(self.get_data(), &model)] = json!(field(self.get_data(), &model))
            });
        res
    }

    fn do_print_raw<T: Write>(
        &self,
        model: &Self::Model,
        output: &mut T,
        round: usize,
    ) -> Result<()> {
        let repeat = self.get_opts().repeat_title.unwrap_or(0);
        let disable_title = self.get_opts().disable_title;
        if !disable_title && (round == 0 || (repeat != 0 && round % repeat == 0)) {
            self.print_title_line(&model, output, " ")?;
        }
        self.print_field_line(&model, output, " ")
    }

    fn do_print_csv<T: Write>(
        &self,
        model: &Self::Model,
        output: &mut T,
        round: usize,
    ) -> Result<()> {
        let disable_title = self.get_opts().disable_title;
        if !disable_title && round == 0 {
            self.print_title_line(&model, output, ",")?;
        }
        self.print_field_line(&model, output, ",")
    }
}

impl CommonField {
    /// Default RenderConfig for CommonField
    pub fn get_render_config(&self) -> RenderConfig {
        use render::render_config as rc;
        match self {
            Self::Timestamp => rc!(title("Timestamp"), width(10)),
            Self::Datetime => rc!(title("Datetime"), width(19)),
        }
    }
}

pub trait HasRenderConfigForDump: HasRenderConfig {
    fn get_render_config_for_dump(field_id: &Self::FieldId) -> RenderConfig {
        Self::get_render_config(field_id)
    }
}

impl<F> DumpField<F>
where
    F: FieldId,
    F::Queriable: HasRenderConfigForDump,
{
    pub fn get_render_config(&self) -> RenderConfig {
        match self {
            Self::Common(common) => common.get_render_config(),
            Self::FieldId(field_id) => F::Queriable::get_render_config_for_dump(field_id),
        }
    }

    pub fn get_field(&self, ctx: &CommonFieldContext, model: &F::Queriable) -> Option<Field> {
        match self {
            Self::Common(common) => common.get_field(ctx),
            Self::FieldId(field_id) => model.query(field_id),
        }
    }

    pub fn dump_field(
        &self,
        ctx: &CommonFieldContext,
        model: &F::Queriable,
        raw: bool,
        fixed_width: bool,
    ) -> String {
        let mut config = self.get_render_config();
        if raw {
            config.format = None;
            config.suffix = None;
        }
        config.render(self.get_field(ctx, model), fixed_width)
    }
}

impl<F> DumpField<F>
where
    F: FieldId,
    F::Queriable: HasRenderConfigForDump + Recursive,
{
    pub fn dump_field_indented(
        &self,
        ctx: &CommonFieldContext,
        model: &F::Queriable,
        raw: bool,
        fixed_width: bool,
    ) -> String {
        let mut config = self.get_render_config();
        if raw {
            config.format = None;
            config.suffix = None;
        }
        config.render_indented(self.get_field(ctx, model), fixed_width, model.get_depth())
    }
}

pub fn dump_kv<T: HasRenderConfigForDump>(
    fields: &[DumpField<T::FieldId>],
    ctx: &CommonFieldContext,
    model: &T,
    raw: bool,
) -> String {
    let mut res = String::new();
    for field in fields {
        let config = field.get_render_config();
        res.push_str(&format!(
            "{}: {}\n",
            config.render_title(false),
            field.dump_field(ctx, model, raw, false),
        ));
    }
    res.push('\n');
    res
}

pub fn dump_json<T: HasRenderConfigForDump>(
    fields: &[DumpField<T::FieldId>],
    ctx: &CommonFieldContext,
    model: &T,
    raw: bool,
) -> Value {
    let mut res = json!({});
    for field in fields {
        let config = field.get_render_config();
        res[config.render_title(false)] = json!(field.dump_field(ctx, model, raw, false));
    }
    res
}

fn dump_title_line<F>(fields: &[DumpField<F>], sep: &'static str, fixed_width: bool) -> String
where
    F: FieldId,
    F::Queriable: HasRenderConfigForDump,
{
    let mut line = String::new();
    for field in fields {
        line.push_str(&field.get_render_config().render_title(fixed_width));
        line.push_str(sep);
    }
    line.push('\n');
    line
}

pub fn dump_raw<T: HasRenderConfigForDump>(
    fields: &[DumpField<T::FieldId>],
    ctx: &CommonFieldContext,
    model: &T,
    round: usize,
    repeat_title: Option<usize>,
    disable_title: bool,
    raw: bool,
) -> String {
    let mut res = String::new();
    let repeat = repeat_title.unwrap_or(0);
    if !disable_title && (round == 0 || (repeat != 0 && round % repeat == 0)) {
        res.push_str(&dump_title_line(fields, " ", true));
    }
    for field in fields {
        res.push_str(&field.dump_field(ctx, model, raw, true));
        res.push(' ');
    }
    res.push('\n');
    res
}

pub fn dump_raw_indented<T: HasRenderConfigForDump + Recursive>(
    fields: &[DumpField<T::FieldId>],
    ctx: &CommonFieldContext,
    model: &T,
    round: usize,
    repeat_title: Option<usize>,
    disable_title: bool,
    raw: bool,
) -> String {
    let mut res = String::new();
    let repeat = repeat_title.unwrap_or(0);
    if !disable_title && (round == 0 || (repeat != 0 && round % repeat == 0)) {
        res.push_str(&dump_title_line(fields, " ", true));
    }
    for field in fields {
        res.push_str(&field.dump_field_indented(ctx, model, raw, true));
        res.push(' ');
    }
    res.push('\n');
    res
}

pub fn dump_csv<T: HasRenderConfigForDump>(
    fields: &[DumpField<T::FieldId>],
    ctx: &CommonFieldContext,
    model: &T,
    round: usize,
    disable_title: bool,
    raw: bool,
) -> String {
    let mut res = String::new();
    if !disable_title && round == 0 {
        res.push_str(&dump_title_line(fields, ",", false));
    }
    for field in fields {
        res.push_str(&field.dump_field(ctx, model, raw, false));
        res.push(',');
    }
    res.push('\n');
    res
}
