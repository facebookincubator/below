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

use model::SingleDiskModelFieldId;

use super::*;

impl HasRenderConfigForDump for model::SingleDiskModel {}

pub struct Disk {
    opts: GeneralOpt,
    select: Option<SingleDiskModelFieldId>,
    fields: Vec<DiskField>,
}

impl Disk {
    pub fn new(
        opts: &GeneralOpt,
        select: Option<SingleDiskModelFieldId>,
        fields: Vec<DiskField>,
    ) -> Self {
        Self {
            opts: opts.to_owned(),
            select,
            fields,
        }
    }
}

impl Dumper for Disk {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        let mut disks: Vec<_> = model
            .system
            .disks
            .iter()
            .filter_map(
                |(_, model)| match (self.select.as_ref(), self.opts.filter.as_ref()) {
                    (Some(field_id), Some(filter))
                        if !filter.is_match(
                            &model
                                .query(&field_id)
                                .map_or("?".to_owned(), |v| v.to_string()),
                        ) =>
                    {
                        None
                    }
                    _ => Some(model),
                },
            )
            .collect();

        if let Some(field_id) = &self.select {
            if self.opts.sort {
                model::sort_queriables(&mut disks, &field_id, false);
            }

            if self.opts.rsort {
                model::sort_queriables(&mut disks, &field_id, true);
            }

            if (self.opts.sort || self.opts.rsort) && self.opts.top != 0 {
                disks.truncate(self.opts.top as usize);
            }
        }
        let json = self.opts.output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        disks
            .into_iter()
            .map(|model| {
                match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => write!(
                        output,
                        "{}",
                        print::dump_raw_indented(
                            &self.fields,
                            ctx,
                            model,
                            *round,
                            self.opts.repeat_title,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::Csv) => write!(
                        output,
                        "{}",
                        print::dump_csv(
                            &self.fields,
                            ctx,
                            model,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::Tsv) => write!(
                        output,
                        "{}",
                        print::dump_tsv(
                            &self.fields,
                            ctx,
                            model,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&self.fields, ctx, model, self.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        let par = print::dump_json(&self.fields, ctx, model, self.opts.raw);
                        json_output.as_array_mut().unwrap().push(par);
                    }
                }
                *round += 1;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        match (json, comma_flag) {
            (true, true) => write!(output, ",{}", json_output)?,
            (true, false) => write!(output, "{}", json_output)?,
            _ => write!(output, "\n")?,
        };

        Ok(IterExecResult::Success)
    }
}
