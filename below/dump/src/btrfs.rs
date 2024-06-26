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

use model::BtrfsModelFieldId;

use super::*;

pub struct Btrfs {
    opts: GeneralOpt,
    select: Option<BtrfsModelFieldId>,
    fields: Vec<BtrfsField>,
}

impl Btrfs {
    pub fn new(
        opts: &GeneralOpt,
        select: Option<BtrfsModelFieldId>,
        fields: Vec<BtrfsField>,
    ) -> Self {
        Self {
            opts: opts.to_owned(),
            select,
            fields,
        }
    }
}

impl Dumper for Btrfs {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        match model.system.btrfs.as_ref() {
            Some(btrfs_items_ref) => {
                let mut btrfs_items: Vec<_> = btrfs_items_ref
                    .iter()
                    .filter_map(|(_, model)| {
                        match (self.select.as_ref(), self.opts.filter.as_ref()) {
                            (Some(field_id), Some(filter))
                                if !filter.is_match(
                                    &model
                                        .query(field_id)
                                        .map_or("?".to_owned(), |v| v.to_string()),
                                ) =>
                            {
                                None
                            }
                            _ => Some(model),
                        }
                    })
                    .collect();

                if let Some(field_id) = &self.select {
                    if self.opts.sort {
                        model::sort_queriables(&mut btrfs_items, field_id, false);
                    }

                    if self.opts.rsort {
                        model::sort_queriables(&mut btrfs_items, field_id, true);
                    }

                    if (self.opts.sort || self.opts.rsort) && self.opts.top != 0 {
                        btrfs_items.truncate(self.opts.top as usize);
                    }
                }
                let mut json_output = json!([]);

                btrfs_items
                    .into_iter()
                    .map(|model| {
                        match self.opts.output_format {
                            Some(OutputFormat::Raw) | None => write!(
                                output,
                                "{}",
                                print::dump_raw(
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
                            Some(OutputFormat::KeyVal) => write!(
                                output,
                                "{}",
                                print::dump_kv(&self.fields, ctx, model, self.opts.raw)
                            )?,
                            Some(OutputFormat::Json) => {
                                let par = print::dump_json(&self.fields, ctx, model, self.opts.raw);
                                json_output.as_array_mut().unwrap().push(par);
                            }
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
                            Some(OutputFormat::OpenMetrics) => write!(
                                output,
                                "{}",
                                print::dump_openmetrics(&self.fields, ctx, model)
                            )?,
                        }
                        *round += 1;
                        Ok(())
                    })
                    .collect::<Result<Vec<_>>>()?;

                match (self.opts.output_format, comma_flag) {
                    (Some(OutputFormat::Json), true) => write!(output, ",{}", json_output)?,
                    (Some(OutputFormat::Json), false) => write!(output, "{}", json_output)?,
                    (Some(OutputFormat::OpenMetrics), _) => (),
                    _ => writeln!(output)?,
                };

                Ok(IterExecResult::Success)
            }
            None => Ok(IterExecResult::Skip),
        }
    }
}
