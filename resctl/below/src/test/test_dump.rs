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

use std::io;
use std::path::PathBuf;

use serde_json::Value;

use super::*;
use command::{expand_fields, GeneralOpt, OutputFormat, ProcField};
use common::util::convert_bytes;
use dump::*;
use get::Dget;
use model::Queriable;
use print::HasRenderConfigForDump;
use tmain::{Dump, Dumper};

#[test]
// Test correctness of system decoration
fn test_dump_sys_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_SYSTEM_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let system_dumper = system::System::new(&opts, fields);

    // update model again to populate cpu and io data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut system_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    system_dumper
        .dump_model(&ctx, &model, &mut system_content, &mut round, false)
        .expect("Failed to dump cgroup model");
    let jval: Value =
        serde_json::from_str(&system_content.content).expect("Fail parse json of system dump");

    let cpu = model.system.total_cpu;
    assert_eq!(jval["Usage"].as_str().unwrap(), cpu.get_usage_pct_str());
    assert_eq!(jval["User"].as_str().unwrap(), cpu.get_user_pct_str());
    assert_eq!(jval["Idle"].as_str().unwrap(), cpu.get_idle_pct_str());
    assert_eq!(jval["System"].as_str().unwrap(), cpu.get_system_pct_str());
    assert_eq!(jval["Nice"].as_str().unwrap(), cpu.get_nice_pct_str());
    assert_eq!(jval["IOWait"].as_str().unwrap(), cpu.get_iowait_pct_str());
    assert_eq!(jval["Irq"].as_str().unwrap(), cpu.get_irq_pct_str());
    assert_eq!(jval["SoftIrq"].as_str().unwrap(), cpu.get_softirq_pct_str());
    assert_eq!(jval["Stolen"].as_str().unwrap(), cpu.get_stolen_pct_str());
    assert_eq!(jval["Guest"].as_str().unwrap(), cpu.get_guest_pct_str());
    assert_eq!(
        jval["Guest Nice"].as_str().unwrap(),
        cpu.get_guest_nice_pct_str()
    );

    let mem = model.system.mem;
    assert_eq!(jval["Total"].as_str().unwrap(), mem.get_total_str());
    assert_eq!(jval["Free"].as_str().unwrap(), mem.get_free_str());
    assert_eq!(jval["Available"].as_str().unwrap(), mem.get_available_str());
    assert_eq!(jval["Buffers"].as_str().unwrap(), mem.get_buffers_str());
    assert_eq!(jval["Cached"].as_str().unwrap(), mem.get_cached_str());
    assert_eq!(
        jval["Swap Cached"].as_str().unwrap(),
        mem.get_swap_cached_str()
    );
    assert_eq!(jval["Active"].as_str().unwrap(), mem.get_active_str());
    assert_eq!(jval["Inactive"].as_str().unwrap(), mem.get_inactive_str());
    assert_eq!(jval["Anon"].as_str().unwrap(), mem.get_anon_str());
    assert_eq!(jval["File"].as_str().unwrap(), mem.get_file_str());
    assert_eq!(
        jval["Unevictable"].as_str().unwrap(),
        mem.get_unevictable_str()
    );
    assert_eq!(jval["Mlocked"].as_str().unwrap(), mem.get_mlocked_str());
    assert_eq!(
        jval["Swap Total"].as_str().unwrap(),
        mem.get_swap_total_str()
    );
    assert_eq!(jval["Swap Free"].as_str().unwrap(), mem.get_swap_free_str());
    assert_eq!(jval["Dirty"].as_str().unwrap(), mem.get_dirty_str());
    assert_eq!(jval["Writeback"].as_str().unwrap(), mem.get_writeback_str());
    assert_eq!(
        jval["Anon Pages"].as_str().unwrap(),
        mem.get_anon_pages_str()
    );
    assert_eq!(jval["Mapped"].as_str().unwrap(), mem.get_mapped_str());
    assert_eq!(jval["Shmem"].as_str().unwrap(), mem.get_shmem_str());
    assert_eq!(
        jval["Kreclaimable"].as_str().unwrap(),
        mem.get_kreclaimable_str()
    );
    assert_eq!(jval["Slab"].as_str().unwrap(), mem.get_slab_str());
    assert_eq!(
        jval["Slab Reclaimable"].as_str().unwrap(),
        mem.get_slab_reclaimable_str()
    );
    assert_eq!(
        jval["Slab Unreclaimable"].as_str().unwrap(),
        mem.get_slab_unreclaimable_str()
    );
    assert_eq!(
        jval["Kernel Stack"].as_str().unwrap(),
        mem.get_kernel_stack_str()
    );
    assert_eq!(
        jval["Page Tables"].as_str().unwrap(),
        mem.get_page_tables_str()
    );
    assert_eq!(
        jval["Anon Huge Pages"].as_str().unwrap(),
        mem.get_anon_huge_pages_bytes_str()
    );
    assert_eq!(
        jval["Shmem Huge Pages"].as_str().unwrap(),
        mem.get_shmem_huge_pages_bytes_str()
    );
    assert_eq!(
        jval["File Huge Pages"].as_str().unwrap(),
        mem.get_file_huge_pages_bytes_str()
    );
    assert_eq!(
        jval["Total Huge Pages"].as_str().unwrap(),
        mem.get_total_huge_pages_bytes_str()
    );
    assert_eq!(
        jval["Free Huge Pages"].as_str().unwrap(),
        mem.get_free_huge_pages_bytes_str()
    );
    assert_eq!(
        jval["Huge Page Size"].as_str().unwrap(),
        mem.get_huge_page_size_str()
    );
    assert_eq!(jval["Cma Total"].as_str().unwrap(), mem.get_cma_total_str());
    assert_eq!(jval["Cma Free"].as_str().unwrap(), mem.get_cma_free_str());
    assert_eq!(
        jval["Vmalloc Total"].as_str().unwrap(),
        mem.get_vmalloc_total_str()
    );
    assert_eq!(
        jval["Vmalloc Used"].as_str().unwrap(),
        mem.get_vmalloc_used_str()
    );
    assert_eq!(
        jval["Vmalloc Chunk"].as_str().unwrap(),
        mem.get_vmalloc_chunk_str()
    );
    assert_eq!(
        jval["Direct Map 4K"].as_str().unwrap(),
        mem.get_direct_map_4k_str()
    );
    assert_eq!(
        jval["Direct Map 2M"].as_str().unwrap(),
        mem.get_direct_map_2m_str()
    );
    assert_eq!(
        jval["Direct Map 1G"].as_str().unwrap(),
        mem.get_direct_map_1g_str()
    );

    let vm = model.system.vm;
    assert_eq!(
        jval["Page In"].as_str().unwrap(),
        vm.get_pgpgin_per_sec_str()
    );
    assert_eq!(
        jval["Page Out"].as_str().unwrap(),
        vm.get_pgpgout_per_sec_str()
    );
    assert_eq!(
        jval["Swap In"].as_str().unwrap(),
        vm.get_pswpin_per_sec_str()
    );
    assert_eq!(
        jval["Swap Out"].as_str().unwrap(),
        vm.get_pswpout_per_sec_str()
    );
    assert_eq!(
        jval["Pgsteal Kswapd"].as_str().unwrap(),
        vm.get_pgsteal_kswapd_str()
    );
    assert_eq!(
        jval["Pgsteal Direct"].as_str().unwrap(),
        vm.get_pgsteal_direct_str()
    );
    assert_eq!(
        jval["Pgscan Kswapd"].as_str().unwrap(),
        vm.get_pgscan_kswapd_str()
    );
    assert_eq!(
        jval["Pgscan Direct"].as_str().unwrap(),
        vm.get_pgscan_direct_str()
    );
    assert_eq!(jval["OOM Kills"].as_str().unwrap(), vm.get_oom_kill_str());
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
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(logger.clone(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut proc_handle = process::Process::new(opts, advance, time, None);
    proc_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut proc_content = StrIo::new();
    let mut round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round, false)
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
        assert_eq!(value["Ppid"].as_str().unwrap(), spm.get_ppid_str());
        assert_eq!(value["Comm"].as_str().unwrap(), spm.get_comm_str());
        assert_eq!(value["Exe Path"].as_str().unwrap(), spm.get_exe_path_str());
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
        assert_eq!(
            value["RSS"].as_str().unwrap(),
            match mem.rss_bytes {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["VM Size"].as_str().unwrap(),
            match mem.vm_size {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Lock"].as_str().unwrap(),
            match mem.lock {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Pin"].as_str().unwrap(),
            match mem.pin {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Anon"].as_str().unwrap(),
            match mem.anon {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["File"].as_str().unwrap(),
            match mem.file {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Shmem"].as_str().unwrap(),
            match mem.shmem {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["PTE"].as_str().unwrap(),
            match mem.pte {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Swap"].as_str().unwrap(),
            match mem.swap {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
        assert_eq!(
            value["Huge TLB"].as_str().unwrap(),
            match mem.huge_tlb {
                Some(v) => convert_bytes(v as f64),
                None => "?".into(),
            }
        );
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
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");
    let time = SystemTime::now();
    let advance = Advance::new(logger.clone(), PathBuf::new(), time);

    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    let mut proc_handle = process::Process::new(opts, advance, time, Some(ProcField::Pid));
    proc_handle.init(None);

    // update model again to populate cpu and io data
    let model = collector.update_model(&logger).expect("Fail to get model");
    proc_handle.get_opts_mut().filter = Some(
        regex::Regex::new(&model.process.processes.iter().last().unwrap().0.to_string())
            .expect("Fail to construct regex"),
    );
    let mut proc_content = StrIo::new();
    let mut round = 0;
    proc_handle
        .iterate_exec(&model, &mut proc_content, &mut round, false)
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
        .iterate_exec(&model, &mut proc_content, &mut round, false)
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
        .iterate_exec(&model, &mut proc_content, &mut round, false)
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

fn traverse_cgroup_tree(model: &CgroupModel, jval: &Value) {
    for dump_field in expand_fields(command::DEFAULT_CGROUP_FIELDS, true) {
        match dump_field {
            DumpField::Common(_) => continue,
            DumpField::FieldId(field_id) => {
                let rc = CgroupModel::get_render_config_for_dump(&field_id);
                assert_eq!(
                    rc.render(model.query(&field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or("?")
                        .to_owned(),
                    "Model value and json value do not match for field: {}",
                    field_id.to_string(),
                );
            }
        }
    }
    model
        .children
        .iter()
        .zip(jval["children"].as_array().unwrap().iter())
        .take(2)
        .for_each(|(child_model, child_jval)| traverse_cgroup_tree(child_model, child_jval));
}

#[test]
fn test_dump_cgroup_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_CGROUP_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let cgroup_dumper = cgroup::Cgroup::new(&opts, None, fields);

    // update model again to populate cpu and io data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut cgroup_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    cgroup_dumper
        .dump_model(&ctx, &model, &mut cgroup_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!cgroup_content.content.is_empty());
    let mut jval: Value =
        serde_json::from_str(&cgroup_content.content).expect("Fail parse json of process dump");
    traverse_cgroup_tree(&model.cgroup, &mut jval);
}

#[test]
fn test_dump_cgroup_titles() {
    let titles = expand_fields(command::DEFAULT_CGROUP_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = CgroupModel::get_render_config_for_dump(&field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "Name",
        "Inode Number",
        "CPU Usage",
        "CPU User",
        "CPU Sys",
        "Nr Period",
        "Nr Throttled",
        "Throttled Pct",
        "Mem Total",
        "Mem Swap",
        "Mem Anon",
        "Mem File",
        "Kernel Stack",
        "Mem Slab",
        "Mem Sock",
        "Mem Shmem",
        "File Mapped",
        "File Dirty",
        "File WB",
        "Anon THP",
        "Inactive Anon",
        "Active Anon",
        "Inactive File",
        "Active File",
        "Unevictable",
        "Slab Reclaimable",
        "Slab Unreclaimable",
        "Pgfault",
        "Pgmajfault",
        "Workingset Refault",
        "Workingset Activate",
        "Workingset Nodereclaim",
        "Pgrefill",
        "Pgscan",
        "Pgsteal",
        "Pgactivate",
        "Pgdeactivate",
        "Pglazyfree",
        "Pglazyfreed",
        "THP Fault Alloc",
        "THP Collapse Alloc",
        "Memory High",
        "Events Low",
        "Events High",
        "Events Max",
        "Events OOM",
        "Events Kill",
        "RBytes",
        "WBytes",
        "R I/O",
        "W I/O",
        "DBytes",
        "D I/O",
        "RW Total",
        "CPU Pressure",
        "I/O Some Pressure",
        "I/O Pressure",
        "Memory Some Pressure",
        "Memory Pressure",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
// Test correctness of iface decoration
// This test will also test JSON correctness.
fn test_dump_iface_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_IFACE_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let iface_dumper = iface::Iface::new(&opts, None, fields);

    // update model again to populate net data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut iface_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    iface_dumper
        .dump_model(&ctx, &model, &mut iface_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!iface_content.content.is_empty());
    let jval: Value =
        serde_json::from_str(&iface_content.content).expect("Fail parse json of network dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let iface = value["Interface"].as_str().unwrap();
        let snm = model
            .network
            .interfaces
            .get(iface)
            .expect("Json iface and snm iface not match");

        assert_eq!(
            value["RX Bytes/s"].as_str().unwrap(),
            snm.get_rx_bytes_per_sec_str()
        );
        assert_eq!(
            value["TX Bytes/s"].as_str().unwrap(),
            snm.get_tx_bytes_per_sec_str()
        );
        assert_eq!(
            value["I/O Bytes/s"].as_str().unwrap(),
            snm.get_throughput_per_sec_str()
        );
        assert_eq!(
            value["RX Pkts/s"].as_str().unwrap(),
            snm.get_rx_packets_per_sec_str()
        );
        assert_eq!(
            value["TX Pkts/s"].as_str().unwrap(),
            snm.get_tx_packets_per_sec_str()
        );
        assert_eq!(
            value["Collisions"].as_str().unwrap(),
            snm.get_collisions_str()
        );
        assert_eq!(
            value["Multicast"].as_str().unwrap(),
            snm.get_multicast_str()
        );
        assert_eq!(value["RX Bytes"].as_str().unwrap(), snm.get_rx_bytes_str());
        assert_eq!(
            value["RX Compressed"].as_str().unwrap(),
            snm.get_rx_compressed_str()
        );
        assert_eq!(
            value["RX CRC Errors"].as_str().unwrap(),
            snm.get_rx_crc_errors_str()
        );
        assert_eq!(
            value["RX Dropped"].as_str().unwrap(),
            snm.get_rx_dropped_str()
        );
        assert_eq!(
            value["RX Errors"].as_str().unwrap(),
            snm.get_rx_errors_str()
        );
        assert_eq!(
            value["RX Fifo Errors"].as_str().unwrap(),
            snm.get_rx_fifo_errors_str()
        );
        assert_eq!(
            value["RX Frame Errors"].as_str().unwrap(),
            snm.get_rx_frame_errors_str()
        );
        assert_eq!(
            value["RX Length Errors"].as_str().unwrap(),
            snm.get_rx_length_errors_str()
        );
        assert_eq!(
            value["RX Missed Errors"].as_str().unwrap(),
            snm.get_rx_missed_errors_str()
        );
        assert_eq!(
            value["RX Nohandler"].as_str().unwrap(),
            snm.get_rx_nohandler_str()
        );
        assert_eq!(
            value["RX Over Errors"].as_str().unwrap(),
            snm.get_rx_over_errors_str()
        );
        assert_eq!(
            value["RX Packets"].as_str().unwrap(),
            snm.get_rx_packets_str()
        );
        assert_eq!(
            value["TX Aborted Errors"].as_str().unwrap(),
            snm.get_tx_aborted_errors_str()
        );
        assert_eq!(value["TX Bytes"].as_str().unwrap(), snm.get_tx_bytes_str());
        assert_eq!(
            value["TX Carrier Errors"].as_str().unwrap(),
            snm.get_tx_carrier_errors_str()
        );
        assert_eq!(
            value["TX Compressed"].as_str().unwrap(),
            snm.get_tx_compressed_str()
        );
        assert_eq!(
            value["TX Dropped"].as_str().unwrap(),
            snm.get_tx_dropped_str()
        );
        assert_eq!(
            value["TX Errors"].as_str().unwrap(),
            snm.get_tx_errors_str()
        );
        assert_eq!(
            value["TX Fifo Errors"].as_str().unwrap(),
            snm.get_tx_fifo_errors_str()
        );
        assert_eq!(
            value["TX Heartbeat Errors"].as_str().unwrap(),
            snm.get_tx_heartbeat_errors_str()
        );
        assert_eq!(
            value["TX Packets"].as_str().unwrap(),
            snm.get_tx_packets_str()
        );
        assert_eq!(
            value["TX Window Errors"].as_str().unwrap(),
            snm.get_tx_window_errors_str()
        );
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
// Test correctness of network decoration
// This test will also test JSON correctness.
fn test_dump_network_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_NETWORK_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let network_dumper = network::Network::new(&opts, fields);

    // update model again to populate net data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut network_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    network_dumper
        .dump_model(&ctx, &model, &mut network_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!network_content.content.is_empty());
    let jval: Value =
        serde_json::from_str(&network_content.content).expect("Fail parse json of network dump");

    let nm = model.network;
    // ip
    assert_eq!(
        jval["IpInPkts/s"].as_str().unwrap(),
        nm.ip.get_in_receives_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpForwPkts/s"].as_str().unwrap(),
        nm.ip.get_forwarding_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpForwDatagrams/s"].as_str().unwrap(),
        nm.ip.get_forw_datagrams_per_sec_str()
    );
    assert_eq!(
        jval["IpInDiscardPkts/s"].as_str().unwrap(),
        nm.ip.get_in_discards_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpInDeliversPkts/s"].as_str().unwrap(),
        nm.ip.get_in_delivers_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpOutReqs/s"].as_str().unwrap(),
        nm.ip.get_out_requests_per_sec_str()
    );
    assert_eq!(
        jval["IpOutDiscardPkts/s"].as_str().unwrap(),
        nm.ip.get_out_discards_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpOutNoRoutesPkts/s"].as_str().unwrap(),
        nm.ip.get_out_no_routes_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpInMcastPkts/s"].as_str().unwrap(),
        nm.ip.get_in_mcast_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpOutMcastPkts/s"].as_str().unwrap(),
        nm.ip.get_out_mcast_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpInBcastPkts/s"].as_str().unwrap(),
        nm.ip.get_in_bcast_pkts_per_sec_str()
    );
    assert_eq!(
        jval["IpOutBcastPkts/s"].as_str().unwrap(),
        nm.ip.get_out_bcast_pkts_per_sec_str()
    );
    //ip6
    assert_eq!(
        jval["Ip6InPkts/s"].as_str().unwrap(),
        nm.ip6.get_in_receives_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6InHdrErrs"].as_str().unwrap(),
        nm.ip6.get_in_hdr_errors_str()
    );
    assert_eq!(
        jval["Ip6InNoRoutesPkts/s"].as_str().unwrap(),
        nm.ip6.get_in_no_routes_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6InAddrErrs"].as_str().unwrap(),
        nm.ip6.get_in_addr_errors_str()
    );
    assert_eq!(
        jval["Ip6InDiscardsPkts/s"].as_str().unwrap(),
        nm.ip6.get_in_discards_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6InDeliversPkts/s"].as_str().unwrap(),
        nm.ip6.get_in_delivers_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6ForwDatagrams/s"].as_str().unwrap(),
        nm.ip6.get_out_forw_datagrams_per_sec_str()
    );
    assert_eq!(
        jval["Ip6OutReqs/s"].as_str().unwrap(),
        nm.ip6.get_out_requests_per_sec_str()
    );
    assert_eq!(
        jval["Ip6OutNoRoutesPkts/s"].as_str().unwrap(),
        nm.ip6.get_out_no_routes_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6InMcastPkts/s"].as_str().unwrap(),
        nm.ip6.get_in_mcast_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6OutMcastPkts/s"].as_str().unwrap(),
        nm.ip6.get_out_mcast_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Ip6InBcastOctets/s"].as_str().unwrap(),
        nm.ip6.get_in_bcast_octets_per_sec_str()
    );
    assert_eq!(
        jval["Ip6OutBcastOctets/s"].as_str().unwrap(),
        nm.ip6.get_out_bcast_octets_per_sec_str()
    );
    //Icmp
    assert_eq!(
        jval["IcmpInMsg/s"].as_str().unwrap(),
        nm.icmp.get_in_msgs_per_sec_str()
    );
    assert_eq!(
        jval["IcmpInErrs"].as_str().unwrap(),
        nm.icmp.get_in_errors_str()
    );
    assert_eq!(
        jval["IcmpInDestUnreachs"].as_str().unwrap(),
        nm.icmp.get_in_dest_unreachs_str()
    );
    assert_eq!(
        jval["IcmpOutMsg/s"].as_str().unwrap(),
        nm.icmp.get_out_msgs_per_sec_str()
    );
    assert_eq!(
        jval["IcmpOutErrs"].as_str().unwrap(),
        nm.icmp.get_out_errors_str()
    );
    assert_eq!(
        jval["IcmpOutDestUnreachs"].as_str().unwrap(),
        nm.icmp.get_out_dest_unreachs_str()
    );
    //Icmp6
    assert_eq!(
        jval["Icmp6InMsg/s"].as_str().unwrap(),
        nm.icmp6.get_in_msgs_per_sec_str()
    );
    assert_eq!(
        jval["Icmp6InErrs"].as_str().unwrap(),
        nm.icmp6.get_in_errors_str()
    );
    assert_eq!(
        jval["Icmp6InDestUnreachs"].as_str().unwrap(),
        nm.icmp6.get_in_dest_unreachs_str()
    );
    assert_eq!(
        jval["Icmp6OutMsg/s"].as_str().unwrap(),
        nm.icmp6.get_out_msgs_per_sec_str()
    );
    assert_eq!(
        jval["Icmp6OutErrs"].as_str().unwrap(),
        nm.icmp6.get_out_errors_str()
    );
    assert_eq!(
        jval["Icmp6OutDestUnreachs"].as_str().unwrap(),
        nm.icmp6.get_out_dest_unreachs_str()
    );
}

#[test]
// Test correctness of transport decoration
// This test will also test JSON correctness.
fn test_dump_transport_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_TRANSPORT_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let transport_dumper = transport::Transport::new(&opts, fields);

    // update model again to populate net data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut transport_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    transport_dumper
        .dump_model(&ctx, &model, &mut transport_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!transport_content.content.is_empty());
    let jval: Value =
        serde_json::from_str(&transport_content.content).expect("Fail parse json of network dump");

    let nm = model.network;
    // ip
    assert_eq!(
        jval["TcpActiveOpens/s"].as_str().unwrap(),
        nm.tcp.get_active_opens_per_sec_str()
    );
    assert_eq!(
        jval["TcpPassiveOpens/s"].as_str().unwrap(),
        nm.tcp.get_passive_opens_per_sec_str()
    );
    assert_eq!(
        jval["TcpAttemptFails/s"].as_str().unwrap(),
        nm.tcp.get_attempt_fails_per_sec_str()
    );
    assert_eq!(
        jval["TcpEstabResets/s"].as_str().unwrap(),
        nm.tcp.get_estab_resets_per_sec_str()
    );
    assert_eq!(
        jval["CurEstabConn"].as_str().unwrap(),
        nm.tcp.get_curr_estab_conn_str()
    );
    assert_eq!(
        jval["TcpInSegs/s"].as_str().unwrap(),
        nm.tcp.get_in_segs_per_sec_str()
    );
    assert_eq!(
        jval["TcpOutSegs/s"].as_str().unwrap(),
        nm.tcp.get_out_segs_per_sec_str()
    );
    assert_eq!(
        jval["TcpRetransSegs/s"].as_str().unwrap(),
        nm.tcp.get_retrans_segs_per_sec_str()
    );
    assert_eq!(
        jval["TcpRetransSegs"].as_str().unwrap(),
        nm.tcp.get_retrans_segs_str()
    );
    assert_eq!(
        jval["TcpInErrors"].as_str().unwrap(),
        nm.tcp.get_in_errs_str()
    );
    assert_eq!(
        jval["TcpOutRsts/s"].as_str().unwrap(),
        nm.tcp.get_out_rsts_per_sec_str()
    );
    assert_eq!(
        jval["TcpInCsumErrors"].as_str().unwrap(),
        nm.tcp.get_in_csum_errors_str()
    );
    assert_eq!(
        jval["UdpInPkts/s"].as_str().unwrap(),
        nm.udp.get_in_datagrams_pkts_per_sec_str()
    );
    assert_eq!(
        jval["UdpNoPorts"].as_str().unwrap(),
        nm.udp.get_no_ports_str()
    );
    assert_eq!(
        jval["UdpInErrs"].as_str().unwrap(),
        nm.udp.get_in_errors_str()
    );
    assert_eq!(
        jval["UdpOutPkts/s"].as_str().unwrap(),
        nm.udp.get_out_datagrams_pkts_per_sec_str()
    );
    assert_eq!(
        jval["UdpRcvbufErrs"].as_str().unwrap(),
        nm.udp.get_rcvbuf_errors_str()
    );
    assert_eq!(
        jval["UdpSndBufErrs"].as_str().unwrap(),
        nm.udp.get_sndbuf_errors_str()
    );
    assert_eq!(
        jval["UdpIgnoredMulti"].as_str().unwrap(),
        nm.udp.get_ignored_multi_str()
    );
    assert_eq!(
        jval["Udp6InPkts/s"].as_str().unwrap(),
        nm.udp6.get_in_datagrams_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Udp6NoPorts"].as_str().unwrap(),
        nm.udp6.get_no_ports_str()
    );
    assert_eq!(
        jval["Udp6InErrs"].as_str().unwrap(),
        nm.udp6.get_in_errors_str()
    );
    assert_eq!(
        jval["Udp6OutPkts/s"].as_str().unwrap(),
        nm.udp6.get_out_datagrams_pkts_per_sec_str()
    );
    assert_eq!(
        jval["Udp6RcvbufErrs"].as_str().unwrap(),
        nm.udp6.get_rcvbuf_errors_str()
    );
    assert_eq!(
        jval["Udp6SndBufErrs"].as_str().unwrap(),
        nm.udp6.get_sndbuf_errors_str()
    );
    assert_eq!(
        jval["Udp6InCsumErrs"].as_str().unwrap(),
        nm.udp6.get_in_csum_errors_str()
    );
    assert_eq!(
        jval["Udp6IgnoredMulti"].as_str().unwrap(),
        nm.udp6.get_ignored_multi_str()
    );
}

#[test]
// Test correctness of disk decoration
// This test will also test JSON correctness.
fn test_dump_disk_content() {
    let mut collector = Collector::new(get_dummy_exit_data());
    let logger = get_logger();
    collector.update_model(&logger).expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_DISK_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let disk_dumper = disk::Disk::new(&opts, None, fields);

    // update model again to populate disk data
    let model = collector.update_model(&logger).expect("Fail to get model");
    let mut disk_content = StrIo::new();
    let mut round = 0;
    let ctx = CommonFieldContext { timestamp: 0 };
    disk_dumper
        .dump_model(&ctx, &model, &mut disk_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!disk_content.content.is_empty());
    let jval: Value =
        serde_json::from_str(&disk_content.content).expect("Fail parse json of disk dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let name = value["Name"].as_str().unwrap();
        let sdm = model
            .system
            .disks
            .get(name)
            .expect("Json pid and sdm pid not match");

        assert_eq!(value["Name"].as_str().unwrap(), sdm.get_name_str());
        assert_eq!(
            value["Read"].as_str().unwrap(),
            sdm.get_read_bytes_per_sec_str()
        );
        assert_eq!(
            value["Write"].as_str().unwrap(),
            sdm.get_write_bytes_per_sec_str()
        );
        assert_eq!(
            value["Discard"].as_str().unwrap(),
            sdm.get_discard_bytes_per_sec_str()
        );
        assert_eq!(
            value["Disk"].as_str().unwrap(),
            sdm.get_disk_total_bytes_per_sec_str()
        );
        assert_eq!(
            value["Read Completed"].as_str().unwrap(),
            sdm.get_read_completed_str()
        );
        assert_eq!(
            value["Read Merged"].as_str().unwrap(),
            sdm.get_read_merged_str()
        );
        assert_eq!(
            value["Read Sectors"].as_str().unwrap(),
            sdm.get_read_sectors_str()
        );
        assert_eq!(
            value["Time Spend Read"].as_str().unwrap(),
            sdm.get_time_spend_read_ms_str()
        );
        assert_eq!(
            value["Write Completed"].as_str().unwrap(),
            sdm.get_write_completed_str()
        );
        assert_eq!(
            value["Write Merged"].as_str().unwrap(),
            sdm.get_write_merged_str()
        );
        assert_eq!(
            value["Write Sectors"].as_str().unwrap(),
            sdm.get_write_sectors_str()
        );
        assert_eq!(
            value["Time Spend Write"].as_str().unwrap(),
            sdm.get_time_spend_write_ms_str()
        );
        assert_eq!(
            value["Discard Completed"].as_str().unwrap(),
            sdm.get_discard_completed_str()
        );
        assert_eq!(
            value["Discard Merged"].as_str().unwrap(),
            sdm.get_discard_merged_str()
        );
        assert_eq!(
            value["Discard Sectors"].as_str().unwrap(),
            sdm.get_discard_sectors_str()
        );
        assert_eq!(
            value["Time Spend Discard"].as_str().unwrap(),
            sdm.get_time_spend_discard_ms_str()
        );
        assert_eq!(value["Major"].as_str().unwrap(), sdm.get_major_str());
        assert_eq!(value["Minor"].as_str().unwrap(), sdm.get_minor_str());
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
fn test_parse_pattern() {
    let tempdir = TempDir::new("below_dump_pattern").expect("Failed to create temp dir");
    let path = tempdir.path().join("dumprc");

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open dumprc in tempdir");
    let dumprc_str = r#"
[system]
demacia = ["datetime", "os_release"]

[process]
proc = ["datetime", "mem_anon"]
"#;
    file.write_all(dumprc_str.as_bytes())
        .expect("Faild to write temp dumprc file during testing ignore");
    file.flush().expect("Failed to flush during testing ignore");

    let sys_res = parse_pattern::<command::SystemOptionField>(
        path.to_string_lossy().to_string(),
        "demacia".into(),
        "system",
    )
    .expect("Failed to parse system pattern");

    assert_eq!(
        sys_res[0],
        command::SystemOptionField::Unit(SystemField::Common(CommonField::Datetime))
    );
    assert_eq!(
        sys_res[1],
        command::SystemOptionField::Unit(SystemField::FieldId(
            model::SystemModelFieldId::OsRelease
        ))
    );

    let proc_res =
        parse_pattern::<ProcField>(path.to_string_lossy().to_string(), "proc".into(), "process")
            .expect("Failed to parse process pattern");

    assert_eq!(proc_res[0], ProcField::Datetime);
    assert_eq!(proc_res[1], ProcField::Anon);
}
