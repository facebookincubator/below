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

use crate::model::SystemModel;
use crate::util::convert_bytes;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct SystemData {
    #[bttr(title = "Hostname", width = 30, tag = "SysField::Hostname&")]
    #[blink("SystemModel$get_hostname")]
    pub hostname: String,
    #[bttr(
        title = "CPU Usage",
        width = 11,
        unit = "%",
        precision = 2,
        tag = "SysField::CpuUsagePct&"
    )]
    #[blink("SystemModel$cpu?.get_usage_pct")]
    pub cpu_usage_pct: Option<f64>,
    #[bttr(
        title = "CPU User",
        width = 11,
        unit = "%",
        precision = 2,
        tag = "SysField::CpuUserPct&"
    )]
    #[blink("SystemModel$cpu?.get_user_pct")]
    pub cpu_user_pct: Option<f64>,
    #[bttr(
        title = "CPU Sys",
        width = 11,
        unit = "%",
        precision = 2,
        tag = "SysField::CpuSystemPct&"
    )]
    #[blink("SystemModel$cpu?.get_system_pct")]
    pub cpu_system_pct: Option<f64>,
    #[bttr(
        title = "Mem Total",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::MemTotal&"
    )]
    #[blink("SystemModel$mem?.get_total")]
    pub mem_total: Option<u64>,
    #[bttr(
        title = "Mem Free",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::MemFree&"
    )]
    #[blink("SystemModel$mem?.get_free")]
    pub mem_free: Option<u64>,
    #[bttr(
        title = "Mem Anon",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::MemAnon&"
    )]
    #[blink("SystemModel$mem?.get_anon")]
    pub mem_anon: Option<u64>,
    #[bttr(
        title = "Mem File",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::MemFile&"
    )]
    #[blink("SystemModel$mem?.get_file")]
    pub mem_file: Option<u64>,
    #[bttr(
        title = "Huge Page Total",
        width = 16,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::HpTotal&"
    )]
    #[blink("SystemModel$mem?.get_hugepage_total")]
    pub hugepage_total: Option<u64>,
    #[bttr(
        title = "Huge Page Free",
        width = 16,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::HpFree&"
    )]
    #[blink("SystemModel$mem?.get_hugepage_free")]
    pub hugepage_free: Option<u64>,
    #[bttr(
        title = "Reads",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::IoRead&",
        unit = "/s"
    )]
    #[blink("SystemModel$io?.get_rbytes_per_sec")]
    pub io_rbytes_per_sec: Option<f64>,
    #[bttr(
        title = "Writes",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        tag = "SysField::IoWrite&",
        unit = "/s"
    )]
    #[blink("SystemModel$io?.get_wbytes_per_sec")]
    pub io_wbytes_per_sec: Option<f64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime($)",
        tag = "SysField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "SysField::Timestamp")]
    timestamp: i64,
    #[bttr(class = "SysField$cpu_usage_pct&,cpu_user_pct&,cpu_system_pct&")]
    pub cpu: AwaysNone,
    #[bttr(
        class = "SysField$mem_total&,mem_free&:mem_anon&,mem_file&,hugepage_total&,hugepage_free&"
    )]
    pub mem: AwaysNone,
    #[bttr(class = "SysField$io_rbytes_per_sec&,io_wbytes_per_sec&")]
    pub io: AwaysNone,
}

type TitleFtype = Box<dyn Fn(&SystemData, &SystemModel) -> &'static str>;
type FieldFtype = Box<dyn Fn(&SystemData, &SystemModel) -> String>;

pub struct System {
    data: SystemData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for System {
    type Model = SystemModel;
    type FieldsType = SysField;
    type DataType = SystemData;
}

make_dget!(
    System,
    SysField::Datetime,
    SysField::Cpu,
    SysField::Mem,
    SysField::Io,
);

impl Dprint for System {}

impl Dump for System {
    fn new(opts: GeneralOpt, advance: Advance, time_end: SystemTime, _: Option<SysField>) -> Self {
        Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
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
        match self.opts.output_format {
            Some(OutputFormat::Raw) | None => self.do_print_raw(&model.system, output, *round)?,
            Some(OutputFormat::Csv) => self.do_print_csv(&model.system, output, *round)?,
            Some(OutputFormat::KeyVal) => self.do_print_kv(&model.system, output)?,
            Some(OutputFormat::Json) => {
                let par = self.do_print_json(&model.system);
                if comma_flag {
                    write!(output, ",{}", par.to_string())?;
                } else {
                    write!(output, "{}", par.to_string())?;
                }
            }
        };

        *round += 1;

        Ok(IterExecResult::Success)
    }
}
