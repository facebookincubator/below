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

use crate::process_view::ProcessState;
use crate::stats_view::StateCommon;
use below_derive::BelowDecor;
use common::util::{convert_bytes, is_cpu_significant};
use model::SingleProcessModel;

use cursive::utils::markup::StyledString;

// All available sorting tags
#[derive(Copy, Clone, PartialEq)]
pub enum ProcessOrders {
    Keep,
    Pid,
    Ppid,
    Comm,
    State,
    UptimeSecs,
    Cgroup,
    CpuUser,
    CpuSys,
    CpuNumThreads,
    CpuTotal,
    Rss,
    MinorFaults,
    MajorFaults,
    Read,
    Write,
    IoTotal,
    Cmdline,
}

impl Default for ProcessOrders {
    fn default() -> Self {
        ProcessOrders::Keep
    }
}

// Defines how to iterate through the process stats and generate get_rows for ViewBridge
pub trait ProcessTab {
    fn get_title_vec(&self, model: &SingleProcessModel) -> Vec<String>;
    fn get_field_line(&self, model: &SingleProcessModel) -> StyledString;
    fn sort(
        &self,
        sort_order: ProcessOrders,
        processes: &mut Vec<&SingleProcessModel>,
        reverse: bool,
    );

    fn get_rows(&mut self, state: &ProcessState) -> Vec<(StyledString, String)> {
        let unknown = "?".to_string();
        let process_model = state.get_model();
        let mut processes: Vec<&SingleProcessModel> =
            process_model.processes.iter().map(|(_, spm)| spm).collect();

        self.sort(state.sort_order, &mut processes, state.reverse);
        processes
            .iter()
            .filter(|spm| {
                // If we're in zoomed cgroup mode, only show processes belonging to
                // our zoomed cgroup
                if let Some(f) = &state.cgroup_filter {
                    spm.cgroup.as_ref().unwrap_or(&unknown).starts_with(f)
                } else {
                    true
                }
            })
            .filter(|spm| {
                // If we're filtering by name, only show processes who pass the filter
                if let Some(f) = &state.filter {
                    spm.comm.as_ref().unwrap_or(&unknown).contains(f)
                } else {
                    true
                }
            })
            .map(|spm| (self.get_field_line(&spm), spm.pid.unwrap_or(0).to_string()))
            .collect()
    }
}

macro_rules! impl_process_tab {
    ($name:ident) => {
        fn get_title_vec(&self, model: &SingleProcessModel) -> Vec<String> {
            let mut res: Vec<String> = self
                .get_title_pipe(&model)
                .trim()
                .split("|")
                .map(|s| s.to_string())
                .collect();
            res.pop();
            res
        }

        fn sort(
            &self,
            sort_order: ProcessOrders,
            processes: &mut Vec<&SingleProcessModel>,
            reverse: bool,
        ) {
            if ProcessGeneral::has_tag(sort_order) {
                ProcessGeneral::sort(sort_order, processes, reverse)
            } else if ProcessCPU::has_tag(sort_order) {
                ProcessCPU::sort(sort_order, processes, reverse)
            } else if ProcessMem::has_tag(sort_order) {
                ProcessMem::sort(sort_order, processes, reverse)
            } else {
                ProcessIO::sort(sort_order, processes, reverse)
            }
        }
    };
}

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessGeneral {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[blink("SingleProcessModel$get_pid")]
    #[bttr(sort_tag = "ProcessOrders::Pid")]
    pub pid: Option<i32>,
    #[blink("SingleProcessModel$get_ppid")]
    #[bttr(sort_tag = "ProcessOrders::Ppid")]
    pub ppid: Option<i32>,
    #[blink("SingleProcessModel$get_state")]
    #[bttr(sort_tag = "ProcessOrders::State")]
    pub state: Option<procfs::PidState>,
    #[bttr(
        title = "CPU",
        width = 11,
        precision = 2,
        unit = "%",
        aggr = "SingleProcessModel: cpu?.user_pct? + cpu?.system_pct?",
        sort_tag = "ProcessOrders::CpuTotal",
        highlight_if = "is_cpu_significant($)"
    )]
    pub cpu: Option<f64>,
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    #[bttr(sort_tag = "ProcessOrders::CpuUser")]
    pub cpu_user_pct: Option<f64>,
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    #[bttr(sort_tag = "ProcessOrders::CpuSys")]
    pub cpu_system_pct: Option<f64>,
    #[blink("SingleProcessModel$mem?.get_rss_bytes")]
    #[bttr(sort_tag = "ProcessOrders::Rss")]
    pub mem_rss_bytes: Option<u64>,
    #[blink("SingleProcessModel$mem?.get_minorfaults_per_sec")]
    #[bttr(sort_tag = "ProcessOrders::MinorFaults")]
    pub mem_minorfaults_per_sec: Option<f64>,
    #[blink("SingleProcessModel$mem?.get_majorfaults_per_sec")]
    #[bttr(sort_tag = "ProcessOrders::MajorFaults")]
    pub mem_majorfaults_per_sec: Option<f64>,
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    #[bttr(sort_tag = "ProcessOrders::Read")]
    pub io_rbytes_per_sec: Option<f64>,
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    #[bttr(sort_tag = "ProcessOrders::Write")]
    pub io_wbytes_per_sec: Option<f64>,
    #[blink("SingleProcessModel$get_uptime_secs")]
    #[bttr(sort_tag = "ProcessOrders::UptimeSecs")]
    pub uptime_secs: Option<u64>,
    #[blink("SingleProcessModel$cpu?.get_num_threads")]
    #[bttr(sort_tag = "ProcessOrders::CpuNumThreads")]
    pub cpu_num_threads: Option<u64>,
    #[bttr(
        title = "RW Total",
        width = 10,
        aggr = "SingleProcessModel: io?.rbytes_per_sec? + io?.wbytes_per_sec?",
        sort_tag = "ProcessOrders::IoTotal",
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub disk: Option<f64>,
    #[blink("SingleProcessModel$get_cmdline")]
    #[bttr(sort_tag = "ProcessOrders::Cmdline")]
    pub cmdline: Option<String>,
}

impl ProcessTab for ProcessGeneral {
    impl_process_tab!(ProcessGeneral);

    fn get_field_line(&self, model: &SingleProcessModel) -> StyledString {
        self.get_field_line(&model, &model)
    }
}

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessCPU {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(title = "Comm", width = 30, sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(
        title = "CPU User",
        width = 11,
        precision = 2,
        unit = "%",
        sort_tag = "ProcessOrders::CpuUser"
    )]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    pub cpu_user: Option<f64>,
    #[bttr(
        title = "CPU Sys",
        width = 11,
        precision = 2,
        unit = "%",
        sort_tag = "ProcessOrders::CpuSys"
    )]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_sys: Option<f64>,
    #[bttr(
        title = "Threads",
        width = 11,
        sort_tag = "ProcessOrders::CpuNumThreads"
    )]
    #[blink("SingleProcessModel$cpu?.get_num_threads")]
    pub cpu_num_threads: Option<u64>,
    #[bttr(
        title = "CPU",
        width = 11,
        precision = 2,
        unit = "%",
        aggr = "SingleProcessModel: cpu?.user_pct? + cpu?.system_pct?",
        sort_tag = "ProcessOrders::CpuTotal"
    )]
    pub cpu_total: Option<f64>,
}

impl ProcessTab for ProcessCPU {
    impl_process_tab!(ProcessGeneral);

    fn get_field_line(&self, model: &SingleProcessModel) -> StyledString {
        self.get_field_line(&model, &model)
    }
}

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessMem {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(title = "Comm", width = 30, sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(
        title = "RSS",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        sort_tag = "ProcessOrders::Rss"
    )]
    #[blink("SingleProcessModel$mem?.get_rss_bytes")]
    pub mem_rss: Option<u64>,
    #[bttr(
        title = "Minflt",
        width = 11,
        precision = 2,
        sort_tag = "ProcessOrders::MinorFaults",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$mem?.get_minorfaults_per_sec")]
    pub mem_minorfaults: Option<f64>,
    #[bttr(
        title = "Majflt",
        width = 11,
        precision = 2,
        sort_tag = "ProcessOrders::MajorFaults",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$mem?.get_majorfaults_per_sec")]
    pub mem_majorfaults: Option<f64>,
}

impl ProcessTab for ProcessMem {
    impl_process_tab!(ProcessGeneral);

    fn get_field_line(&self, model: &SingleProcessModel) -> StyledString {
        self.get_field_line(&model)
    }
}

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessIO {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(title = "Comm", width = 30, sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(
        title = "Reads",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        sort_tag = "ProcessOrders::Read",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    pub io_read: Option<f64>,
    #[bttr(
        title = "Writes",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        sort_tag = "ProcessOrders::Write",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_write: Option<f64>,
    #[bttr(
        title = "RW Total",
        decorator = "convert_bytes($ as f64)",
        width = 11,
        aggr = "SingleProcessModel: io?.rbytes_per_sec? + io?.wbytes_per_sec?",
        sort_tag = "ProcessOrders::IoTotal",
        unit = "/s"
    )]
    pub io_total: Option<f64>,
}

impl ProcessTab for ProcessIO {
    impl_process_tab!(ProcessGeneral);

    fn get_field_line(&self, model: &SingleProcessModel) -> StyledString {
        self.get_field_line(&model, &model)
    }
}
