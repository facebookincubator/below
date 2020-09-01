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

use common::util::{convert_bytes, get_prefix};
use model::CgroupModel;

use below_derive::BelowDecor;
use std::iter::FromIterator;

#[derive(BelowDecor, Default)]
pub struct CgroupData {
    #[bttr(dfill_struct = "Cgroup")]
    #[blink("CgroupModel$get_name")]
    #[bttr(
        title = "Name",
        tag = "CgroupField::Name",
        depth = "model.depth as usize * 3",
        prefix = "get_prefix(false)"
    )]
    pub name: String,
    #[blink("CgroupModel$get_full_path")]
    #[bttr(title = "Full Path", tag = "CgroupField::FullPath")]
    pub full_path: String,
    #[blink("CgroupModel$cpu?.get_usage_pct")]
    #[bttr(
        title = "CPU Usage",
        tag = "CgroupField::CpuUsage",
        cmp = true,
        class = "CgroupField::Cpu"
    )]
    pub usage_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_user_pct")]
    #[bttr(
        title = "CPU User",
        tag = "CgroupField::CpuUser",
        cmp = true,
        class = "CgroupField::Cpu",
        class_detail = true
    )]
    pub user_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_system_pct")]
    #[bttr(
        title = "CPU Sys",
        tag = "CgroupField::CpuSystem",
        cmp = true,
        class = "CgroupField::Cpu",
        class_detail = true
    )]
    pub sys_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_periods_per_sec")]
    #[bttr(
        title = "Nr Period",
        tag = "CgroupField::CpuNrPeriods",
        cmp = true,
        class = "CgroupField::Cpu",
        class_detail = true
    )]
    pub nr_periods_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_throttled_per_sec")]
    #[bttr(
        title = "Nr Throttle",
        tag = "CgroupField::CpuNrThrottled",
        cmp = true,
        class = "CgroupField::Cpu",
        class_detail = true
    )]
    pub nr_throttled_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_throttled_pct")]
    #[bttr(
        title = "Throttle Pct",
        tag = "CgroupField::CpuThrottled",
        cmp = true,
        class = "CgroupField::Cpu",
        class_detail = true
    )]
    pub throttled_pct: Option<f64>,
    #[blink("CgroupModel$memory?.get_total")]
    #[bttr(
        title = "Mem Total",
        tag = "CgroupField::MemTotal",
        cmp = true,
        class = "CgroupField::Mem"
    )]
    pub memory_total: Option<u64>,
    #[blink("CgroupModel$memory?.get_swap")]
    #[bttr(
        title = "Mem Swap",
        tag = "CgroupField::MemSwap",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub memory_swap: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon")]
    #[bttr(
        title = "Mem Anon",
        tag = "CgroupField::MemAnon",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_file")]
    #[bttr(
        title = "Mem File",
        tag = "CgroupField::MemFile",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub file: Option<u64>,
    #[blink("CgroupModel$memory?.get_kernel_stack")]
    #[bttr(
        title = "Kernel Stack",
        tag = "CgroupField::MemKernel",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub kernel_stack: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab")]
    #[bttr(
        title = "Mem Slab",
        tag = "CgroupField::MemSlab",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub slab: Option<u64>,
    #[blink("CgroupModel$memory?.get_sock")]
    #[bttr(
        title = "Mem Sock",
        tag = "CgroupField::MemSock",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub sock: Option<u64>,
    #[blink("CgroupModel$memory?.get_shmem")]
    #[bttr(
        title = "Mem Shmem",
        tag = "CgroupField::MemShem",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub shmem: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_mapped")]
    #[bttr(
        title = "File Mapped",
        tag = "CgroupField::MemFileMapped",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub file_mapped: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_dirty")]
    #[bttr(
        title = "File Dirty",
        tag = "CgroupField::MemFileDirty",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub file_dirty: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_writeback")]
    #[bttr(
        title = "File WB",
        tag = "CgroupField::MemFileWriteBack",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub file_writeback: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon_thp")]
    #[bttr(
        title = "Anon THP",
        tag = "CgroupField::MemAnonThp",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub anon_thp: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_anon")]
    #[bttr(
        title = "Inactive Anon",
        tag = "CgroupField::MemInactiveAnon",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub inactive_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_anon")]
    #[bttr(
        title = "Active Anon",
        tag = "CgroupField::MemActiveAnon",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub active_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_file")]
    #[bttr(
        title = "Inactive File",
        tag = "CgroupField::MemInactiveFile",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub inactive_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_file")]
    #[bttr(
        title = "Active File",
        tag = "CgroupField::MemActiveFile",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub active_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_unevictable")]
    #[bttr(
        title = "Unevictable",
        tag = "CgroupField::MemUnevictable",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub unevictable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_reclaimable")]
    #[bttr(
        title = "Slab Reclaimable",
        tag = "CgroupField::MemSlabReclaimable",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub slab_reclaimable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_unreclaimable")]
    #[bttr(
        title = "Slab Unreclaimable",
        tag = "CgroupField::MemSlabUnreclaimable",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub slab_unreclaimable: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgfault")]
    #[bttr(
        title = "Pgfault",
        tag = "CgroupField::Pgfault",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgfault: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgmajfault")]
    #[bttr(
        title = "Pgmajfault",
        tag = "CgroupField::MemPgmajfault",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgmajfault: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_refault")]
    #[bttr(
        title = "Workingset Refault",
        tag = "CgroupField::MemWorkingsetRefault",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub workingset_refault: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_activate")]
    #[bttr(
        title = "Workingset Activate",
        tag = "CgroupField::MemWorkingsetActivate",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub workingset_activate: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_nodereclaim")]
    #[bttr(
        title = "Workingset Nodereclaim",
        tag = "CgroupField::MemWorkingsetNodereclaim",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub workingset_nodereclaim: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgrefill")]
    #[bttr(
        title = "Pgrefill",
        tag = "CgroupField::MemPgrefill",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgrefill: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgscan")]
    #[bttr(
        title = "Pgscan",
        tag = "CgroupField::MemPgscan",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgscan: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgsteal")]
    #[bttr(
        title = "Pgsteal",
        tag = "CgroupField::MemPgsteal",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgsteal: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgactivate")]
    #[bttr(
        title = "Pgactivate",
        tag = "CgroupField::MemPgactivate",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgactivate: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgdeactivate")]
    #[bttr(
        title = "Pgdeactivate",
        tag = "CgroupField::MemPgdeactivate",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pgdeactivate: Option<u64>,
    #[blink("CgroupModel$memory?.get_pglazyfree")]
    #[bttr(
        title = "Pglazyfree",
        tag = "CgroupField::MemPglazyfree",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pglazyfree: Option<u64>,
    #[blink("CgroupModel$memory?.get_pglazyfreed")]
    #[bttr(
        title = "Pglazyfreed",
        tag = "CgroupField::MemPglazyfreed",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub pglazyfreed: Option<u64>,
    #[blink("CgroupModel$memory?.get_thp_fault_alloc")]
    #[bttr(
        title = "THP Fault Alloc",
        tag = "CgroupField::MemTHPFaultAlloc",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub thp_fault_alloc: Option<u64>,
    #[blink("CgroupModel$memory?.get_thp_collapse_alloc")]
    #[bttr(
        title = "THP Collapse Alloc",
        tag = "CgroupField::MemTHPCollapseAlloc",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub thp_collapse_alloc: Option<u64>,
    #[blink("CgroupModel$memory?.get_memory_high")]
    #[bttr(
        title = "Memory.High",
        tag = "CgroupField::MemHigh",
        cmp = true,
        class = "CgroupField::Mem",
        class_detail = true
    )]
    pub mem_high: Option<i64>,
    #[blink("CgroupModel$pressure?.get_cpu_some_pct")]
    #[bttr(
        title = "CPU Pressure",
        tag = "CgroupField::CpuSome",
        cmp = true,
        class = "CgroupField::Pressure"
    )]
    pub cpu_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_some_pct")]
    #[bttr(
        title = "Memory Some Pressure",
        tag = "CgroupField::MemSome",
        cmp = true,
        class = "CgroupField::Pressure",
        class_detail = true
    )]
    pub pressure_memory_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_full_pct")]
    #[bttr(
        title = "Memory Pressure",
        tag = "CgroupField::MemFull",
        cmp = true,
        class = "CgroupField::Pressure"
    )]
    pub pressure_memory_full_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_some_pct")]
    #[bttr(
        title = "I/O Some Pressure",
        tag = "CgroupField::IoSome",
        cmp = true,
        class = "CgroupField::Pressure",
        class_detail = true
    )]
    pub pressure_io_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_full_pct")]
    #[bttr(
        title = "I/O Pressure",
        tag = "CgroupField::IoFull",
        cmp = true,
        class = "CgroupField::Pressure"
    )]
    pub pressure_io_full_pct: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[bttr(
        title = "RBytes",
        tag = "CgroupField::IoRead",
        cmp = true,
        class = "CgroupField::Io"
    )]
    pub rbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    #[bttr(
        title = "WBytes",
        tag = "CgroupField::IoWrite",
        cmp = true,
        class = "CgroupField::Io"
    )]
    pub wbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rios_per_sec")]
    #[bttr(
        title = "R I/O",
        tag = "CgroupField::IoRiops",
        cmp = true,
        class = "CgroupField::Io",
        class_detail = true
    )]
    pub rios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wios_per_sec")]
    #[bttr(
        title = "W I/O",
        tag = "CgroupField::IoWiops",
        cmp = true,
        class = "CgroupField::Io",
        class_detail = true
    )]
    pub wios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dbytes_per_sec")]
    #[bttr(
        title = "DBytes",
        tag = "CgroupField::IoDbps",
        cmp = true,
        class = "CgroupField::Io",
        class_detail = true
    )]
    pub dbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dios_per_sec")]
    #[bttr(
        title = "D I/O",
        tag = "CgroupField::IoDiops",
        cmp = true,
        class = "CgroupField::Io",
        class_detail = true
    )]
    pub dios_per_sec: Option<f64>,
    #[bttr(
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoTotal",
        title = "RW Total",
        cmp = true,
        class = "CgroupField::Io",
        class_detail = true
    )]
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    pub rw_total: Option<f64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "CgroupField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "CgroupField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&CgroupData, &CgroupModel) -> String>;
type FieldFtype = Box<dyn Fn(&CgroupData, &CgroupModel) -> String>;

pub struct Cgroup {
    data: CgroupData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    select: Option<CgroupField>,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Cgroup {
    type Model = CgroupModel;
    type FieldsType = CgroupField;
    type DataType = CgroupData;
}

make_dget!(
    Cgroup,
    CgroupField::Name,
    CgroupField::Datetime,
    CgroupField::Cpu,
    CgroupField::Mem,
    CgroupField::Io,
    CgroupField::Pressure,
    CgroupField::Timestamp,
);

impl Dprint for Cgroup {}

impl Dump for Cgroup {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        select: Option<CgroupField>,
    ) -> Self {
        Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
            select,
            title_fns: vec![],
            field_fns: vec![],
        }
    }

    fn advance_timestamp(&mut self, model: &model::Model) -> Result<()> {
        self.data.timestamp = match model.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(t) => t.as_secs() as i64,
            Err(e) => bail!("Fail to convert system time: {}", e),
        };
        self.data.datetime = self.data.timestamp;

        Ok(())
    }

    fn iterate_exec<T: Write>(
        &self,
        model: &model::Model,
        output: &mut T,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        fn output_cgroup<T: Write>(
            handle: &Cgroup,
            model: &CgroupModel,
            output: &mut T,
            round: &mut usize,
            json: bool,
            jval: &mut Value,
        ) -> Result<()> {
            //filter
            let should_print = match (handle.select, handle.opts.filter.as_ref()) {
                (Some(tag), Some(filter)) => handle.filter_by(model, &tag, &filter),
                _ => true,
            };

            if should_print {
                match handle.opts.output_format {
                    Some(OutputFormat::Raw) | None => {
                        handle.do_print_raw(&model, output, *round)?
                    }
                    Some(OutputFormat::Csv) => handle.do_print_csv(&model, output, *round)?,
                    Some(OutputFormat::KeyVal) => handle.do_print_kv(&model, output)?,
                    Some(OutputFormat::Json) => {
                        *jval = handle.do_print_json(&model);
                        jval["children"] = json!([]);
                    }
                };
                *round += 1;
            }

            let mut children = Vec::from_iter(&model.children);
            //sort
            if let Some(tag) = handle.select {
                if handle.opts.sort {
                    Cgroup::sort_by(&mut children, &tag, false);
                }

                if handle.opts.rsort {
                    Cgroup::sort_by(&mut children, &tag, true);
                }

                if (handle.opts.sort && handle.opts.rsort) || handle.opts.top != 0 {
                    children.truncate(handle.opts.top as usize);
                }
            }

            for child_cgroup in &children {
                let mut child = json!({});
                output_cgroup(handle, child_cgroup, output, round, json, &mut child)?;
                if json && child["children"].is_array() {
                    // Parent does not match, but child does, we should also render parent.
                    if !jval["children"].is_array() {
                        *jval = handle.do_print_json(&model);
                        jval["children"] = json!([]);
                    }
                    jval["children"].as_array_mut().unwrap().push(child);
                }
            }

            Ok(())
        };
        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let mut jval = json!({});
        output_cgroup(&self, &model.cgroup, output, round, json, &mut jval)?;
        match (json, comma_flag) {
            (true, true) => write!(output, ",{}", jval)?,
            (true, false) => write!(output, "{}", jval)?,
            _ => (),
        };

        Ok(IterExecResult::Success)
    }
}
