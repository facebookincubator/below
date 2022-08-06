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

use model::SingleCgroupModelFieldId;
use render::RenderConfig;

use super::*;

impl HasRenderConfigForDump for model::SingleCgroupModel {
    fn get_render_config_for_dump(field_id: &SingleCgroupModelFieldId) -> RenderConfig {
        use common::util::get_prefix;
        use model::CgroupCpuModelFieldId::ThrottledPct;
        use model::CgroupIoModelFieldId::CostIndebtPct;
        use model::CgroupIoModelFieldId::CostIndelayPct;
        use model::CgroupIoModelFieldId::CostUsagePct;
        use model::CgroupIoModelFieldId::CostWaitPct;
        use model::CgroupIoModelFieldId::DbytesPerSec;
        use model::CgroupIoModelFieldId::DiosPerSec;
        use model::CgroupIoModelFieldId::RbytesPerSec;
        use model::CgroupIoModelFieldId::RiosPerSec;
        use model::CgroupIoModelFieldId::RwbytesPerSec;
        use model::CgroupIoModelFieldId::WbytesPerSec;
        use model::CgroupIoModelFieldId::WiosPerSec;
        use model::CgroupMemoryModelFieldId::Anon;
        use model::CgroupMemoryModelFieldId::File;
        use model::CgroupMemoryModelFieldId::Pgactivate;
        use model::CgroupMemoryModelFieldId::Pgdeactivate;
        use model::CgroupMemoryModelFieldId::Pgfault;
        use model::CgroupMemoryModelFieldId::Pglazyfree;
        use model::CgroupMemoryModelFieldId::Pglazyfreed;
        use model::CgroupMemoryModelFieldId::Pgmajfault;
        use model::CgroupMemoryModelFieldId::Pgrefill;
        use model::CgroupMemoryModelFieldId::Pgscan;
        use model::CgroupMemoryModelFieldId::Pgsteal;
        use model::CgroupMemoryModelFieldId::Shmem;
        use model::CgroupMemoryModelFieldId::Slab;
        use model::CgroupMemoryModelFieldId::Sock;
        use model::CgroupMemoryModelFieldId::Swap;
        use model::CgroupMemoryModelFieldId::ThpCollapseAlloc;
        use model::CgroupMemoryModelFieldId::ThpFaultAlloc;
        use model::CgroupMemoryModelFieldId::Total;
        use model::CgroupMemoryModelFieldId::WorkingsetActivate;
        use model::CgroupMemoryModelFieldId::WorkingsetNodereclaim;
        use model::CgroupMemoryModelFieldId::WorkingsetRefault;
        use model::CgroupMemoryModelFieldId::Zswap;
        use model::CgroupPressureModelFieldId::MemoryFullPct;
        use model::CgroupPressureModelFieldId::MemorySomePct;
        use model::SingleCgroupModelFieldId::Cpu;
        use model::SingleCgroupModelFieldId::Io;
        use model::SingleCgroupModelFieldId::Mem;
        use model::SingleCgroupModelFieldId::Name;
        use model::SingleCgroupModelFieldId::Pressure;
        use render::HasRenderConfig;

        let rc = model::SingleCgroupModel::get_render_config_builder(field_id);
        match field_id {
            Name => rc.indented_prefix(get_prefix(false)),
            Cpu(ThrottledPct) => rc.title("Throttled Pct"),
            Io(RbytesPerSec) => rc.title("RBytes"),
            Io(WbytesPerSec) => rc.title("WBytes"),
            Io(DbytesPerSec) => rc.title("DBytes"),
            Io(RiosPerSec) => rc.title("R I/O"),
            Io(WiosPerSec) => rc.title("W I/O"),
            Io(DiosPerSec) => rc.title("D I/O"),
            Io(RwbytesPerSec) => rc.title("RW Total"),
            Io(CostUsagePct) => rc.title("Cost Usage"),
            Io(CostWaitPct) => rc.title("Cost Wait"),
            Io(CostIndebtPct) => rc.title("Cost Indebt"),
            Io(CostIndelayPct) => rc.title("Cost Indelay"),
            Mem(Total) => rc.title("Mem Total"),
            Mem(Swap) => rc.title("Mem Swap"),
            Mem(Zswap) => rc.title("Mem Zswap"),
            Mem(Anon) => rc.title("Mem Anon"),
            Mem(File) => rc.title("Mem File"),
            Mem(Slab) => rc.title("Mem Slab"),
            Mem(Sock) => rc.title("Mem Sock"),
            Mem(Shmem) => rc.title("Mem Shmem"),
            Mem(Pgfault) => rc.title("Pgfault"),
            Mem(Pgmajfault) => rc.title("Pgmajfault"),
            Mem(WorkingsetRefault) => rc.title("Workingset Refault"),
            Mem(WorkingsetActivate) => rc.title("Workingset Activate"),
            Mem(WorkingsetNodereclaim) => rc.title("Workingset Nodereclaim"),
            Mem(Pgrefill) => rc.title("Pgrefill"),
            Mem(Pgscan) => rc.title("Pgscan"),
            Mem(Pgsteal) => rc.title("Pgsteal"),
            Mem(Pgactivate) => rc.title("Pgactivate"),
            Mem(Pgdeactivate) => rc.title("Pgdeactivate"),
            Mem(Pglazyfree) => rc.title("Pglazyfree"),
            Mem(Pglazyfreed) => rc.title("Pglazyfreed"),
            Mem(ThpFaultAlloc) => rc.title("THP Fault Alloc"),
            Mem(ThpCollapseAlloc) => rc.title("THP Collapse Alloc"),
            Pressure(MemorySomePct) => rc.title("Memory Some Pressure"),
            Pressure(MemoryFullPct) => rc.title("Memory Pressure"),
            _ => rc,
        }
        .get()
    }
}

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
                };
                *round += 1;
            }

            let mut children = Vec::from_iter(&model.children);
            //sort
            if let Some(field_id) = &handle.select {
                if handle.opts.sort {
                    model::sort_queriables(&mut children, &field_id.to_owned().into(), false);
                }

                if handle.opts.rsort {
                    model::sort_queriables(&mut children, &field_id.to_owned().into(), true);
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
