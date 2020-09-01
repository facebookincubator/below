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

use common::util::convert_bytes;
use model::SingleProcessModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct ProcessData {
    #[bttr(dfill_struct = "Process")]
    #[bttr(title = "Pid", tag = "ProcField::Pid", cmp = true)]
    #[blink("SingleProcessModel$get_pid")]
    pub pid: Option<i32>,
    #[bttr(title = "Ppid", tag = "ProcField::Ppid", cmp = true)]
    #[blink("SingleProcessModel$get_ppid")]
    pub ppid: Option<i32>,
    #[bttr(title = "Comm", tag = "ProcField::Comm", cmp = true)]
    #[blink("SingleProcessModel$get_comm")]
    pub comm: Option<String>,
    #[bttr(title = "Cmdline", tag = "ProcField::Cmdline", cmp = true)]
    #[blink("SingleProcessModel$get_cmdline")]
    pub cmdline: Option<String>,
    #[bttr(title = "Exe Path", tag = "ProcField::ExePath", cmp = true)]
    #[blink("SingleProcessModel$get_exe_path")]
    pub exe_path: Option<String>,
    #[bttr(title = "State", tag = "ProcField::State", cmp = true)]
    #[blink("SingleProcessModel$get_state")]
    pub state: Option<procfs::PidState>,
    #[bttr(title = "Uptime(sec)", tag = "ProcField::Uptime", cmp = true)]
    #[blink("SingleProcessModel$get_uptime_secs")]
    pub uptime_secs: Option<u64>,
    #[bttr(title = "Cgroup", tag = "ProcField::Cgroup", cmp = true)]
    #[blink("SingleProcessModel$get_cgroup")]
    pub cgroup: Option<String>,
    #[bttr(
        title = "User CPU",
        tag = "ProcField::CpuUserPct",
        cmp = true,
        class = "ProcField::Cpu"
    )]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    pub cpu_user: Option<f64>,
    #[bttr(
        title = "Sys CPU",
        tag = "ProcField::CpuSysPct",
        cmp = true,
        class = "ProcField::Cpu",
        class_detail = true
    )]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_sys: Option<f64>,
    #[bttr(
        title = "Threads",
        tag = "ProcField::CpuNumThreads",
        cmp = true,
        class = "ProcField::Cpu",
        class_detail = true
    )]
    #[blink("SingleProcessModel$cpu?.get_num_threads")]
    pub cpu_num_threads: Option<u64>,
    #[bttr(
        title = "CPU",
        tag = "ProcField::CpuTotalPct",
        cmp = true,
        class = "ProcField::Cpu",
        class_detail = true,
        width = 11,
        precision = 2,
        unit = "%"
    )]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_total: Option<f64>,
    #[bttr(
        title = "RSS",
        tag = "ProcField::MemRssBytes",
        cmp = true,
        class = "ProcField::Mem"
    )]
    #[blink("SingleProcessModel$mem?.get_rss_bytes")]
    pub mem_rss: Option<u64>,
    #[bttr(
        title = "Minflt",
        tag = "ProcField::MemMinor",
        cmp = true,
        class = "ProcField::Mem",
        class_detail = true
    )]
    #[blink("SingleProcessModel$mem?.get_minorfaults_per_sec")]
    pub mem_minorfaults: Option<f64>,
    #[bttr(
        title = "Majflt",
        tag = "ProcField::MemMajor",
        cmp = true,
        class = "ProcField::Mem",
        class_detail = true
    )]
    #[blink("SingleProcessModel$mem?.get_majorfaults_per_sec")]
    pub mem_majorfaults: Option<f64>,
    #[bttr(
        title = "Reads",
        tag = "ProcField::IoRead",
        cmp = true,
        class = "ProcField::Io"
    )]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    pub io_read: Option<f64>,
    #[bttr(
        title = "Writes",
        tag = "ProcField::IoWrite",
        cmp = true,
        class = "ProcField::Io"
    )]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_write: Option<f64>,
    #[bttr(
        title = "RW",
        tag = "ProcField::IoTotal",
        decorator = "convert_bytes($ as f64)",
        width = 11,
        unit = "/s",
        cmp = true,
        class = "ProcField::Io",
        class_detail = true
    )]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_total: Option<f64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "ProcField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "ProcField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&ProcessData, &SingleProcessModel) -> String>;
type FieldFtype = Box<dyn Fn(&ProcessData, &SingleProcessModel) -> String>;

pub struct Process {
    data: ProcessData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    select: Option<ProcField>,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Process {
    type Model = SingleProcessModel;
    type FieldsType = ProcField;
    type DataType = ProcessData;
}

make_dget!(
    Process,
    ProcField::Datetime,
    ProcField::Pid,
    ProcField::Ppid,
    ProcField::Comm,
    ProcField::State,
    ProcField::Cpu,
    ProcField::Mem,
    ProcField::Io,
    ProcField::Uptime,
    ProcField::Cgroup,
    ProcField::Timestamp,
    ProcField::Cmdline,
    ProcField::ExePath,
);

impl Dprint for Process {}

impl Dump for Process {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        select: Option<ProcField>,
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
        let mut processes: Vec<&SingleProcessModel> = model
            .process
            .processes
            .iter()
            .filter_map(|(_, spm)| match (self.select, self.opts.filter.as_ref()) {
                (Some(tag), Some(filter)) => {
                    if self.filter_by(spm, &tag, &filter) {
                        Some(spm)
                    } else {
                        None
                    }
                }
                _ => Some(spm),
            })
            .collect();

        // Return if we filtered everything.
        if processes.is_empty() {
            return Ok(IterExecResult::Skip);
        }

        if let Some(tag) = self.select {
            if self.opts.sort {
                Self::sort_by(&mut processes, &tag, false);
            }

            if self.opts.rsort {
                Self::sort_by(&mut processes, &tag, true);
            }

            if (self.opts.sort || self.opts.rsort) && self.opts.top != 0 {
                processes.truncate(self.opts.top as usize);
            }
        }
        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        processes
            .iter()
            .map(|spm| {
                let ret = match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => self.do_print_raw(&spm, output, *round),
                    Some(OutputFormat::Csv) => self.do_print_csv(&spm, output, *round),
                    Some(OutputFormat::KeyVal) => self.do_print_kv(&spm, output),
                    Some(OutputFormat::Json) => {
                        let par = self.do_print_json(&spm);
                        json_output.as_array_mut().unwrap().push(par);
                        Ok(())
                    }
                };
                *round += 1;
                ret
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
