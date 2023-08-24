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

use model::CgroupModelFieldId;
use model::SingleCgroupModelFieldId;

use super::*;

pub struct Cgroup {
    opts: GeneralOpt,
    select: Option<SingleCgroupModelFieldId>,
    fields: Vec<CgroupField>,
}

impl Cgroup {
    pub fn new(
        opts: &GeneralOpt,
        select: Option<SingleCgroupModelFieldId>,
        fields: Vec<CgroupField>,
    ) -> Self {
        Self {
            opts: opts.to_owned(),
            select,
            fields,
        }
    }
}

impl Dumper for Cgroup {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        fn output_cgroup(
            handle: &Cgroup,
            ctx: &CommonFieldContext,
            model: &model::CgroupModel,
            output: &mut dyn Write,
            round: &mut usize,
            json: bool,
            jval: &mut Value,
        ) -> Result<()> {
            let cgroup = &model.data;
            //filter
            let should_print = match (handle.select.as_ref(), handle.opts.filter.as_ref()) {
                (Some(field_id), Some(filter)) => filter.is_match(
                    &cgroup
                        .query(&field_id)
                        .map_or("?".to_owned(), |v| v.to_string()),
                ),
                _ => true,
            };

            if should_print {
                match handle.opts.output_format {
                    Some(OutputFormat::Raw) | None => write!(
                        output,
                        "{}",
                        print::dump_raw_indented(
                            &handle.fields,
                            ctx,
                            cgroup,
                            *round,
                            handle.opts.repeat_title,
                            handle.opts.disable_title,
                            handle.opts.raw,
                        )
                    )?,
                    Some(OutputFormat::Csv) => write!(
                        output,
                        "{}",
                        print::dump_csv(
                            &handle.fields,
                            ctx,
                            cgroup,
                            *round,
                            handle.opts.disable_title,
                            handle.opts.raw,
                        )
                    )?,
                    Some(OutputFormat::Tsv) => write!(
                        output,
                        "{}",
                        print::dump_tsv(
                            &handle.fields,
                            ctx,
                            cgroup,
                            *round,
                            handle.opts.disable_title,
                            handle.opts.raw,
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&handle.fields, ctx, cgroup, handle.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        *jval = print::dump_json(&handle.fields, ctx, cgroup, handle.opts.raw);
                        jval["children"] = json!([]);
                    }
                    Some(OutputFormat::OpenMetrics) => write!(
                        output,
                        "{}",
                        print::dump_openmetrics(&handle.fields, ctx, cgroup)
                    )?,
                };
                *round += 1;
            }

            let mut children = Vec::from_iter(&model.children);
            //sort
            if let Some(field_id) = &handle.select {
                // field_id that queries its own data
                let field_id = CgroupModelFieldId {
                    path: Some(vec![]),
                    subquery_id: field_id.to_owned(),
                };
                if handle.opts.sort {
                    model::sort_queriables(&mut children, &field_id, false);
                }

                if handle.opts.rsort {
                    model::sort_queriables(&mut children, &field_id, false);
                }

                if (handle.opts.sort || handle.opts.rsort) && handle.opts.top != 0 {
                    children.truncate(handle.opts.top as usize);
                }
            }

            for child_cgroup in &children {
                let mut child = json!({});
                output_cgroup(handle, ctx, child_cgroup, output, round, json, &mut child)?;
                if json && child["children"].is_array() {
                    // Parent does not match, but child does, we should also render parent.
                    if !jval["children"].is_array() {
                        *jval = print::dump_json(&handle.fields, ctx, cgroup, handle.opts.raw);
                        jval["children"] = json!([]);
                    }
                    jval["children"].as_array_mut().unwrap().push(child);
                }
            }

            Ok(())
        }
        let json = self.opts.output_format == Some(OutputFormat::Json);
        let mut jval = json!({});
        output_cgroup(&self, ctx, &model.cgroup, output, round, json, &mut jval)?;
        match (json, comma_flag) {
            (true, true) => write!(output, ",{}", jval)?,
            (true, false) => write!(output, "{}", jval)?,
            _ => {}
        };

        Ok(IterExecResult::Success)
    }
}
