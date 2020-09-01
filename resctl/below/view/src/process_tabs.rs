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
    VmSize,
    Lock,
    Pin,
    Anon,
    File,
    Shmem,
    Pte,
    Swap,
    HugeTLB,
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
    fn get_process_field_line(
        &self,
        model: &SingleProcessModel,
        offset: Option<usize>,
    ) -> StyledString;
    fn sort_process(
        &self,
        sort_order: ProcessOrders,
        processes: &mut Vec<&SingleProcessModel>,
        reverse: bool,
    );

    fn get_rows(
        &mut self,
        state: &ProcessState,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        let unknown = "?".to_string();
        let process_model = state.get_model();
        let mut processes: Vec<&SingleProcessModel> =
            process_model.processes.iter().map(|(_, spm)| spm).collect();

        self.sort_process(state.sort_order, &mut processes, state.reverse);
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
            .map(|spm| {
                (
                    self.get_process_field_line(&spm, offset),
                    spm.pid.unwrap_or(0).to_string(),
                )
            })
            .collect()
    }
}

macro_rules! impl_process_tab {
    ($name:ident) => {
        impl ProcessTab for $name {
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

            fn sort_process(
                &self,
                sort_order: ProcessOrders,
                processes: &mut Vec<&SingleProcessModel>,
                reverse: bool,
            ) {
                self.sort(sort_order, processes, reverse)
            }

            fn get_process_field_line(
                &self,
                model: &SingleProcessModel,
                offset: Option<usize>,
            ) -> StyledString {
                match offset {
                    Some(offset) => {
                        let mut field_iter = self.get_field_vec(&model).into_iter();
                        let mut res = StyledString::new();
                        if let Some(name) = field_iter.next() {
                            res.append(name);
                            res.append_plain(" ")
                        };

                        field_iter.skip(offset).for_each(|item| {
                            res.append(item);
                            res.append_plain(" ")
                        });
                        res
                    }
                    _ => self.get_field_line(&model),
                }
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
        sort_tag = "ProcessOrders::CpuTotal",
        highlight_if = "is_cpu_significant($)"
    )]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
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
        sort_tag = "ProcessOrders::IoTotal",
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub disk: Option<f64>,
    #[blink("SingleProcessModel$get_cmdline")]
    #[bttr(sort_tag = "ProcessOrders::Cmdline")]
    pub cmdline: Option<String>,
}

impl_process_tab!(ProcessGeneral);

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessCPU {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(sort_tag = "ProcessOrders::CpuUser")]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    pub cpu_user: Option<f64>,
    #[bttr(sort_tag = "ProcessOrders::CpuSys")]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_sys: Option<f64>,
    #[bttr(sort_tag = "ProcessOrders::CpuNumThreads")]
    #[blink("SingleProcessModel$cpu?.get_num_threads")]
    pub cpu_num_threads: Option<u64>,
    #[bttr(
        title = "CPU",
        width = 11,
        precision = 2,
        unit = "%",
        sort_tag = "ProcessOrders::CpuTotal"
    )]
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_total: Option<f64>,
}

impl_process_tab!(ProcessCPU);

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessMem {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(sort_tag = "ProcessOrders::Rss")]
    #[blink("SingleProcessModel$mem?.get_rss_bytes")]
    pub mem_rss: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::VmSize")]
    #[blink("SingleProcessModel$mem?.get_vm_size")]
    pub vm_size: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Swap")]
    #[blink("SingleProcessModel$mem?.get_swap")]
    pub swap: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Anon")]
    #[blink("SingleProcessModel$mem?.get_anon")]
    pub anon: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::File")]
    #[blink("SingleProcessModel$mem?.get_file")]
    pub file: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Shmem")]
    #[blink("SingleProcessModel$mem?.get_shmem")]
    pub shmem: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Pte")]
    #[blink("SingleProcessModel$mem?.get_pte")]
    pub pte: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Lock")]
    #[blink("SingleProcessModel$mem?.get_lock")]
    pub lock: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::Pin")]
    #[blink("SingleProcessModel$mem?.get_pin")]
    pub pin: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::HugeTLB")]
    #[blink("SingleProcessModel$mem?.get_huge_tlb")]
    pub huge_tlb: Option<u64>,
    #[bttr(sort_tag = "ProcessOrders::MinorFaults")]
    #[blink("SingleProcessModel$mem?.get_minorfaults_per_sec")]
    pub mem_minorfaults: Option<f64>,
    #[bttr(sort_tag = "ProcessOrders::MajorFaults")]
    #[blink("SingleProcessModel$mem?.get_majorfaults_per_sec")]
    pub mem_majorfaults: Option<f64>,
}

impl_process_tab!(ProcessMem);

#[derive(BelowDecor, Default, Clone)]
pub struct ProcessIO {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(sort_tag = "ProcessOrders::Comm")]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    #[bttr(sort_tag = "ProcessOrders::Cgroup")]
    pub cgroup: Option<String>,
    #[bttr(sort_tag = "ProcessOrders::Read")]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    pub io_read: Option<f64>,
    #[bttr(sort_tag = "ProcessOrders::Write")]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_write: Option<f64>,
    #[bttr(
        title = "RW Total",
        decorator = "convert_bytes($ as f64)",
        width = 11,
        sort_tag = "ProcessOrders::IoTotal",
        unit = "/s"
    )]
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_total: Option<f64>,
}

impl_process_tab!(ProcessIO);
