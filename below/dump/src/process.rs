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

use model::SingleProcessModelFieldId;
use render::RenderConfig;

use super::*;

impl HasRenderConfigForDump for model::SingleProcessModel {
    fn get_render_config_for_dump(field_id: &SingleProcessModelFieldId) -> RenderConfig {
        use model::ProcessCpuModelFieldId::SystemPct;
        use model::ProcessCpuModelFieldId::UserPct;
        use model::ProcessIoModelFieldId::RwbytesPerSec;
        use model::SingleProcessModelFieldId::Cpu;
        use model::SingleProcessModelFieldId::Io;
        use render::HasRenderConfig;

        let rc = model::SingleProcessModel::get_render_config_builder(field_id);
        match field_id {
            Cpu(UserPct) => rc.title("User CPU"),
            Cpu(SystemPct) => rc.title("Sys CPU"),
            Io(RwbytesPerSec) => rc.title("RW"),
            _ => rc,
        }
        .get()
    }
}

pub struct Process {
    opts: GeneralOpt,
    select: Option<SingleProcessModelFieldId>,
    fields: Vec<ProcessField>,
}

impl Process {
    pub fn new(
        opts: &GeneralOpt,
        select: Option<SingleProcessModelFieldId>,
        fields: Vec<ProcessField>,
    ) -> Self {
        Self {
            opts: opts.to_owned(),
            select,
            fields,
        }
    }
}

impl Dumper for Process {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        let mut processes: Vec<_> = model
            .process
            .processes
            .iter()
            .filter_map(
                |(_, spm)| match (self.select.as_ref(), self.opts.filter.as_ref()) {
                    (Some(field_id), Some(filter))
                        if !filter.is_match(
                            &spm.query(&field_id)
                                .map_or("?".to_owned(), |v| v.to_string()),
                        ) =>
                    {
                        None
                    }
                    _ => Some(spm),
                },
            )
            .collect();

        // Return if we filtered everything.
        if processes.is_empty() {
            return Ok(IterExecResult::Skip);
        }

        if let Some(field_id) = self.select.as_ref() {
            if self.opts.sort {
                model::sort_queriables(&mut processes, &field_id, false);
            }

            if self.opts.rsort {
                model::sort_queriables(&mut processes, &field_id, true);
            }

            if (self.opts.sort || self.opts.rsort) && self.opts.top != 0 {
                processes.truncate(self.opts.top as usize);
            }
        }
        let json = self.opts.output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        processes
            .into_iter()
            .map(|spm| {
                match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => write!(
                        output,
                        "{}",
                        print::dump_raw(
                            &self.fields,
                            ctx,
                            spm,
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
                            spm,
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
                            spm,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&self.fields, ctx, spm, self.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        let par = print::dump_json(&self.fields, ctx, spm, self.opts.raw);
                        json_output.as_array_mut().unwrap().push(par);
                    }
                    Some(OutputFormat::OpenMetrics) => write!(
                        output,
                        "{}",
                        print::dump_openmetrics(&self.fields, ctx, spm)
                    )?,
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
