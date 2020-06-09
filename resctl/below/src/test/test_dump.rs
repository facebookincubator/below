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
use crate::dump::*;
use crate::util::convert_bytes;
use command::{GeneralOpt, OutputFormat, ProcField, SysField};
use get::Dget;
use print::Dprint;
use tmain::Dump;

use std::io;
use std::iter::FromIterator;
use std::path::PathBuf;

use serde_json::Value;

#[test]
fn test_tmain_init() {
    let mut opts: GeneralOpt = Default::default();
    let time = SystemTime::now();
    let advance = Advance::new(get_logger(), PathBuf::new(), time);
    let mut collector = Collector::new();
    let model = collector.update_model().expect("Fail to get model");

    // Since we are using the same function for field and title generation,
    // testing title should be enough if we don't care about the content.
    // case1: pick field and verify order
    opts.output_format = Some(OutputFormat::Csv);
    let mut sys_handle = system::System::new(opts, advance, time, None);
    let fields = Some(vec![SysField::Timestamp, SysField::Datetime]);
    sys_handle.init(fields.clone());
    assert_eq!(sys_handle.title_fns.len(), 2);
    assert_eq!(sys_handle.field_fns.len(), 2);
    let mut title_iter = sys_handle.title_fns.iter();
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Timestamp"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Datetime"
    );

    // case2: when default is set
    sys_handle.title_fns.clear();
    sys_handle.field_fns.clear();
    sys_handle.get_opts_mut().default = true;
    sys_handle.init(fields.clone());
    assert_eq!(sys_handle.title_fns.len(), 8);
    assert_eq!(sys_handle.field_fns.len(), 8);
    let mut title_iter = sys_handle.title_fns.iter();
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Datetime"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU Usage"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU User"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU Sys"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem Total"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem Free"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Reads"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Writes"
    );

    // case3: when everything is set
    sys_handle.title_fns.clear();
    sys_handle.field_fns.clear();
    sys_handle.get_opts_mut().default = true;
    sys_handle.get_opts_mut().everything = true;
    sys_handle.init(fields);
    assert!(sys_handle.get_opts().default);
    assert!(sys_handle.get_opts().detail);
    assert_eq!(sys_handle.title_fns.len(), 12);
    assert_eq!(sys_handle.field_fns.len(), 12);
    let mut title_iter = sys_handle.title_fns.iter();
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Datetime"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU Usage"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU User"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "CPU Sys"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem Total"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem Free"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem Anon"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Mem File"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Huge Page Total"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Huge Page Free"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Reads"
    );
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Writes"
    );

    // case4: test json dedup
    sys_handle.title_fns.clear();
    sys_handle.field_fns.clear();
    sys_handle.get_opts_mut().default = false;
    sys_handle.get_opts_mut().everything = false;
    sys_handle.get_opts_mut().output_format = Some(OutputFormat::Json);
    let fields = Some(vec![SysField::Timestamp, SysField::Timestamp]);
    sys_handle.init(fields);
    assert_eq!(sys_handle.title_fns.len(), 1);
    assert_eq!(sys_handle.field_fns.len(), 1);
    let mut title_iter = sys_handle.title_fns.iter();
    assert_eq!(
        title_iter.next().unwrap()(sys_handle.get_data(), &model.system),
        "Timestamp"
    );
}

#[test]
// Test correctness of system decoration
fn test_dump_sys_content() {
    let mut collector = Collector::new();
    collector.update_model().expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(get_logger(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut sys_handle = system::System::new(opts, advance, time, None);
    sys_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model().expect("Fail to get model");
    let jval = sys_handle.do_print_json(&model.system);

    let cpu = model
        .system
        .cpu
        .as_ref()
        .expect("Fail to get cpu from model.sys");
    assert_eq!(jval["CPU Usage"].as_str().unwrap(), cpu.get_usage_pct_str());
    assert_eq!(jval["CPU User"].as_str().unwrap(), cpu.get_user_pct_str());
    assert_eq!(jval["CPU Sys"].as_str().unwrap(), cpu.get_system_pct_str());

    let mem = model
        .system
        .mem
        .as_ref()
        .expect("Fail to get mem from model.sys");
    assert_eq!(jval["Mem Total"].as_str().unwrap(), mem.get_total_str());
    assert_eq!(jval["Mem Free"].as_str().unwrap(), mem.get_free_str());
    assert_eq!(jval["Mem Anon"].as_str().unwrap(), mem.get_anon_str());
    assert_eq!(jval["Mem File"].as_str().unwrap(), mem.get_file_str());

    let io = model
        .system
        .io
        .as_ref()
        .expect("Fail to get io from model.sys");
    assert_eq!(
        jval["Writes"].as_str().unwrap(),
        io.get_wbytes_per_sec_str()
    );
    assert_eq!(jval["Reads"].as_str().unwrap(), io.get_rbytes_per_sec_str());
}

struct StrIo {
    content: String,
}

impl StrIo {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
}

impl io::Write for StrIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let to_write = String::from_utf8(buf.to_vec()).unwrap();
        self.content += &to_write;
        Ok(to_write.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
// Test correctness of process decoration
// This test will also test JSON correctness.
fn test_dump_proc_content() {
    let mut collector = Collector::new();
    collector.update_model().expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(get_logger(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut proc_handle = process::Process::new(opts, advance, time, None);
    proc_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model().expect("Fail to get model");
    let mut proc_content = StrIo::new();
    let mut round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round)
        .expect("Fail to get json from iterate_exec");

    // verify json correctness
    assert!(!proc_content.content.is_empty());
    let jval: Value =
        serde_json::from_str(&proc_content.content).expect("Fail parse json of process dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let pid = value["Pid"].as_str().unwrap();
        let spm = model
            .process
            .processes
            .get(&pid.parse().unwrap())
            .expect("Json pid and spm pid not match");

        assert_eq!(value["Pid"].as_str().unwrap(), spm.get_pid_str());
        assert_eq!(value["Comm"].as_str().unwrap(), spm.get_comm_str());
        assert_eq!(value["State"].as_str().unwrap(), spm.get_state_str());
        assert_eq!(
            value["Uptime(sec)"].as_str().unwrap(),
            spm.get_uptime_secs_str()
        );
        assert_eq!(value["Cgroup"].as_str().unwrap(), spm.get_cgroup_str());

        let cpu = spm.cpu.as_ref().expect("SPM cpu is none");
        assert_eq!(value["User CPU"].as_str().unwrap(), cpu.get_user_pct_str());
        assert_eq!(value["Sys CPU"].as_str().unwrap(), cpu.get_system_pct_str());
        assert_eq!(
            value["Threads"].as_str().unwrap(),
            cpu.get_num_threads_str()
        );
        assert_eq!(
            value["CPU"].as_str().unwrap(),
            format!("{:.2}%", cpu.user_pct.unwrap() + cpu.system_pct.unwrap())
        );

        let mem = spm.mem.as_ref().expect("SPM mem is none");
        assert_eq!(value["RSS"].as_str().unwrap(), mem.get_rss_bytes_str());
        assert_eq!(
            value["Minflt"].as_str().unwrap(),
            mem.get_minorfaults_per_sec_str()
        );
        assert_eq!(
            value["Majflt"].as_str().unwrap(),
            mem.get_majorfaults_per_sec_str()
        );

        let io = spm.io.as_ref().expect("SPM io is none");
        assert_eq!(
            value["Reads"].as_str().unwrap(),
            io.get_rbytes_per_sec_str()
        );
        assert_eq!(
            value["Writes"].as_str().unwrap(),
            io.get_wbytes_per_sec_str()
        );
        assert_eq!(
            value["RW"].as_str().unwrap(),
            format!(
                "{}/s",
                convert_bytes(
                    io.rbytes_per_sec.unwrap_or_default() + io.wbytes_per_sec.unwrap_or_default()
                )
            )
        );
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
fn test_dump_proc_select() {
    let mut collector = Collector::new();
    collector.update_model().expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(get_logger(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut proc_handle = process::Process::new(opts, advance, time, Some(ProcField::Pid));
    proc_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model().expect("Fail to get model");
    proc_handle.get_opts_mut().filter =
        Some(model.process.processes.iter().last().unwrap().0.to_string());
    let mut proc_content = StrIo::new();
    let mut round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round)
        .expect("Fail to get json from iterate_exec");

    // test select filter
    let jval: Value =
        serde_json::from_str(&proc_content.content).expect("Fail parse json of process dump");
    assert_eq!(jval.as_array().unwrap().len(), 1);

    // test select rsort top
    proc_handle.get_opts_mut().sort = true;
    proc_handle.get_opts_mut().top = 5;
    proc_handle.get_opts_mut().filter = None;

    proc_content.content = String::new();
    round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round)
        .expect("Fail to get json from iterate_exec");

    assert_eq!(round, 5);
    let jval: Value =
        serde_json::from_str(&proc_content.content).expect("Fail parse json of process dump");

    let mut prev_id = 0;
    for item in jval.as_array().unwrap() {
        let pid = item["Pid"].as_str().unwrap();
        let cur_id = pid.parse::<i32>().unwrap();
        if prev_id > 0 {
            assert!(prev_id < cur_id, "prev_id: {}, cur_id: {}", prev_id, cur_id);
        }
        prev_id = cur_id;
    }

    // test select sort top
    proc_handle.get_opts_mut().sort = false;
    proc_handle.get_opts_mut().rsort = true;
    proc_content.content = String::new();
    round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round)
        .expect("Fail to get json from iterate_exec");

    assert_eq!(round, 5);
    let jval: Value =
        serde_json::from_str(&proc_content.content).expect("Fail parse json of process dump");

    prev_id = 0;
    for item in jval.as_array().unwrap() {
        let pid = item["Pid"].as_str().unwrap();
        let cur_id = pid.parse::<i32>().unwrap();
        if prev_id > 0 {
            assert!(prev_id > cur_id, "prev_id: {}, cur_id: {}", prev_id, cur_id);
        }
        prev_id = cur_id;
    }
}

fn traverse_cgroup_tree(model: &CgroupModel, jval: &mut Value) {
    assert_eq!(jval["Name"].as_str().unwrap(), model.get_name_str());

    if let Some(cpu) = model.cpu.as_ref() {
        assert_eq!(jval["CPU Usage"].as_str().unwrap(), cpu.get_usage_pct_str());
        assert_eq!(jval["CPU User"].as_str().unwrap(), cpu.get_user_pct_str());
        assert_eq!(jval["CPU Sys"].as_str().unwrap(), cpu.get_system_pct_str());
        assert_eq!(
            jval["Nr Period"].as_str().unwrap(),
            cpu.get_nr_periods_per_sec_str()
        );
        assert_eq!(
            jval["Nr Throttle"].as_str().unwrap(),
            cpu.get_nr_throttled_per_sec_str()
        );
        assert_eq!(
            jval["Throttle Pct"].as_str().unwrap(),
            cpu.get_throttled_pct_str()
        );
    }

    if let Some(mem) = model.memory.as_ref() {
        assert_eq!(jval["Mem Total"].as_str().unwrap(), mem.get_total_str());
        assert_eq!(
            jval["Mem Swap"].as_str().unwrap(),
            convert_bytes(mem.swap.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Mem Anon"].as_str().unwrap(),
            convert_bytes(mem.anon.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Mem File"].as_str().unwrap(),
            convert_bytes(mem.file.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Kernel Stack"].as_str().unwrap(),
            convert_bytes(mem.kernel_stack.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Mem Slab"].as_str().unwrap(),
            convert_bytes(mem.slab.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Mem Sock"].as_str().unwrap(),
            convert_bytes(mem.sock.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Mem Shmem"].as_str().unwrap(),
            convert_bytes(mem.shmem.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["File Mapped"].as_str().unwrap(),
            convert_bytes(mem.file_mapped.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["File Dirty"].as_str().unwrap(),
            convert_bytes(mem.file_dirty.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["File WB"].as_str().unwrap(),
            convert_bytes(mem.file_writeback.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Anon THP"].as_str().unwrap(),
            convert_bytes(mem.anon_thp.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Inactive Anon"].as_str().unwrap(),
            convert_bytes(mem.inactive_anon.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Active Anon"].as_str().unwrap(),
            convert_bytes(mem.active_anon.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Inactive File"].as_str().unwrap(),
            convert_bytes(mem.inactive_file.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Active File"].as_str().unwrap(),
            convert_bytes(mem.active_file.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Unevictable"].as_str().unwrap(),
            convert_bytes(mem.unevictable.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Slab Reclaimable"].as_str().unwrap(),
            convert_bytes(mem.slab_reclaimable.unwrap_or_default() as f64)
        );
        assert_eq!(
            jval["Slab Unreclaimable"].as_str().unwrap(),
            convert_bytes(mem.slab_unreclaimable.unwrap_or_default() as f64)
        );
        assert_eq!(jval["Pgfault"].as_str().unwrap(), mem.get_pgfault_str());
        assert_eq!(
            jval["Pgmajfault"].as_str().unwrap(),
            mem.get_pgmajfault_str()
        );
        assert_eq!(
            jval["Workingset Refault"].as_str().unwrap(),
            mem.get_workingset_refault_str()
        );
        assert_eq!(
            jval["Workingset Activate"].as_str().unwrap(),
            mem.get_workingset_activate_str()
        );
        assert_eq!(
            jval["Workingset Nodereclaim"].as_str().unwrap(),
            mem.get_workingset_nodereclaim_str()
        );
        assert_eq!(jval["Pgrefill"].as_str().unwrap(), mem.get_pgrefill_str());
        assert_eq!(jval["Pgscan"].as_str().unwrap(), mem.get_pgscan_str());
        assert_eq!(jval["Pgsteal"].as_str().unwrap(), mem.get_pgsteal_str());
        assert_eq!(
            jval["Pgactivate"].as_str().unwrap(),
            mem.get_pgactivate_str()
        );
        assert_eq!(
            jval["Pgdeactivate"].as_str().unwrap(),
            mem.get_pgdeactivate_str()
        );
        assert_eq!(
            jval["Pglazyfree"].as_str().unwrap(),
            mem.get_pglazyfree_str()
        );
        assert_eq!(
            jval["Pglazyfreed"].as_str().unwrap(),
            mem.get_pglazyfreed_str()
        );
        assert_eq!(
            jval["THP Fault Alloc"].as_str().unwrap(),
            mem.get_thp_fault_alloc_str()
        );
        assert_eq!(
            jval["THP Collapse Alloc"].as_str().unwrap(),
            mem.get_thp_collapse_alloc_str()
        );
    }

    if let Some(pressure) = model.pressure.as_ref() {
        assert_eq!(
            jval["CPU Pressure"].as_str().unwrap(),
            pressure.get_cpu_some_pct_str()
        );
        assert_eq!(
            jval["Memory Some Pressure"].as_str().unwrap(),
            format!("{:.2}%", pressure.memory_some_pct.unwrap_or_default())
        );
        assert_eq!(
            jval["Memory Pressure"].as_str().unwrap(),
            pressure.get_memory_full_pct_str()
        );
        assert_eq!(
            jval["I/O Some Pressure"].as_str().unwrap(),
            format!("{:.2}%", pressure.io_some_pct.unwrap_or_default())
        );
        assert_eq!(
            jval["I/O Pressure"].as_str().unwrap(),
            pressure.get_io_full_pct_str()
        );
    }

    if let Some(io) = model.io_total.as_ref() {
        assert_eq!(
            jval["RBytes"].as_str().unwrap(),
            io.get_rbytes_per_sec_str()
        );
        assert_eq!(
            jval["WBytes"].as_str().unwrap(),
            io.get_wbytes_per_sec_str()
        );
        assert_eq!(
            jval["R I/O"].as_str().unwrap(),
            format!("{}/s", convert_bytes(io.rios_per_sec.unwrap_or_default()))
        );
        assert_eq!(
            jval["W I/O"].as_str().unwrap(),
            format!("{}/s", convert_bytes(io.wios_per_sec.unwrap_or_default()))
        );
        assert_eq!(
            jval["DBytes"].as_str().unwrap(),
            format!("{}/s", convert_bytes(io.dbytes_per_sec.unwrap_or_default()))
        );
        assert_eq!(
            jval["D I/O"].as_str().unwrap(),
            format!("{}/s", convert_bytes(io.dios_per_sec.unwrap_or_default()))
        );
        assert_eq!(
            jval["RW Total"].as_str().unwrap(),
            format!(
                "{}/s",
                convert_bytes(
                    io.rbytes_per_sec.unwrap_or_default() + io.wbytes_per_sec.unwrap_or_default()
                )
            )
        );
    }

    let jval_children = jval["children"].as_array_mut().unwrap();
    let mut model_children = Vec::from_iter(&model.children);
    jval_children.truncate(2);
    model_children.truncate(2);

    model_children
        .iter_mut()
        .zip(jval_children.iter_mut())
        .for_each(|(model, jval)| traverse_cgroup_tree(model, jval));
}

#[test]
fn test_dump_cgroup_content() {
    let mut collector = Collector::new();
    collector.update_model().expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(get_logger(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut cgroup_handle = cgroup::Cgroup::new(opts, advance, time, None);
    cgroup_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model().expect("Fail to get model");
    let mut cgroup_content = StrIo::new();
    let mut round = 0;
    cgroup_handle
        .iterate_exec(&model, &mut cgroup_content, &mut round)
        .expect("Fail to get json from iterate_exec");

    // verify json correctness
    assert!(!cgroup_content.content.is_empty());
    let mut jval: Value =
        serde_json::from_str(&cgroup_content.content).expect("Fail parse json of process dump");
    traverse_cgroup_tree(&model.cgroup, &mut jval);
}
