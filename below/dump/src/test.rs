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

use command::expand_fields;
use command::GeneralOpt;
use command::OutputFormat;
use common::logutil::get_logger;
use model::Collector;
use model::EnumIter;
use model::Queriable;
use render::HasRenderConfigForDump;
use serde_json::Value;
use tempdir::TempDir;
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
    for subquery_id in model::SingleCpuModelFieldId::unit_variant_iter() {
        fields.push(DumpField::FieldId(model::SystemModelFieldId::Cpus(
            model::BTreeMapFieldId {
                key: Some(31),
                subquery_id,
            },
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
                let rc = model::SystemModel::get_render_config_for_dump(&field_id);
                assert_eq!(
                    rc.render(model.system.query(&field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false)
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {}",
                    field_id.to_string(),
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
            model::SingleCpuModelFieldId::unit_variant_iter().map(|subquery_id| {
                DumpField::FieldId(model::SystemModelFieldId::Cpus(model::BTreeMapFieldId {
                    key: Some(31),
                    subquery_id,
                }))
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
                    let rc = model::SingleProcessModel::get_render_config_for_dump(&field_id);
                    assert_eq!(
                        rc.render(spm.query(&field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false)
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {}",
                        field_id.to_string(),
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
                let rc = model::SingleProcessModel::get_render_config_for_dump(&field_id);
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
    let mut opts: GeneralOpt = Default::default();
    opts.everything = true;
    opts.output_format = Some(OutputFormat::Json);
    opts.filter = Some(
        regex::Regex::new(&model.process.processes.iter().last().unwrap().0.to_string())
            .expect("Fail to construct regex"),
    );
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
            assert!(prev_id < cur_id, "prev_id: {}, cur_id: {}", prev_id, cur_id);
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
            assert!(prev_id > cur_id, "prev_id: {}, cur_id: {}", prev_id, cur_id);
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
    let mut jval: Value =
        serde_json::from_slice(&cgroup_content).expect("Fail parse json of process dump");
    traverse_cgroup_tree(&model.cgroup, &mut jval);
}

#[test]
fn test_dump_cgroup_titles() {
    let titles = expand_fields(command::DEFAULT_CGROUP_FIELDS, true)
        .iter()
        .filter_map(|dump_field| match dump_field {
            DumpField::Common(_) => None,
            DumpField::FieldId(field_id) => {
                let rc = model::SingleCgroupModel::get_render_config_for_dump(&field_id);
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
        "Mem Zswap",
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
        "Mem High",
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
                    let rc = model::SingleNetModel::get_render_config_for_dump(&field_id);
                    assert_eq!(
                        rc.render(snm.query(&field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false)
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {}",
                        field_id.to_string(),
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
                let rc = model::SingleNetModel::get_render_config_for_dump(&field_id);
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
                let rc = model::NetworkModel::get_render_config_for_dump(&field_id);
                assert_eq!(
                    rc.render(model.network.query(&field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false),
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {}",
                    field_id.to_string(),
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
                let rc = model::NetworkModel::get_render_config_for_dump(&field_id);
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
                let rc = model::NetworkModel::get_render_config_for_dump(&field_id);
                assert_eq!(
                    rc.render(model.network.query(&field_id), false),
                    jval[rc.render_title(false)]
                        .as_str()
                        .unwrap_or_else(|| panic!(
                            "Key not found in Json: {}",
                            rc.render_title(false),
                        ))
                        .to_owned(),
                    "Model value and json value do not match for field: {}",
                    field_id.to_string(),
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
                let rc = model::NetworkModel::get_render_config_for_dump(&field_id);
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
                    let rc = model::SingleDiskModel::get_render_config_for_dump(&field_id);
                    assert_eq!(
                        rc.render(sdm.query(&field_id), false),
                        value[rc.render_title(false)]
                            .as_str()
                            .unwrap_or_else(|| panic!(
                                "Key not found in Json: {}",
                                rc.render_title(false),
                            ))
                            .to_owned(),
                        "Model value and json value do not match for field: {}",
                        field_id.to_string(),
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
                let rc = model::SingleDiskModel::get_render_config_for_dump(&field_id);
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
    let tempdir = TempDir::new("below_dump_pattern").expect("Failed to create temp dir");
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
