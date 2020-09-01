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

#[derive(Default)]
pub struct ProcessModel {
    pub processes: BTreeMap<i32, SingleProcessModel>,
}

impl ProcessModel {
    pub fn new(sample: &procfs::PidMap, last: Option<(&procfs::PidMap, Duration)>) -> ProcessModel {
        let mut processes: BTreeMap<i32, SingleProcessModel> = BTreeMap::new();

        for (pid, pidinfo) in sample.iter() {
            processes.insert(
                *pid,
                SingleProcessModel::new(
                    &pidinfo,
                    last.and_then(|(p, d)| p.get(pid).map(|p| (p, d))),
                ),
            );
        }

        ProcessModel { processes }
    }
}

#[derive(BelowDecor, Default)]
pub struct SingleProcessModel {
    #[bttr(title = "Pid", width = 11)]
    pub pid: Option<i32>,
    #[bttr(title = "Ppid", width = 11)]
    pub ppid: Option<i32>,
    #[bttr(title = "Comm", width = 30)]
    pub comm: Option<String>,
    #[bttr(title = "State", width = 11)]
    pub state: Option<procfs::PidState>,
    #[bttr(title = "Uptime(sec)", width = 11)]
    pub uptime_secs: Option<u64>,
    #[bttr(
        title = "Cgroup",
        width = 50,
        decorator = "fold_string(&$, 50, 1, |c: char| c == '/')"
    )]
    pub cgroup: Option<String>,
    pub io: Option<ProcessIoModel>,
    pub mem: Option<ProcessMemoryModel>,
    pub cpu: Option<ProcessCpuModel>,
    #[bttr(title = "Cmdline", width = 50)]
    pub cmdline: Option<String>,
    #[bttr(title = "Exe Path")]
    pub exe_path: Option<String>,
}

impl SingleProcessModel {
    fn new(
        sample: &procfs::PidInfo,
        last: Option<(&procfs::PidInfo, Duration)>,
    ) -> SingleProcessModel {
        SingleProcessModel {
            pid: sample.stat.pid,
            ppid: sample.stat.ppid,
            comm: sample.stat.comm.clone(),
            state: sample.stat.state.clone(),
            uptime_secs: sample.stat.running_secs.map(|s| s as u64),
            cgroup: Some(sample.cgroup.clone()),
            io: last.map(|(l, d)| ProcessIoModel::new(&l.io, &sample.io, d)),
            mem: last.map(|(l, d)| ProcessMemoryModel::new(&l, &sample, d)),
            cpu: last.map(|(l, d)| ProcessCpuModel::new(&l.stat, &sample.stat, d)),
            cmdline: if let Some(cmd_vec) = sample.cmdline_vec.as_ref() {
                Some(cmd_vec.join(" "))
            } else {
                Some("?".into())
            },
            exe_path: sample.exe_path.clone(),
        }
    }
}

#[derive(Clone, BelowDecor, Default)]
pub struct ProcessIoModel {
    #[bttr(
        title = "Reads",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub rbytes_per_sec: Option<f64>,
    #[bttr(
        title = "Writes",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub wbytes_per_sec: Option<f64>,
}

impl ProcessIoModel {
    fn new(begin: &procfs::PidIo, end: &procfs::PidIo, delta: Duration) -> ProcessIoModel {
        ProcessIoModel {
            rbytes_per_sec: count_per_sec!(begin.rbytes, end.rbytes, delta),
            wbytes_per_sec: count_per_sec!(begin.wbytes, end.wbytes, delta),
        }
    }
}

#[derive(Clone, BelowDecor, Default)]
pub struct ProcessCpuModel {
    #[bttr(
        title = "CPU User",
        width = 11,
        precision = 2,
        unit = "%",
        highlight_if = "is_cpu_significant($)"
    )]
    pub user_pct: Option<f64>,
    #[bttr(
        title = "CPU Sys",
        width = 11,
        precision = 2,
        unit = "%",
        highlight_if = "is_cpu_significant($)"
    )]
    pub system_pct: Option<f64>,
    #[bttr(title = "Threads", width = 11)]
    pub num_threads: Option<u64>,
}

impl ProcessCpuModel {
    fn new(begin: &procfs::PidStat, end: &procfs::PidStat, delta: Duration) -> ProcessCpuModel {
        ProcessCpuModel {
            user_pct: usec_pct!(begin.user_usecs, end.user_usecs, delta),
            system_pct: usec_pct!(begin.system_usecs, end.system_usecs, delta),
            num_threads: end.num_threads.map(|t| t as u64),
        }
    }
}

#[derive(Clone, BelowDecor, Default)]
pub struct ProcessMemoryModel {
    #[bttr(title = "Minflt", width = 11, precision = 2, unit = "/s")]
    pub minorfaults_per_sec: Option<f64>,
    #[bttr(title = "Majflt", width = 11, precision = 2, unit = "/s")]
    pub majorfaults_per_sec: Option<f64>,
    #[bttr(
        title = "RSS",
        width = 11,
        decorator = "convert_bytes($ as f64)",
        cmp = true
    )]
    pub rss_bytes: Option<u64>,
    #[bttr(title = "VM Size", width = 11, decorator = "convert_bytes($ as f64)")]
    pub vm_size: Option<u64>,
    #[bttr(title = "Lock", width = 11, decorator = "convert_bytes($ as f64)")]
    pub lock: Option<u64>,
    #[bttr(title = "Pin", width = 11, decorator = "convert_bytes($ as f64)")]
    pub pin: Option<u64>,
    #[bttr(title = "Anon", width = 11, decorator = "convert_bytes($ as f64)")]
    pub anon: Option<u64>,
    #[bttr(title = "File", width = 11, decorator = "convert_bytes($ as f64)")]
    pub file: Option<u64>,
    #[bttr(title = "Shmem", width = 11, decorator = "convert_bytes($ as f64)")]
    pub shmem: Option<u64>,
    #[bttr(title = "PTE", width = 11, decorator = "convert_bytes($ as f64)")]
    pub pte: Option<u64>,
    #[bttr(title = "Swap", width = 11, decorator = "convert_bytes($ as f64)")]
    pub swap: Option<u64>,
    #[bttr(title = "Huge TLB", width = 11, decorator = "convert_bytes($ as f64)")]
    pub huge_tlb: Option<u64>,
}

impl ProcessMemoryModel {
    fn new(begin: &procfs::PidInfo, end: &procfs::PidInfo, delta: Duration) -> ProcessMemoryModel {
        ProcessMemoryModel {
            minorfaults_per_sec: count_per_sec!(begin.stat.minflt, end.stat.minflt, delta),
            majorfaults_per_sec: count_per_sec!(begin.stat.majflt, end.stat.majflt, delta),
            rss_bytes: end.stat.rss_bytes.map(|i| i as u64),
            vm_size: end.mem.vm_size.map(|i| i as u64),
            lock: end.mem.lock.map(|i| i as u64),
            pin: end.mem.pin.map(|i| i as u64),
            anon: end.mem.anon.map(|i| i as u64),
            file: end.mem.file.map(|i| i as u64),
            shmem: end.mem.shmem.map(|i| i as u64),
            pte: end.mem.pte.map(|i| i as u64),
            swap: end.mem.swap.map(|i| i as u64),
            huge_tlb: end.mem.huge_tlb.map(|i| i as u64),
        }
    }
}
