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

use crate::model::CgroupModel;
use crate::util::{convert_bytes, get_prefix};

use below_derive::BelowDecor;
use std::iter::FromIterator;

#[derive(BelowDecor, Default)]
pub struct CgroupData {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        title = "Name",
        width = 50,
        tag = "CgroupField::Name&",
        depth = "model.depth as usize * 3",
        prefix = "get_prefix(false)"
    )]
    pub name: String,
    #[blink("CgroupModel$get_full_path")]
    #[bttr(title = "Full Path", width = 50, tag = "CgroupField::FullPath&")]
    pub full_path: String,
    #[blink("CgroupModel$cpu?.get_usage_pct")]
    #[bttr(
        title = "CPU Usage",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::CpuUsage&",
        cmp = true
    )]
    pub usage_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_user_pct")]
    #[bttr(
        title = "CPU User",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::CpuUser&",
        cmp = true
    )]
    pub user_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_system_pct")]
    #[bttr(
        title = "CPU Sys",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::CpuSystem&",
        cmp = true
    )]
    pub sys_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_periods_per_sec")]
    #[bttr(
        title = "Nr Period",
        width = 15,
        unit = "/s",
        precision = 2,
        tag = "CgroupField::CpuNrPeriods&",
        cmp = true
    )]
    pub nr_periods_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_throttled_per_sec")]
    #[bttr(
        title = "Nr Throttle",
        width = 15,
        unit = "/s",
        precision = 2,
        tag = "CgroupField::CpuNrThrottled&",
        cmp = true
    )]
    pub nr_throttled_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_throttled_pct")]
    #[bttr(
        title = "Throttle Pct",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::CpuThrottled&",
        cmp = true
    )]
    pub throttled_pct: Option<f64>,
    #[blink("CgroupModel$memory?.get_total")]
    #[bttr(
        title = "Mem Total",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemTotal&",
        cmp = true
    )]
    pub memory_total: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon")]
    #[bttr(
        title = "Mem Anon",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemAnon&",
        cmp = true
    )]
    pub anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_file")]
    #[bttr(
        title = "Mem File",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemFile&",
        cmp = true
    )]
    pub file: Option<u64>,
    #[blink("CgroupModel$memory?.get_kernel_stack")]
    #[bttr(
        title = "Kernel Stack",
        width = 12,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemKernel&",
        cmp = true
    )]
    pub kernel_stack: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab")]
    #[bttr(
        title = "Mem Slab",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemSlab&",
        cmp = true
    )]
    pub slab: Option<u64>,
    #[blink("CgroupModel$memory?.get_sock")]
    #[bttr(
        title = "Mem Sock",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemSock&",
        cmp = true
    )]
    pub sock: Option<u64>,
    #[blink("CgroupModel$memory?.get_shmem")]
    #[bttr(
        title = "Mem Shmem",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemShem&",
        cmp = true
    )]
    pub shmem: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_mapped")]
    #[bttr(
        title = "File Mapped",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemFileMapped&",
        cmp = true
    )]
    pub file_mapped: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_dirty")]
    #[bttr(
        title = "File Dirty",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemFileDirty&",
        cmp = true
    )]
    pub file_dirty: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_writeback")]
    #[bttr(
        title = "File WB",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemFileWriteBack&",
        cmp = true
    )]
    pub file_writeback: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon_thp")]
    #[bttr(
        title = "Anon THP",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemAnonThp&",
        cmp = true
    )]
    pub anon_thp: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_anon")]
    #[bttr(
        title = "Inactive Anon",
        width = 13,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemInactiveAnon&",
        cmp = true
    )]
    pub inactive_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_anon")]
    #[bttr(
        title = "Active Anon",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemActiveAnon&",
        cmp = true
    )]
    pub active_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_file")]
    #[bttr(
        title = "Inactive File",
        width = 13,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemInactiveFile&",
        cmp = true
    )]
    pub inactive_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_file")]
    #[bttr(
        title = "Active File",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemActiveFile&",
        cmp = true
    )]
    pub active_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_unevictable")]
    #[bttr(
        title = "Unevictable",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemUnevictable&",
        cmp = true
    )]
    pub unevictable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_reclaimable")]
    #[bttr(
        title = "Slab Reclaimable",
        width = 16,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemSlabReclaimable&",
        cmp = true
    )]
    pub slab_reclaimable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_unreclaimable")]
    #[bttr(
        title = "Slab Unreclaimable",
        width = 18,
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::MemSlabUnreclaimable&",
        cmp = true
    )]
    pub slab_unreclaimable: Option<u64>,
    #[blink("CgroupModel$pressure?.get_cpu_some_pct")]
    #[bttr(
        title = "CPU Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::CpuSome&",
        cmp = true
    )]
    pub cpu_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_some_pct")]
    #[bttr(
        title = "Memory Some Pressure",
        width = 20,
        unit = "%",
        precision = 2,
        tag = "CgroupField::MemSome&",
        cmp = true
    )]
    pub pressure_memory_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_full_pct")]
    #[bttr(
        title = "Memory Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::MemFull&",
        cmp = true
    )]
    pub pressure_memory_full_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_some_pct")]
    #[bttr(
        title = "I/O Some Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::IoSome&",
        cmp = true
    )]
    pub pressure_io_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_full_pct")]
    #[bttr(
        title = "I/O Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        tag = "CgroupField::IoFull&",
        cmp = true
    )]
    pub pressure_io_full_pct: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[bttr(
        title = "RBytes",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoRead&",
        cmp = true
    )]
    pub rbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    #[bttr(
        title = "WBytes",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoWrite&",
        cmp = true
    )]
    pub wbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rios_per_sec")]
    #[bttr(
        title = "R I/O",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoRiops&",
        cmp = true
    )]
    pub rios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wios_per_sec")]
    #[bttr(
        title = "W I/O",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoWiops&",
        cmp = true
    )]
    pub wios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dbytes_per_sec")]
    #[bttr(
        title = "DBytes",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoDbps&",
        cmp = true
    )]
    pub dbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dios_per_sec")]
    #[bttr(
        title = "D I/O",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoDiops&",
        cmp = true
    )]
    pub dios_per_sec: Option<f64>,
    #[bttr(
        aggr = "CgroupModel: io_total?.rbytes_per_sec? + io_total?.wbytes_per_sec?",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        tag = "CgroupField::IoTotal&",
        title = "RW Total",
        cmp = true
    )]
    pub rw_total: Option<f64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime($)",
        tag = "CgroupField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "CgroupField::Timestamp")]
    timestamp: i64,
    #[bttr(
        class = "CgroupField$usage_pct&:user_pct&,sys_pct&,nr_periods_per_sec&,nr_throttled_per_sec&,throttled_pct&"
    )]
    pub cpu: AwaysNone,
    #[bttr(
        class = "CgroupField$memory_total&:anon&,file&,kernel_stack&,slab&,sock&,shmem&,file_mapped&,file_dirty&,file_writeback&,anon_thp&,inactive_anon&,active_anon&,inactive_file&,active_file&,unevictable&,slab_reclaimable&,slab_unreclaimable&"
    )]
    pub mem: AwaysNone,
    #[bttr(
        class = "CgroupField$rbytes_per_sec&,wbytes_per_sec&:rw_total@,rios_per_sec&,wios_per_sec&,dbytes_per_sec&,dios_per_sec&"
    )]
    pub io: AwaysNone,
    #[bttr(
        class = "CgroupField$cpu_some_pct&,pressure_memory_full_pct&,pressure_io_full_pct&:pressure_io_some_pct&,pressure_memory_some_pct&"
    )]
    pub pressure: AwaysNone,
}

type TitleFtype = Box<dyn Fn(&CgroupData, &CgroupModel) -> &'static str>;
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
    ) -> Result<()> {
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
                (Some(tag), Some(re)) => handle.filter_by(model, &tag, &Regex::new(re)?),
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
        if json {
            write!(output, "{}", jval)?;
        }

        Ok(())
    }
}
