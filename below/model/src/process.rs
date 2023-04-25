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

/// Folds two optionals together with either `+` operator or provided closure
macro_rules! fold_optionals {
    ($left:expr, $right:expr) => {
        fold_optionals!($left, $right, |l, r| l + r)
    };

    ($left:expr, $right:expr, $f:expr) => {
        match ($left, $right) {
            (Some(l), Some(r)) => Some($f(l, r)),
            (Some(l), None) => Some(l.clone()),
            (None, Some(r)) => Some(r.clone()),
            (None, None) => None,
        }
    };
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct ProcessModel {
    #[queriable(subquery)]
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

impl Nameable for ProcessModel {
    fn name() -> &'static str {
        "process"
    }
}

#[derive(Default, Clone, Serialize, Deserialize, below_derive::Queriable)]
pub struct SingleProcessModel {
    pub pid: Option<i32>,
    pub ppid: Option<i32>,
    pub ns_tgid: Option<Vec<u32>>,
    pub comm: Option<String>,
    pub state: Option<procfs::PidState>,
    pub uptime_secs: Option<u64>,
    pub cgroup: Option<String>,
    #[queriable(subquery)]
    pub io: Option<ProcessIoModel>,
    #[queriable(subquery)]
    pub mem: Option<ProcessMemoryModel>,
    #[queriable(subquery)]
    pub cpu: Option<ProcessCpuModel>,
    pub cmdline: Option<String>,
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
            ns_tgid: sample
                .status
                .ns_tgid
                .as_ref()
                // Skip the first item as it is always the same as pid
                .map(|v| v.iter().skip(1).cloned().collect()),
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

    /// Sums stats between two process models together, None'ing out fields that semantically
    /// cannot be summed
    pub fn fold(left: &SingleProcessModel, right: &SingleProcessModel) -> SingleProcessModel {
        SingleProcessModel {
            pid: None,
            ppid: None,
            ns_tgid: None,
            comm: None,
            state: None,
            // 80% sure it should be None here. Don't know what someone can infer from summed uptime
            uptime_secs: None,
            cgroup: None,
            io: fold_optionals!(&left.io, &right.io, ProcessIoModel::fold),
            mem: fold_optionals!(&left.mem, &right.mem, ProcessMemoryModel::fold),
            cpu: fold_optionals!(&left.cpu, &right.cpu, ProcessCpuModel::fold),
            cmdline: None,
            exe_path: None,
        }
    }
}

impl Nameable for SingleProcessModel {
    fn name() -> &'static str {
        "process"
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct ProcessIoModel {
    pub rbytes_per_sec: Option<f64>,
    pub wbytes_per_sec: Option<f64>,
    pub rwbytes_per_sec: Option<f64>,
}

impl ProcessIoModel {
    fn new(begin: &procfs::PidIo, end: &procfs::PidIo, delta: Duration) -> ProcessIoModel {
        let rbytes_per_sec = count_per_sec!(begin.rbytes, end.rbytes, delta);
        let wbytes_per_sec = count_per_sec!(begin.wbytes, end.wbytes, delta);
        let rwbytes_per_sec = Some(
            rbytes_per_sec.clone().unwrap_or_default() + wbytes_per_sec.clone().unwrap_or_default(),
        );
        ProcessIoModel {
            rbytes_per_sec,
            wbytes_per_sec,
            rwbytes_per_sec,
        }
    }

    /// See `SingleProcessModel::fold`
    pub fn fold(left: &ProcessIoModel, right: &ProcessIoModel) -> ProcessIoModel {
        ProcessIoModel {
            rbytes_per_sec: fold_optionals!(left.rbytes_per_sec, right.rbytes_per_sec),
            wbytes_per_sec: fold_optionals!(left.wbytes_per_sec, right.wbytes_per_sec),
            rwbytes_per_sec: fold_optionals!(left.rwbytes_per_sec, right.rwbytes_per_sec),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct ProcessCpuModel {
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
    pub num_threads: Option<u64>,
}

impl ProcessCpuModel {
    fn new(begin: &procfs::PidStat, end: &procfs::PidStat, delta: Duration) -> ProcessCpuModel {
        let user_pct = usec_pct!(begin.user_usecs, end.user_usecs, delta);
        let system_pct = usec_pct!(begin.system_usecs, end.system_usecs, delta);
        let usage_pct = collector::opt_add(user_pct.clone(), system_pct.clone());
        ProcessCpuModel {
            usage_pct,
            user_pct,
            system_pct,
            num_threads: end.num_threads.map(|t| t as u64),
        }
    }

    /// See `SingleProcessModel::fold`
    pub fn fold(left: &ProcessCpuModel, right: &ProcessCpuModel) -> ProcessCpuModel {
        ProcessCpuModel {
            usage_pct: fold_optionals!(left.usage_pct, right.usage_pct),
            user_pct: fold_optionals!(left.user_pct, right.user_pct),
            system_pct: fold_optionals!(left.system_pct, right.system_pct),
            num_threads: fold_optionals!(left.num_threads, right.num_threads),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct ProcessMemoryModel {
    pub minorfaults_per_sec: Option<f64>,
    pub majorfaults_per_sec: Option<f64>,
    pub rss_bytes: Option<u64>,
    pub vm_size: Option<u64>,
    pub lock: Option<u64>,
    pub pin: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub shmem: Option<u64>,
    pub pte: Option<u64>,
    pub swap: Option<u64>,
    pub huge_tlb: Option<u64>,
}

impl ProcessMemoryModel {
    fn new(begin: &procfs::PidInfo, end: &procfs::PidInfo, delta: Duration) -> ProcessMemoryModel {
        ProcessMemoryModel {
            minorfaults_per_sec: count_per_sec!(begin.stat.minflt, end.stat.minflt, delta),
            majorfaults_per_sec: count_per_sec!(begin.stat.majflt, end.stat.majflt, delta),
            rss_bytes: end.stat.rss_bytes,
            vm_size: end.status.vm_size,
            lock: end.status.lock,
            pin: end.status.pin,
            anon: end.status.anon,
            file: end.status.file,
            shmem: end.status.shmem,
            pte: end.status.pte,
            swap: end.status.swap,
            huge_tlb: end.status.huge_tlb,
        }
    }

    /// See `SingleProcessModel::fold`
    pub fn fold(left: &ProcessMemoryModel, right: &ProcessMemoryModel) -> ProcessMemoryModel {
        ProcessMemoryModel {
            minorfaults_per_sec: fold_optionals!(
                left.minorfaults_per_sec,
                right.minorfaults_per_sec
            ),
            majorfaults_per_sec: fold_optionals!(
                left.majorfaults_per_sec,
                right.majorfaults_per_sec
            ),
            rss_bytes: fold_optionals!(left.rss_bytes, right.rss_bytes),
            vm_size: fold_optionals!(left.vm_size, right.vm_size),
            lock: fold_optionals!(left.lock, right.lock),
            pin: fold_optionals!(left.pin, right.pin),
            anon: fold_optionals!(left.anon, right.anon),
            file: fold_optionals!(left.file, right.file),
            shmem: fold_optionals!(left.shmem, right.shmem),
            pte: fold_optionals!(left.pte, right.pte),
            swap: fold_optionals!(left.swap, right.swap),
            huge_tlb: fold_optionals!(left.huge_tlb, right.huge_tlb),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn query_model() {
        let model_json = r#"
        {
            "processes": {
                "1": {
                    "pid": 1,
                    "comm": "systemd"
                }
            }
        }
        "#;
        let model: ProcessModel = serde_json::from_str(model_json).unwrap();
        assert_eq!(
            model.query(&ProcessModelFieldId::from_str("processes.1.comm").unwrap()),
            Some(Field::Str("systemd".to_owned()))
        );
    }
}
