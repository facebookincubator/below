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

use model::CgroupModelFieldId;
use render::RenderConfig;

use std::iter::FromIterator;

impl HasRenderConfigForDump for model::CgroupModel {
    fn get_render_config_for_dump(field_id: &CgroupModelFieldId) -> RenderConfig {
        use common::util::get_prefix;
        use model::CgroupCpuModelFieldId::ThrottledPct;
        use model::CgroupIoModelFieldId::{
            DbytesPerSec, DiosPerSec, RbytesPerSec, RiosPerSec, RwbytesPerSec, WbytesPerSec,
            WiosPerSec,
        };
        use model::CgroupMemoryModelFieldId::{
            Anon, File, Pgactivate, Pgdeactivate, Pgfault, Pglazyfree, Pglazyfreed, Pgmajfault,
            Pgrefill, Pgscan, Pgsteal, Shmem, Slab, Sock, Swap, ThpCollapseAlloc, ThpFaultAlloc,
            Total, WorkingsetActivate, WorkingsetNodereclaim, WorkingsetRefault,
        };
        use model::CgroupModelFieldId::{Cpu, Io, Mem, Name, Pressure};
        use model::CgroupPressureModelFieldId::{MemoryFullPct, MemorySomePct};
        use render::{render_config as rc, HasRenderConfig};

        let config = model::CgroupModel::get_render_config(field_id);
        match field_id {
            Name => config.update(rc!(indented_prefix(get_prefix(false)))),
            Cpu(ThrottledPct) => config.update(rc!(title("Throttled Pct"))),
            Io(RbytesPerSec) => config.update(rc!(title("RBytes"))),
            Io(WbytesPerSec) => config.update(rc!(title("WBytes"))),
            Io(DbytesPerSec) => config.update(rc!(title("DBytes"))),
            Io(RiosPerSec) => config.update(rc!(title("R I/O"))),
            Io(WiosPerSec) => config.update(rc!(title("W I/O"))),
            Io(DiosPerSec) => config.update(rc!(title("D I/O"))),
            Io(RwbytesPerSec) => config.update(rc!(title("RW Total"))),
            Mem(Total) => config.update(rc!(title("Mem Total"))),
            Mem(Swap) => config.update(rc!(title("Mem Swap"))),
            Mem(Anon) => config.update(rc!(title("Mem Anon"))),
            Mem(File) => config.update(rc!(title("Mem File"))),
            Mem(Slab) => config.update(rc!(title("Mem Slab"))),
            Mem(Sock) => config.update(rc!(title("Mem Sock"))),
            Mem(Shmem) => config.update(rc!(title("Mem Shmem"))),
            Mem(Pgfault) => config.update(rc!(title("Pgfault"))),
            Mem(Pgmajfault) => config.update(rc!(title("Pgmajfault"))),
            Mem(WorkingsetRefault) => config.update(rc!(title("Workingset Refault"))),
            Mem(WorkingsetActivate) => config.update(rc!(title("Workingset Activate"))),
            Mem(WorkingsetNodereclaim) => config.update(rc!(title("Workingset Nodereclaim"))),
            Mem(Pgrefill) => config.update(rc!(title("Pgrefill"))),
            Mem(Pgscan) => config.update(rc!(title("Pgscan"))),
            Mem(Pgsteal) => config.update(rc!(title("Pgsteal"))),
            Mem(Pgactivate) => config.update(rc!(title("Pgactivate"))),
            Mem(Pgdeactivate) => config.update(rc!(title("Pgdeactivate"))),
            Mem(Pglazyfree) => config.update(rc!(title("Pglazyfree"))),
            Mem(Pglazyfreed) => config.update(rc!(title("Pglazyfreed"))),
            Mem(ThpFaultAlloc) => config.update(rc!(title("THP Fault Alloc"))),
            Mem(ThpCollapseAlloc) => config.update(rc!(title("THP Collapse Alloc"))),
            Pressure(MemorySomePct) => config.update(rc!(title("Memory Some Pressure"))),
            Pressure(MemoryFullPct) => config.update(rc!(title("Memory Pressure"))),
            _ => config,
        }
    }
}

pub struct Cgroup {
    opts: GeneralOpt,
    select: Option<CgroupModelFieldId>,
    fields: Vec<CgroupField>,
}

impl Cgroup {
    pub fn new(
        opts: &GeneralOpt,
        select: Option<CgroupModelFieldId>,
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
            //filter
            let should_print = match (handle.select.as_ref(), handle.opts.filter.as_ref()) {
                (Some(field_id), Some(filter)) => filter.is_match(
                    &model
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
                            model,
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
                            model,
                            *round,
                            handle.opts.disable_title,
                            handle.opts.raw,
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&handle.fields, ctx, model, handle.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        *jval = print::dump_json(&handle.fields, ctx, model, handle.opts.raw);
                        jval["children"] = json!([]);
                    }
                };
                *round += 1;
            }

            let mut children = Vec::from_iter(&model.children);
            //sort
            if let Some(field_id) = &handle.select {
                if handle.opts.sort {
                    model::sort_queriables(&mut children, &field_id, false);
                }

                if handle.opts.rsort {
                    model::sort_queriables(&mut children, &field_id, true);
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
                        *jval = print::dump_json(&handle.fields, ctx, model, handle.opts.raw);
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
