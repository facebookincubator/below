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

use std::collections::BTreeMap;
use std::time::Duration;

use command::GeneralOpt;
use command::OutputFormat;
use command::expand_fields;
use common::logutil::get_logger;
use model::Collector;
use model::Queriable;
use render::HasRenderConfigForDump;
use serde_json::Value;
use tempfile::TempDir;
use tmain::Dumper;

use super::*;

#[test]
// Test correctness of system decoration
fn test_dump_sys_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let mut fields = command::expand_fields(command::DEFAULT_SYSTEM_FIELDS, true);
    for subquery_id in enum_iterator::all::<model::SingleCpuModelFieldId>() {
        fields.push(DumpField::FieldId(model::SystemModelFieldId::Cpus(
            model::BTreeMapFieldId::new(Some(31), subquery_id),
        )));
    }
    opts.output_format = Some(OutputFormat::Json);
    let system_dumper = system::System::new(&opts, fields.clone());

    // update model again to populate cpu and io data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut system_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    system_dumper
        .dump_model(&ctx, &model, &mut system_content, &mut round, false)
        .expect("Failed to dump system model");

    // verify json correctness
    assert!(!system_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&system_content).expect("Fail parse json of system dump");

    for dump_field in fields.iter() {
        match dump_field {
            DumpField::Common(_) => continue,
            DumpField::FieldId(field_id) => {
                let rc = model::SystemModel::get_render_config_for_dump(field_id);
                assert_eq!(
                    rc.render(model.system.query(field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false)
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {field_id}",
                );
            }
        }
    }
}

#[test]
fn test_dump_sys_titles() {
    let titles = expand_fields(command::DEFAULT_SYSTEM_FIELDS, true)
        .into_iter()
        .chain(
            enum_iterator::all::<model::SingleCpuModelFieldId>().map(|subquery_id| {
                DumpField::FieldId(model::SystemModelFieldId::Cpus(
                    model::BTreeMapFieldId::new(Some(31), subquery_id),
                ))
            }),
        )
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SystemModel::get_render_config_for_dump(&field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "Hostname",
        "Usage",
        "User",
        "System",
        "Idle",
        "Nice",
        "IOWait",
        "Irq",
        "SoftIrq",
        "Stolen",
        "Guest",
        "Guest Nice",
        "Total",
        "Free",
        "Available",
        "Buffers",
        "Cached",
        "Swap Cached",
        "Active",
        "Inactive",
        "Anon",
        "File",
        "Unevictable",
        "Mlocked",
        "Swap Total",
        "Swap Free",
        "Dirty",
        "Writeback",
        "Anon Pages",
        "Mapped",
        "Shmem",
        "Kreclaimable",
        "Slab",
        "Slab Reclaimable",
        "Slab Unreclaimable",
        "Kernel Stack",
        "Page Tables",
        "Anon Huge Pages",
        "Shmem Huge Pages",
        "File Huge Pages",
        "Hugetlb",
        "Cma Total",
        "Cma Free",
        "Vmalloc Total",
        "Vmalloc Used",
        "Vmalloc Chunk",
        "Direct Map 4K",
        "Direct Map 2M",
        "Direct Map 1G",
        "Page In",
        "Page Out",
        "Swap In",
        "Swap Out",
        "Pgsteal Kswapd",
        "Pgsteal Direct",
        "Pgscan Kswapd",
        "Pgscan Direct",
        "OOM Kills",
        "Kernel Version",
        "OS Release",
        "Total Interrupts",
        "Context Switches",
        "Boot Time Epoch",
        "Total Procs",
        "Running Procs",
        "Blocked Procs",
        "CPU 31 Idx",
        "CPU 31 Usage",
        "CPU 31 User",
        "CPU 31 System",
        "CPU 31 Idle",
        "CPU 31 Nice",
        "CPU 31 IOWait",
        "CPU 31 Irq",
        "CPU 31 SoftIrq",
        "CPU 31 Stolen",
        "CPU 31 Guest",
        "CPU 31 Guest Nice",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
// Test correctness of process decoration
// This test will also test JSON correctness.
fn test_dump_process_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_PROCESS_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let process_dumper = process::Process::new(&opts, None, fields.clone());

    // update model again to populate cpu and io data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut process_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    process_dumper
        .dump_model(&ctx, &model, &mut process_content, &mut round, false)
        .expect("Failed to dump process model");

    // verify json correctness
    assert!(!process_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&process_content).expect("Fail parse json of process dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let pid = value["Pid"].as_str().unwrap();
        let spm = model
            .process
            .processes
            .get(&pid.parse().unwrap())
            .expect("Json pid and spm pid not match");

        for dump_field in fields.iter() {
            match dump_field {
                DumpField::Common(_) => continue,
                DumpField::FieldId(field_id) => {
                    let rc = model::SingleProcessModel::get_render_config_for_dump(field_id);
                    assert_eq!(
                        rc.render(spm.query(field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false)
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {field_id}",
                    );
                }
            }
        }
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
fn test_dump_proc_titles() {
    let titles = expand_fields(command::DEFAULT_PROCESS_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleProcessModel::get_render_config_for_dump(field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "Pid",
        "Ppid",
        "Comm",
        "State",
        "CPU",
        "User CPU",
        "Sys CPU",
        "Threads",
        "Processor",
        "Minflt",
        "Majflt",
        "RSS",
        "VM Size",
        "Lock",
        "Pin",
        "Anon",
        "File",
        "Shmem",
        "PTE",
        "Swap",
        "Huge TLB",
        "Reads",
        "Writes",
        "RW",
        "Uptime(sec)",
        "Cgroup",
        "Cmdline",
        "Exe Path",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
fn test_dump_proc_select() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");
    // update model again to populate cpu and io data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let fields = command::expand_fields(command::DEFAULT_PROCESS_FIELDS, true);
    let mut opts = GeneralOpt {
        everything: true,
        output_format: Some(OutputFormat::Json),
        filter: Some(
            regex::Regex::new(&model.process.processes.iter().last().unwrap().0.to_string())
                .expect("Fail to construct regex"),
        ),
        ..Default::default()
    };
    let process_dumper = process::Process::new(
        &opts,
        Some(model::SingleProcessModelFieldId::Pid),
        fields.clone(),
    );

    let mut process_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    process_dumper
        .dump_model(&ctx, &model, &mut process_content, &mut round, false)
        .expect("Failed to dump process model");

    // test select filter
    let jval: Value =
        serde_json::from_slice(&process_content).expect("Fail parse json of process dump");
    assert_eq!(jval.as_array().unwrap().len(), 1);

    // test select rsort top
    opts.sort = true;
    opts.top = 5;
    opts.filter = None;
    let process_dumper = process::Process::new(
        &opts,
        Some(model::SingleProcessModelFieldId::Pid),
        fields.clone(),
    );

    process_content = Vec::new();
    round = 0;
    process_dumper
        .dump_model(&ctx, &model, &mut process_content, &mut round, false)
        .expect("Failed to dump process model");

    assert_eq!(round, 5);
    let jval: Value =
        serde_json::from_slice(&process_content).expect("Fail parse json of process dump");

    let mut prev_id = 0;
    for item in jval.as_array().unwrap() {
        let pid = item["Pid"].as_str().unwrap();
        let cur_id = pid.parse::<i32>().unwrap();
        if prev_id > 0 {
            assert!(prev_id < cur_id, "prev_id: {prev_id}, cur_id: {cur_id}");
        }
        prev_id = cur_id;
    }

    // test select sort top
    opts.sort = false;
    opts.rsort = true;
    let process_dumper =
        process::Process::new(&opts, Some(model::SingleProcessModelFieldId::Pid), fields);

    process_content = Vec::new();
    round = 0;
    process_dumper
        .dump_model(&ctx, &model, &mut process_content, &mut round, false)
        .expect("Failed to dump process model");

    assert_eq!(round, 5);
    let jval: Value =
        serde_json::from_slice(&process_content).expect("Fail parse json of process dump");

    prev_id = 0;
    for item in jval.as_array().unwrap() {
        let pid = item["Pid"].as_str().unwrap();
        let cur_id = pid.parse::<i32>().unwrap();
        if prev_id > 0 {
            assert!(prev_id > cur_id, "prev_id: {prev_id}, cur_id: {cur_id}");
        }
        prev_id = cur_id;
    }
}

fn traverse_cgroup_tree(model: &model::CgroupModel, jval: &Value) {
    for dump_field in expand_fields(command::DEFAULT_CGROUP_FIELDS, true) {
        match dump_field {
            DumpField::Common(_) => continue,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleCgroupModel::get_render_config_for_dump(&field_id);
                assert_eq!(
                    rc.render(model.data.query(&field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false)
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {field_id}",
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
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_CGROUP_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let cgroup_dumper = cgroup::Cgroup::new(&opts, None, fields);

    // update model again to populate cpu and io data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut cgroup_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    cgroup_dumper
        .dump_model(&ctx, &model, &mut cgroup_content, &mut round, false)
        .expect("Failed to dump cgroup model");

    // verify json correctness
    assert!(!cgroup_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&cgroup_content).expect("Fail parse json of process dump");
    traverse_cgroup_tree(&model.cgroup, &jval);
}

#[test]
fn test_dump_cgroup_titles() {
    let titles = expand_fields(command::DEFAULT_CGROUP_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleCgroupModel::get_render_config_for_dump(field_id);
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
        "Kernel",
        "Kernel Stack",
        "Mem Slab",
        "Mem Sock",
        "Mem Shmem",
        "Mem Zswap",
        "Mem Zswapped",
        "File Mapped",
        "File Dirty",
        "File WB",
        "File THP",
        "Anon THP",
        "Shmem THP",
        "Inactive Anon",
        "Active Anon",
        "Inactive File",
        "Active File",
        "Unevictable",
        "Slab Reclaimable",
        "Slab Unreclaimable",
        "Pgfault",
        "Pgmajfault",
        "Workingset Refault Anon",
        "Workingset Refault File",
        "Workingset Activate Anon",
        "Workingset Activate File",
        "Workingset Restore Anon",
        "Workingset Restore File",
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
        "Events Low/s",
        "Events High/s",
        "Events Max/s",
        "Events OOM/s",
        "Events Kill/s",
        "Events Local Low/s",
        "Events Local High/s",
        "Events Local Max/s",
        "Events Local OOM/s",
        "Events Local Kill/s",
        "RBytes",
        "WBytes",
        "R I/O",
        "W I/O",
        "DBytes",
        "D I/O",
        "RW Total",
        "Cost Usage",
        "Cost Wait",
        "Cost Indebt",
        "Cost Indelay",
        "CPU Some Pressure",
        "CPU Pressure",
        "I/O Some Pressure",
        "I/O Pressure",
        "Mem Some Pressure",
        "Mem Pressure",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
// Test correctness of iface decoration
// This test will also test JSON correctness.
fn test_dump_iface_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_IFACE_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let iface_dumper = iface::Iface::new(&opts, None, fields.clone());

    // update model again to populate net data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut iface_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    iface_dumper
        .dump_model(&ctx, &model, &mut iface_content, &mut round, false)
        .expect("Failed to dump iface model");

    // verify json correctness
    assert!(!iface_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&iface_content).expect("Fail parse json of network dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let iface = value["Interface"].as_str().unwrap();
        let snm = model
            .network
            .interfaces
            .get(iface)
            .expect("Json iface and snm iface not match");

        for dump_field in fields.iter() {
            match dump_field {
                DumpField::Common(_) => continue,
                DumpField::FieldId(field_id) => {
                    let rc = model::SingleNetModel::get_render_config_for_dump(field_id);
                    assert_eq!(
                        rc.render(snm.query(field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false)
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {field_id}",
                    );
                }
            }
        }
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
fn test_dump_iface_titles() {
    let titles = expand_fields(command::DEFAULT_IFACE_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleNetModel::get_render_config_for_dump(field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "Collisions",
        "Multicast",
        "Interface",
        "RX Bytes/s",
        "TX Bytes/s",
        "I/O Bytes/s",
        "RX Pkts/s",
        "TX Pkts/s",
        "RX Bytes",
        "RX Compressed",
        "RX CRC Errors",
        "RX Dropped",
        "RX Errors",
        "RX Fifo Errors",
        "RX Frame Errors",
        "RX Length Errors",
        "RX Missed Errors",
        "RX Nohandler",
        "RX Over Errors",
        "RX Packets",
        "TX Aborted Errors",
        "TX Bytes",
        "TX Carrier Errors",
        "TX Compressed",
        "TX Dropped",
        "TX Errors",
        "TX Fifo Errors",
        "TX Heartbeat Errors",
        "TX Packets",
        "TX Window Errors",
        "TX Timeout",
        "Raw Stats",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
// Test correctness of network decoration
// This test will also test JSON correctness.
fn test_dump_network_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_NETWORK_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let network_dumper = network::Network::new(&opts, fields.clone());

    // update model again to populate net data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut network_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    network_dumper
        .dump_model(&ctx, &model, &mut network_content, &mut round, false)
        .expect("Failed to dump network model");

    // verify json correctness
    assert!(!network_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&network_content).expect("Fail parse json of network dump");

    for dump_field in fields.iter() {
        match dump_field {
            DumpField::Common(_) => continue,
            DumpField::FieldId(field_id) => {
                let rc = model::NetworkModel::get_render_config_for_dump(field_id);
                assert_eq!(
                    rc.render(model.network.query(field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false),
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {field_id}",
                );
            }
        }
    }
}

#[test]
fn test_dump_network_titles() {
    let titles = expand_fields(command::DEFAULT_NETWORK_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::NetworkModel::get_render_config_for_dump(field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "IpForwPkts/s",
        "IpInPkts/s",
        "IpForwDatagrams/s",
        "IpInDiscardPkts/s",
        "IpInDeliversPkts/s",
        "IpOutReqs/s",
        "IpOutDiscardPkts/s",
        "IpOutNoRoutesPkts/s",
        "IpInMcastPkts/s",
        "IpOutMcastPkts/s",
        "IpInBcastPkts/s",
        "IpOutBcastPkts/s",
        "IpInOctets/s",
        "IpOutOctets/s",
        "IpInMcastOctets/s",
        "IpOutMcastOctets/s",
        "IpInBcastOctets/s",
        "IpOutBcastOctets/s",
        "IpInNoEctPkts/s",
        "Ip6InPkts/s",
        "Ip6InHdrErrs",
        "Ip6InNoRoutesPkts/s",
        "Ip6InAddrErrs",
        "Ip6InDiscardsPkts/s",
        "Ip6InDeliversPkts/s",
        "Ip6ForwDatagrams/s",
        "Ip6OutReqs/s",
        "Ip6OutNoRoutesPkts/s",
        "Ip6InMcastPkts/s",
        "Ip6OutMcastPkts/s",
        "Ip6InOctets/s",
        "Ip6OutOctets/s",
        "Ip6InMcastOctets/s",
        "Ip6OutMcastOctets/s",
        "Ip6InBcastOctets/s",
        "Ip6OutBcastOctets/s",
        "IcmpInMsg/s",
        "IcmpInErrs",
        "IcmpInDestUnreachs",
        "IcmpOutMsg/s",
        "IcmpOutErrs",
        "IcmpOutDestUnreachs",
        "Icmp6InMsg/s",
        "Icmp6InErrs",
        "Icmp6InDestUnreachs",
        "Icmp6OutMsg/s",
        "Icmp6OutErrs",
        "Icmp6OutDestUnreachs",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
// Test correctness of transport decoration
// This test will also test JSON correctness.
fn test_dump_transport_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_TRANSPORT_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let transport_dumper = transport::Transport::new(&opts, fields.clone());

    // update model again to populate net data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut transport_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    transport_dumper
        .dump_model(&ctx, &model, &mut transport_content, &mut round, false)
        .expect("Failed to dump transport model");

    // verify json correctness
    assert!(!transport_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&transport_content).expect("Fail parse json of network dump");

    for dump_field in fields.iter() {
        match dump_field {
            DumpField::Common(_) => continue,
            DumpField::FieldId(field_id) => {
                let rc = model::NetworkModel::get_render_config_for_dump(field_id);
                assert_eq!(
                    rc.render(model.network.query(field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false),
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {field_id}",
                );
            }
        }
    }
}

#[test]
fn test_dump_transport_titles() {
    let titles = expand_fields(command::DEFAULT_TRANSPORT_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::NetworkModel::get_render_config_for_dump(field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "TcpActiveOpens/s",
        "TcpPassiveOpens/s",
        "TcpAttemptFails/s",
        "TcpEstabResets/s",
        "CurEstabConn",
        "TcpInSegs/s",
        "TcpOutSegs/s",
        "TcpRetransSegs/s",
        "TcpRetransSegs",
        "TcpInErrors",
        "TcpOutRsts/s",
        "TcpInCsumErrors",
        "UdpInPkts/s",
        "UdpNoPorts",
        "UdpInErrs",
        "UdpOutPkts/s",
        "UdpRcvbufErrs",
        "UdpSndBufErrs",
        "UdpIgnoredMulti",
        "Udp6InPkts/s",
        "Udp6NoPorts",
        "Udp6InErrs",
        "Udp6OutPkts/s",
        "Udp6RcvbufErrs",
        "Udp6SndBufErrs",
        "Udp6InCsumErrs",
        "Udp6IgnoredMulti",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
fn test_queue_titles() {
    let titles = expand_fields(command::DEFAULT_ETHTOOL_QUEUE_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => Some(field_id.to_string()),
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "interface",
        "queue_id",
        "rx_bytes_per_sec",
        "tx_bytes_per_sec",
        "rx_count_per_sec",
        "tx_count_per_sec",
        "tx_missed_tx",
        "tx_unmask_interrupt",
        "raw_stats",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
fn test_dump_queue_content() {
    let eth0_queue_models = vec![
        model::SingleQueueModel {
            interface: "eth0".to_string(),
            queue_id: 0,
            rx_bytes_per_sec: Some(10),
            tx_bytes_per_sec: Some(20),
            rx_count_per_sec: Some(100),
            tx_count_per_sec: Some(200),
            tx_missed_tx: Some(50),
            tx_unmask_interrupt: Some(5),
            raw_stats: BTreeMap::from([("stat1".to_string(), 1000), ("stat2".to_string(), 2000)]),
        },
        model::SingleQueueModel {
            interface: "eth0".to_string(),
            queue_id: 1,
            rx_bytes_per_sec: Some(20),
            tx_bytes_per_sec: Some(10),
            rx_count_per_sec: Some(200),
            tx_count_per_sec: Some(100),
            tx_missed_tx: Some(5),
            tx_unmask_interrupt: Some(50),
            raw_stats: BTreeMap::from([("stat1".to_string(), 2000), ("stat2".to_string(), 1000)]),
        },
    ];
    let lo_queue_models = vec![model::SingleQueueModel {
        interface: "lo".to_string(),
        queue_id: 1,
        rx_bytes_per_sec: Some(20),
        tx_bytes_per_sec: Some(10),
        rx_count_per_sec: Some(200),
        tx_count_per_sec: Some(100),
        tx_missed_tx: Some(5),
        tx_unmask_interrupt: Some(50),
        raw_stats: BTreeMap::from([("stat1".to_string(), 2000), ("stat2".to_string(), 1000)]),
    }];

    let eth0_model = model::SingleNetModel {
        interface: "eth0".to_string(),
        queues: eth0_queue_models.clone(),
        ..Default::default()
    };
    let lo_model = model::SingleNetModel {
        interface: "lo".to_string(),
        queues: lo_queue_models.clone(),
        ..Default::default()
    };
    let network = model::NetworkModel {
        interfaces: BTreeMap::from([
            ("eth0".to_string(), eth0_model),
            ("lo".to_string(), lo_model),
        ]),
        ..Default::default()
    };
    let model = model::Model {
        time_elapsed: Duration::from_secs(60 * 10),
        timestamp: SystemTime::now(),
        system: model::SystemModel::default(),
        cgroup: model::CgroupModel::default(),
        process: model::ProcessModel::default(),
        network,
        gpu: None,
        resctrl: None,
        tc: None,
    };

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_ETHTOOL_QUEUE_FIELDS, true);

    opts.output_format = Some(OutputFormat::Json);
    let queue_dumper = ethtool::EthtoolQueue::new(&opts, fields.clone());

    let mut queue_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };

    // we are dumping timestamps assuming they are local time
    // so the timezone needs to be set to the expected TZ
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("TZ", "US/Pacific") };

    let result = queue_dumper
        .dump_model(&ctx, &model, &mut queue_content, &mut round, false)
        .expect("Failed to dump queue model");
    assert!(result == tmain::IterExecResult::Success);

    // verify json correctness
    assert!(!queue_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&queue_content).expect("Fail parse json of queue dump");

    let expected_json = json!([
        {
            "Datetime": "1969-12-31 16:00:00",
            "Interface": "eth0",
            "Queue": "0",
            "RawStats": "stat1=1000, stat2=2000",
            "RxBytes": "10 B/s",
            "RxCount": "100/s",
            "Timestamp": "0",
            "TxBytes": "20 B/s",
            "TxCount": "200/s",
            "TxMissedTx": "50",
            "TxUnmaskInterrupt": "5"
        },
        {
            "Datetime": "1969-12-31 16:00:00",
            "Interface": "eth0",
            "Queue": "1",
            "RawStats": "stat1=2000, stat2=1000",
            "RxBytes": "20 B/s",
            "RxCount": "200/s",
            "Timestamp": "0",
            "TxBytes": "10 B/s",
            "TxCount": "100/s",
            "TxMissedTx": "5",
            "TxUnmaskInterrupt": "50"
        },
        {
            "Datetime": "1969-12-31 16:00:00",
            "Interface": "lo",
            "Queue": "1",
            "RawStats": "stat1=2000, stat2=1000",
            "RxBytes": "20 B/s",
            "RxCount": "200/s",
            "Timestamp": "0",
            "TxBytes": "10 B/s",
            "TxCount": "100/s",
            "TxMissedTx": "5",
            "TxUnmaskInterrupt": "50"
        }
    ]);
    assert_eq!(jval, expected_json);
}

#[test]
// Test correctness of disk decoration
// This test will also test JSON correctness.
fn test_dump_disk_content() {
    let logger = get_logger();
    let mut collector = Collector::new(logger.clone(), Default::default());
    collector
        .collect_and_update_model()
        .expect("Fail to get model");

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_DISK_FIELDS, true);
    opts.output_format = Some(OutputFormat::Json);
    let disk_dumper = disk::Disk::new(&opts, None, fields.clone());

    // update model again to populate disk data
    let model = collector
        .collect_and_update_model()
        .expect("Fail to get model");
    let mut disk_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };
    disk_dumper
        .dump_model(&ctx, &model, &mut disk_content, &mut round, false)
        .expect("Failed to dump disk model");

    // verify json correctness
    assert!(!disk_content.is_empty());
    let jval: Value = serde_json::from_slice(&disk_content).expect("Fail parse json of disk dump");

    // verify content correctness, test first 5 should be enough
    let mut count = 5;
    for value in jval.as_array().unwrap() {
        let name = value["Name"].as_str().unwrap();
        let sdm = model
            .system
            .disks
            .get(name)
            .expect("Json pid and sdm pid not match");

        for dump_field in fields.iter() {
            match dump_field {
                DumpField::Common(_) => continue,
                DumpField::FieldId(field_id) => {
                    let rc = model::SingleDiskModel::get_render_config_for_dump(field_id);
                    assert_eq!(
                        rc.render(sdm.query(field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false),
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {field_id}",
                    );
                }
            }
        }
        count -= 1;
        if count == 0 {
            break;
        }
    }
}

#[test]
fn test_dump_disk_titles() {
    let titles = expand_fields(command::DEFAULT_DISK_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleDiskModel::get_render_config_for_dump(field_id);
                Some(rc.render_title(false))
            }
        })
        .collect::<Vec<_>>();
    let expected_titles = vec![
        "Name",
        "Disk",
        "Major",
        "Minor",
        "Read",
        "Read Completed",
        "Read Merged",
        "Read Sectors",
        "Time Spend Read",
        "Write",
        "Write Completed",
        "Write Merged",
        "Write Sectors",
        "Time Spend Write",
        "Discard",
        "Discard Completed",
        "Discard Merged",
        "Discard Sectors",
        "Time Spend Discard",
        "Disk Usage",
        "Partition Size",
        "Filesystem Type",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
fn test_parse_pattern() {
    let tempdir = TempDir::with_prefix("below_dump_pattern.").expect("Failed to create temp dir");
    let path = tempdir.path().join("belowrc");

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open belowrc in tempdir");
    let belowrc_str = r#"
[dump.system]
demacia = ["datetime", "os_release"]

[dump.process]
proc = ["datetime", "mem.anon"]
"#;
    file.write_all(belowrc_str.as_bytes())
        .expect("Faild to write temp belowrc file during testing ignore");
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

    let proc_res = parse_pattern::<command::ProcessOptionField>(
        path.to_string_lossy().to_string(),
        "proc".into(),
        "process",
    )
    .expect("Failed to parse process pattern");

    assert_eq!(
        proc_res[0],
        command::ProcessOptionField::Unit(ProcessField::Common(CommonField::Datetime))
    );
    assert_eq!(
        proc_res[1],
        command::ProcessOptionField::Unit(ProcessField::FieldId(
            model::SingleProcessModelFieldId::Mem(model::ProcessMemoryModelFieldId::Anon)
        ))
    );
}

#[test]
fn test_tc_titles() {
    let titles = expand_fields(command::DEFAULT_TC_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => Some(field_id.to_string()),
        })
        .collect::<Vec<_>>();

    let expected_titles = vec![
        "interface",
        "kind",
        "qlen",
        "bps",
        "pps",
        "bytes_per_sec",
        "packets_per_sec",
        "backlog_per_sec",
        "drops_per_sec",
        "requeues_per_sec",
        "overlimits_per_sec",
        "xstats.fq_codel.maxpacket",
        "xstats.fq_codel.ecn_mark",
        "xstats.fq_codel.new_flows_len",
        "xstats.fq_codel.old_flows_len",
        "xstats.fq_codel.ce_mark",
        "xstats.fq_codel.drop_overlimit_per_sec",
        "xstats.fq_codel.new_flow_count_per_sec",
        "xstats.fq_codel.memory_usage_per_sec",
        "xstats.fq_codel.drop_overmemory_per_sec",
        "qdisc.fq_codel.target",
        "qdisc.fq_codel.limit",
        "qdisc.fq_codel.interval",
        "qdisc.fq_codel.ecn",
        "qdisc.fq_codel.quantum",
        "qdisc.fq_codel.ce_threshold",
        "qdisc.fq_codel.drop_batch_size",
        "qdisc.fq_codel.memory_limit",
        "qdisc.fq_codel.flows",
    ];
    assert_eq!(titles, expected_titles);
}

#[test]
fn test_dump_tc_content() {
    let tc_models = vec![
        model::SingleTcModel {
            interface: "eth0".to_string(),
            kind: "mq".to_string(),
            qlen: Some(42),
            bps: Some(420),
            pps: Some(1337),
            bytes_per_sec: Some(299792458),
            packets_per_sec: Some(314),
            backlog_per_sec: Some(271828182),
            drops_per_sec: Some(8675309),
            requeues_per_sec: Some(12345),
            overlimits_per_sec: Some(314159),
            qdisc: None,
            xstats: None,
        },
        model::SingleTcModel {
            interface: "eth0".to_string(),
            kind: "fq_codel".to_string(),
            qlen: Some(42),
            bps: Some(420),
            pps: Some(1337),
            bytes_per_sec: Some(299792458),
            packets_per_sec: Some(314),
            backlog_per_sec: Some(271828182),
            drops_per_sec: Some(8675309),
            requeues_per_sec: Some(12345),
            overlimits_per_sec: Some(314159),
            qdisc: Some(model::QDiscModel {
                fq_codel: Some(model::FqCodelQDiscModel {
                    target: 2701,
                    limit: 7,
                    interval: 3,
                    ecn: 6,
                    quantum: 42,
                    ce_threshold: 101,
                    drop_batch_size: 9000,
                    memory_limit: 123456,
                    flows: 1024,
                }),
            }),
            xstats: Some(model::XStatsModel {
                fq_codel: Some(model::FqCodelXStatsModel {
                    maxpacket: 8675309,
                    ecn_mark: 299792458,
                    new_flows_len: 314,
                    old_flows_len: 1729,
                    ce_mark: 42,
                    drop_overlimit_per_sec: Some(420),
                    new_flow_count_per_sec: Some(1337),
                    memory_usage_per_sec: Some(271828182),
                    drop_overmemory_per_sec: Some(27182),
                }),
            }),
        },
    ];

    let model = model::Model {
        time_elapsed: Duration::from_secs(60 * 10),
        timestamp: SystemTime::now(),
        system: model::SystemModel::default(),
        cgroup: model::CgroupModel::default(),
        process: model::ProcessModel::default(),
        network: model::NetworkModel::default(),
        gpu: None,
        resctrl: None,
        tc: Some(model::TcModel { tc: tc_models }),
    };

    let mut opts: GeneralOpt = Default::default();
    let fields = command::expand_fields(command::DEFAULT_TC_FIELDS, true);

    opts.output_format = Some(OutputFormat::Json);
    let queue_dumper = tc::Tc::new(&opts, fields.clone());

    let mut queue_content: Vec<u8> = Vec::new();
    let mut round = 0;
    let ctx = CommonFieldContext {
        timestamp: 0,
        hostname: "h".to_string(),
    };

    let result = queue_dumper
        .dump_model(&ctx, &model, &mut queue_content, &mut round, false)
        .expect("Failed to dump queue model");
    assert!(result == tmain::IterExecResult::Success);

    // verify json correctness
    assert!(!queue_content.is_empty());
    let jval: Value =
        serde_json::from_slice(&queue_content).expect("Fail parse json of queue dump");

    let expected_json = json!([
        {
            "Datetime": "1969-12-31 16:00:00",
            "Interface": "eth0",
            "Kind": "mq",
            "Queue Length": "42",
            "Bps": "420 B/s",
            "Pps": "1337/s",
            "Bytes": "285.9 MB/s",
            "Packets": "314/s",
            "Backlog": "271828182/s",
            "Drops": "8675309/s",
            "Requeues": "12345/s",
            "Overlimits": "314159/s",
            "Target": "?",
            "Limit": "?",
            "Interval": "?",
            "Ecn": "?",
            "Quantum": "?",
            "CeThreshold": "?",
            "DropBatchSize": "?",
            "MemoryLimit": "?",
            "Flows": "?",
            "MaxPacket": "?",
            "EcnMark": "?",
            "NewFlowsLen": "?",
            "OldFlowsLen": "?",
            "CeMark": "?",
            "DropOverlimit": "?",
            "NewFlowCount": "?",
            "MemoryUsage": "?",
            "DropOvermemory": "?",
            "Timestamp": "0"
        },
        {
            "Datetime": "1969-12-31 16:00:00",
            "Interface": "eth0",
            "Kind": "fq_codel",
            "Queue Length": "42",
            "Bps": "420 B/s",
            "Pps": "1337/s",
            "Bytes": "285.9 MB/s",
            "Packets": "314/s",
            "Backlog": "271828182/s",
            "Drops": "8675309/s",
            "Requeues": "12345/s",
            "Overlimits": "314159/s",
            "Target": "2701",
            "Limit": "7",
            "Interval": "3",
            "Ecn": "6",
            "Quantum": "42",
            "CeThreshold": "101",
            "DropBatchSize": "9000",
            "MemoryLimit": "123456",
            "Flows": "1024",
            "MaxPacket": "8675309",
            "EcnMark": "299792458",
            "NewFlowsLen": "314",
            "OldFlowsLen": "1729",
            "CeMark": "42",
            "DropOverlimit": "420/s",
            "NewFlowCount": "1337/s",
            "MemoryUsage": "271828182/s",
            "DropOvermemory": "27182/s",
            "Timestamp": "0"
        }
    ]);
    assert_eq!(jval, expected_json);
}
