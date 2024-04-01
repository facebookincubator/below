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

/// This model is meant for testing purposes. The model is not meant to contain
/// realistic numbers. The only condition that needs to be satisfied is that
/// the JSON representation can be deserialized into a valid model.
pub const SAMPLE_MODEL_JSON: &str = r#"
{
    "time_elapsed": {
        "secs": 5,
        "nanos": 0
    },
    "timestamp": {
        "secs_since_epoch": 1100000000,
        "nanos_since_epoch": 0
    },
    "system": {
        "hostname": "hostname.example.com",
        "kernel_version": "5.11.10",
        "os_release": "some os",
        "stat": {
            "total_interrupt_ct": 10000,
            "context_switches": 10000,
            "boot_time_epoch_secs": 1000000000,
            "total_processes": 1,
            "running_processes": 1,
            "blocked_processes": 0
        },
        "total_cpu": {
            "idx": -1,
            "usage_pct": 20.0,
            "user_pct": 10.0,
            "system_pct": 5.0,
            "idle_pct": 80.0,
            "nice_pct": 0.05,
            "iowait_pct": 1.0,
            "irq_pct": 1.0,
            "softirq_pct": 0.2,
            "stolen_pct": 0.0,
            "guest_pct": 0.0,
            "guest_nice_pct": 0.0
        },
        "cpus": {
            "0": {
                "idx": 0,
                "usage_pct": 20.0,
                "user_pct": 10.0,
                "system_pct": 5.0,
                "idle_pct": 80.0,
                "nice_pct": 0.05,
                "iowait_pct": 1.0,
                "irq_pct": 1.0,
                "softirq_pct": 0.2,
                "stolen_pct": 0.0,
                "guest_pct": 0.0,
                "guest_nice_pct": 0.0
            },
            "1": {
                "idx": 1,
                "usage_pct": 20.0,
                "user_pct": 10.0,
                "system_pct": 5.0,
                "idle_pct": 80.0,
                "nice_pct": 0.05,
                "iowait_pct": 1.0,
                "irq_pct": 1.0,
                "softirq_pct": 0.2,
                "stolen_pct": 0.0,
                "guest_pct": 0.0,
                "guest_nice_pct": 0.0
            }
        },
        "mem": {
            "total": 8000000000,
            "free": 4000000000,
            "available": 8000000000,
            "buffers": 20000,
            "cached": 2000000000,
            "swap_cached": 10000000,
            "active": 3000000000,
            "inactive": 1000000000,
            "anon": 2000000000,
            "file": 2000000000,
            "unevictable": 3000000,
            "mlocked": 3000000,
            "swap_total": 2000000000,
            "swap_free": 1000000000,
            "dirty": 500000,
            "writeback": 10000,
            "anon_pages": 2000000000,
            "mapped": 100000000,
            "shmem": 1000000,
            "kreclaimable": 200000000,
            "slab": 300000000,
            "slab_reclaimable": 200000000,
            "slab_unreclaimable": 80000000,
            "kernel_stack": 10000000,
            "page_tables": 30000000,
            "anon_huge_pages_bytes": 9,
            "shmem_huge_pages_bytes": 0,
            "file_huge_pages_bytes": 0,
            "hugetlb": 0,
            "cma_total": 0,
            "cma_free": 0,
            "vmalloc_total": 300000000,
            "vmalloc_used": 3000000,
            "vmalloc_chunk": 0,
            "direct_map_4k": 100000000,
            "direct_map_2m": 400000000,
            "direct_map_1g": null
        },
        "vm": {
            "pgpgin_per_sec": 5555.5,
            "pgpgout_per_sec": 999.9,
            "pswpin_per_sec": 0.0,
            "pswpout_per_sec": 0.0,
            "pgsteal_kswapd": 0,
            "pgsteal_direct": 0,
            "pgscan_kswapd": 0,
            "pgscan_direct": 0,
            "oom_kill": 0
        },
        "slab": {
            "task_struct": {
                "name": "task_group",
                "active_objs": 13000,
                "num_objs": 14000,
                "obj_size": 6000,
                "obj_per_slab": 5,
                "num_slabs": 3000
            },
            "vmap_area": {
                "name": "vmap_area",
                "active_objs": 4000000,
                "num_objs": 6000000,
                "obj_size": 64,
                "obj_per_slab": 64,
                "num_slabs": 100000
            }
        },
        "disks": {
            "vda": {
                "name": "vda",
                "read_bytes_per_sec": 500000.0,
                "write_bytes_per_sec": 100000.0,
                "discard_bytes_per_sec": 0.0,
                "disk_total_bytes_per_sec": 600000.0,
                "read_completed": 2000000,
                "read_merged": 1000000,
                "read_sectors": 6000000,
                "time_spend_read_ms": 200000,
                "write_completed": 1000000,
                "write_merged": 100000,
                "write_sectors": 40000000,
                "time_spend_write_ms": 3000000,
                "discard_completed": 0,
                "discard_merged": 0,
                "discard_sectors": 0,
                "time_spend_discard_ms": 0,
                "major": 20,
                "minor": 0
            },
            "vda1": {
                "name": "vda1",
                "read_bytes_per_sec": 500000.0,
                "write_bytes_per_sec": 100000.0,
                "discard_bytes_per_sec": 0.0,
                "disk_total_bytes_per_sec": 600000.0,
                "read_completed": 2000000,
                "read_merged": 1000000,
                "read_sectors": 6000000,
                "time_spend_read_ms": 200000,
                "write_completed": 1000000,
                "write_merged": 100000,
                "write_sectors": 40000000,
                "time_spend_write_ms": 3000000,
                "discard_completed": 0,
                "discard_merged": 0,
                "discard_sectors": 0,
                "time_spend_discard_ms": 0,
                "major": 20,
                "minor": 0
            }
        },
        "btrfs": {
            "b_name": {
                "name": "b_name",
                "disk_fraction": 5.0,
                "disk_bytes": 123
            }
        }
    },
    "cgroup": {
        "data": {
            "name": "<root>",
            "full_path": "",
            "inode_number": 1,
            "depth": 0,
            "properties": {
                "cgroup_controllers": ["cpu", "cpuset", "hugetlb", "io", "memory", "pids"],
                "cgroup_subtree_control": ["cpu", "cpuset", "io", "memory", "pids"],
                "memory_min": null,
                "memory_low": null,
                "memory_high": null,
                "memory_max": null,
                "memory_swap_max": null,
                "memory_zswap_max": null,
                "cpu_weight": null,
                "cpu_max_usec": null,
                "cpu_max_period_usec": null,
                "cpuset_cpus": {
                    "cpus": []
                },
                "cpuset_cpus_effective": {
                    "cpus": [0, 1, 2, 3]
                },
                "cpuset_mems": {
                    "nodes": []
                },
                "cpuset_mems_effective": {
                    "nodes": [0, 1, 2, 3]
                }
            },
            "cpu": null,
            "memory": {
                "total": 5000000000,
                "swap": 1000000000,
                "anon": 2000000000,
                "file": 2000000000,
                "kernel_stack": 40000000,
                "slab": 300000000,
                "sock": 10000000,
                "shmem": 2000000,
                "zswap": 100,
                "zswapped": 1000000,
                "file_mapped": 200000000,
                "file_dirty": 3000000,
                "file_writeback": 60000000,
                "anon_thp": 10000000,
                "inactive_anon": 500000000,
                "active_anon": 2000000000,
                "inactive_file": 900000000,
                "active_file": 1000000000,
                "unevictable": 5000000,
                "slab_reclaimable": 200000000,
                "slab_unreclaimable": 100000000,
                "pgfault": 3000,
                "pgmajfault": 100,
                "workingset_refault": 3000,
                "workingset_activate": 2000,
                "workingset_nodereclaim": 0,
                "pgrefill": 0,
                "pgscan": 0,
                "pgsteal": 0,
                "pgactivate": 0,
                "pgdeactivate": 0,
                "pglazyfree": 0,
                "pglazyfreed": 0,
                "thp_fault_alloc": 0,
                "thp_collapse_alloc": 0,
                "events_low": 0,
                "events_high": 300000,
                "events_max": 300000,
                "events_oom": 0,
                "events_oom_kill": 0
            },
            "io": null,
            "io_total": null,
            "pressure": {
                "cpu_some_pct": 2.05,
                "io_some_pct": 0.8,
                "io_full_pct": 0.7,
                "memory_some_pct": 0.6,
                "memory_full_pct": 0.3
            }
        },
        "children": [
            {
                "data": {
                    "name": "init.scope",
                    "full_path": "/init.scope",
                    "inode_number": 17,
                    "depth": 1,
                    "properties": {
                        "cgroup_controllers": ["cpu", "cpuset", "io", "memory", "pids"],
                        "cgroup_subtree_control": ["cpu", "cpuset", "io", "memory", "pids"],
                        "memory_min": 0,
                        "memory_low": 0,
                        "memory_high": -1,
                        "memory_max": -1,
                        "memory_swap_max": -1,
                        "memory_zswap_max": -1,
                        "cpu_weight": 100,
                        "cpu_max_usec": -1,
                        "cpu_max_period_usec": 100000,
                        "cpuset_cpus": {
                            "cpus": []
                        },
                        "cpuset_cpus_effective": {
                            "cpus": [0, 1, 2, 3]
                        },
                        "cpuset_mems": {
                            "nodes": []
                        },
                        "cpuset_mems_effective": {
                            "nodes": [0, 1, 2, 3]
                        }
                    },
                    "cpu": {
                        "usage_pct": 0.01000,
                        "user_pct": 0.00900,
                        "system_pct": 0.00400,
                        "nr_periods_per_sec": 0.0,
                        "nr_throttled_per_sec": 0.0,
                        "throttled_pct": 0.0
                    },
                    "memory": {
                        "total": 20000000,
                        "swap": 5000000,
                        "anon": 6000000,
                        "file": 10000000,
                        "kernel_stack": 1000000,
                        "slab": 5000000,
                        "sock": 0,
                        "shmem": 30000,
                        "zswap": 20,
                        "zswapped": 20000,
                        "file_mapped": 7000000,
                        "file_dirty": 0,
                        "file_writeback": 800000,
                        "anon_thp": 0,
                        "inactive_anon": 3000000,
                        "active_anon": 3000000,
                        "inactive_file": 600000,
                        "active_file": 6000000,
                        "unevictable": 4000000,
                        "slab_reclaimable": 3000000,
                        "slab_unreclaimable": 2000000,
                        "pgfault": 0,
                        "pgmajfault": 0,
                        "workingset_refault": 0,
                        "workingset_activate": 0,
                        "workingset_nodereclaim": 0,
                        "pgrefill": 0,
                        "pgscan": 0,
                        "pgsteal": 0,
                        "pgactivate": 0,
                        "pgdeactivate": 0,
                        "pglazyfree": 0,
                        "pglazyfreed": 0,
                        "thp_fault_alloc": 0,
                        "thp_collapse_alloc": 0,
                        "events_low": 0,
                        "events_high": 0,
                        "events_max": 0,
                        "events_oom": 0,
                        "events_oom_kill": 0
                    },
                    "io": null,
                    "io_total": null,
                    "pressure": {
                        "cpu_some_pct": 0.0,
                        "io_some_pct": 0.0,
                        "io_full_pct": 0.0,
                        "memory_some_pct": 0.0,
                        "memory_full_pct": 0.0
                    }
                },
                "children": [],
                "count": 1,
                "recreate_flag": false
            },
            {
                "data": {
                    "name": "child_a.slice",
                    "full_path": "/child_a.slice",
                    "inode_number": 11111,
                    "depth": 1,
                    "properties": {
                        "cgroup_controllers": ["cpu", "cpuset", "io", "memory", "pids"],
                        "cgroup_subtree_control": ["io", "memory", "pids"],
                        "memory_min": 2000000000,
                        "memory_low": 0,
                        "memory_high": -1,
                        "memory_max": -1,
                        "memory_swap_max": 0,
                        "memory_zswap_max": 0,
                        "cpu_weight": null,
                        "cpu_max_usec": null,
                        "cpu_max_period_usec": null,
                        "cpuset_cpus": null,
                        "cpuset_cpus_effective": null,
                        "cpuset_mems": null,
                        "cpuset_mems_effective": null
                    },
                    "cpu": {
                        "usage_pct": 0.0,
                        "user_pct": 0.0,
                        "system_pct": 0.0,
                        "nr_periods_per_sec": 0.0,
                        "nr_throttled_per_sec": 0.0,
                        "throttled_pct": 0.0
                    },
                    "memory": {
                        "total": 8000,
                        "swap": 0,
                        "anon": 0,
                        "file": 0,
                        "kernel_stack": 0,
                        "slab": 3000000,
                        "sock": 0,
                        "shmem": 0,
                        "zswap": 0,
                        "zswapped": 0,
                        "file_mapped": 0,
                        "file_dirty": 0,
                        "file_writeback": 0,
                        "anon_thp": 0,
                        "inactive_anon": 0,
                        "active_anon": 0,
                        "inactive_file": 0,
                        "active_file": 0,
                        "unevictable": 0,
                        "slab_reclaimable": 1000000,
                        "slab_unreclaimable": 1000000,
                        "pgfault": 0,
                        "pgmajfault": 0,
                        "workingset_refault": 0,
                        "workingset_activate": 0,
                        "workingset_nodereclaim": 0,
                        "pgrefill": 0,
                        "pgscan": 0,
                        "pgsteal": 0,
                        "pgactivate": 0,
                        "pgdeactivate": 0,
                        "pglazyfree": 0,
                        "pglazyfreed": 0,
                        "thp_fault_alloc": 0,
                        "thp_collapse_alloc": 0,
                        "events_low": 0,
                        "events_high": 0,
                        "events_max": 0,
                        "events_oom": 0,
                        "events_oom_kill": 0
                    },
                    "io": null,
                    "io_total": null,
                    "pressure": {
                        "cpu_some_pct": 0.0,
                        "io_some_pct": 0.0,
                        "io_full_pct": 0.0,
                        "memory_some_pct": 0.0,
                        "memory_full_pct": 0.0
                    }
                },
                "children": [],
                "count": 1,
                "recreate_flag": false
            },
            {
                "data": {
                    "name": "child_b.slice",
                    "full_path": "/child_b.slice",
                    "inode_number": 1111,
                    "depth": 1,
                    "properties": {
                        "cgroup_controllers": ["cpu", "cpuset", "io", "memory", "pids"],
                        "cgroup_subtree_control": ["cpu", "cpuset", "pids"],
                        "memory_min": null,
                        "memory_low": null,
                        "memory_high": null,
                        "memory_max": null,
                        "memory_swap_max": null,
                        "memory_zswap_max": null,
                        "cpu_weight": 100,
                        "cpu_max_usec": 1500000,
                        "cpu_max_period_usec": 100000,
                        "cpuset_cpus": {
                            "cpus": [2, 3]
                        },
                        "cpuset_cpus_effective": {
                            "cpus": [2, 3]
                        },
                        "cpuset_mems": {
                            "nodes": [2, 3]
                        },
                        "cpuset_mems_effective": {
                            "nodes": [2, 3]
                        }
                    },
                    "cpu": {
                        "usage_pct": 3.5,
                        "user_pct": 3.5,
                        "system_pct": 0.0,
                        "nr_periods_per_sec": 0.0,
                        "nr_throttled_per_sec": 0.0,
                        "throttled_pct": 0.0
                    },
                    "memory": {
                        "total": 30000000,
                        "swap": 9000000,
                        "anon": 8000000,
                        "file": 20000000,
                        "kernel_stack": 800000,
                        "slab": 3000000,
                        "sock": 400000,
                        "shmem": 1000000,
                        "zswap": 500,
                        "zswapped": 500000,
                        "file_mapped": 50000000,
                        "file_dirty": 1000000,
                        "file_writeback": 9000000,
                        "anon_thp": 0,
                        "inactive_anon": 60000000,
                        "active_anon": 40000000,
                        "inactive_file": 70000000,
                        "active_file": 100000000,
                        "unevictable": 0,
                        "slab_reclaimable": 20000000,
                        "slab_unreclaimable": 10000000,
                        "pgfault": 10,
                        "pgmajfault": 0,
                        "workingset_refault": 0,
                        "workingset_activate": 0,
                        "workingset_nodereclaim": 0,
                        "pgrefill": 0,
                        "pgscan": 0,
                        "pgsteal": 0,
                        "pgactivate": 0,
                        "pgdeactivate": 0,
                        "pglazyfree": 0,
                        "pglazyfreed": 0,
                        "thp_fault_alloc": 0,
                        "thp_collapse_alloc": 0,
                        "events_low": 0,
                        "events_high": 0,
                        "events_max": 0,
                        "events_oom": 0,
                        "events_oom_kill": 0
                    },
                    "io": null,
                    "io_total": null,
                    "pressure": {
                        "cpu_some_pct": 0.15,
                        "io_some_pct": 0.0,
                        "io_full_pct": 0.0,
                        "memory_some_pct": 0.0,
                        "memory_full_pct": 0.0
                    }
                },
                "children": [
                    {
                        "data": {
                            "name": "something.service",
                            "full_path": "/child_b.slice/something.service",
                            "inode_number": 11111111,
                            "depth": 2,
                            "properties": {
                                "cgroup_controllers": ["cpu", "cpuset", "pids"],
                                "cgroup_subtree_control": ["cpu", "cpuset", "pids"],
                                "memory_min": null,
                                "memory_low": null,
                                "memory_high": null,
                                "memory_max": null,
                                "memory_swap_max": null,
                                "memory_zswap_max": null,
                                "cpu_weight": 100,
                                "cpu_max_usec": 1200000,
                                "cpu_max_period_usec": 100000,
                                "cpuset_cpus": {
                                    "cpus": []
                                },
                                "cpuset_cpus_effective": {
                                    "cpus": [2, 3]
                                },
                                "cpuset_mems": {
                                    "nodes": []
                                },
                                "cpuset_mems_effective": {
                                    "nodes": [2, 3]
                                }
                            },
                            "cpu": {
                                "usage_pct": 0.6,
                                "user_pct": 0.1,
                                "system_pct": 0.5,
                                "nr_periods_per_sec": null,
                                "nr_throttled_per_sec": null,
                                "throttled_pct": null
                            },
                            "memory": {
                                "total": 500000,
                                "swap": 0,
                                "anon": 100000,
                                "file": 200000,
                                "kernel_stack": 30000,
                                "slab": 200000,
                                "sock": 0,
                                "shmem": 0,
                                "zswap": 50,
                                "zswapped": 50000,
                                "file_mapped": 200000,
                                "file_dirty": 0,
                                "file_writeback": 0,
                                "anon_thp": 0,
                                "inactive_anon": 0,
                                "active_anon": 100000,
                                "inactive_file": 20000,
                                "active_file": 200000,
                                "unevictable": 0,
                                "slab_reclaimable": 70000,
                                "slab_unreclaimable": 100000,
                                "pgfault": 0,
                                "pgmajfault": 0,
                                "workingset_refault": 0,
                                "workingset_activate": 0,
                                "workingset_nodereclaim": 0,
                                "pgrefill": 0,
                                "pgscan": 0,
                                "pgsteal": 0,
                                "pgactivate": 0,
                                "pgdeactivate": 0,
                                "pglazyfree": 0,
                                "pglazyfreed": 0,
                                "thp_fault_alloc": 0,
                                "thp_collapse_alloc": 0,
                                "events_low": 0,
                                "events_high": 0,
                                "events_max": 0,
                                "events_oom": 0,
                                "events_oom_kill": 0
                            },
                            "io": null,
                            "io_total": null,
                            "pressure": {
                                "cpu_some_pct": 0.0,
                                "io_some_pct": 0.0,
                                "io_full_pct": 0.0,
                                "memory_some_pct": 0.0,
                                "memory_full_pct": 0.0
                            }
                        },
                        "children": [],
                        "count": 1,
                        "recreate_flag": false
                    }
                ],
                "count": 1,
                "recreate_flag": false
            }
        ],
        "count": 2,
        "recreate_flag": false
    },
    "process": {
        "processes": {
            "1": {
                "pid": 1,
                "ppid": 0,
                "comm": "systemd",
                "state": "Running",
                "uptime_secs": 4000000,
                "cgroup": "/init.scope",
                "io": {
                    "rbytes_per_sec": 0.0,
                    "wbytes_per_sec": 0.0,
                    "rwbytes_per_sec": 0.0
                },
                "mem": {
                    "minorfaults_per_sec": 100.0,
                    "majorfaults_per_sec": 0.0,
                    "rss_bytes": 10000000,
                    "vm_size": 200000000,
                    "lock": 0,
                    "pin": 0,
                    "anon": 6000000,
                    "file": 7000000,
                    "shmem": 0,
                    "pte": 200000,
                    "swap": 1000000,
                    "huge_tlb": 0
                },
                "cpu": {
                    "usage_pct": 1.0,
                    "user_pct": 1.0,
                    "system_pct": 0.5,
                    "num_threads": 1
                },
                "cmdline": "/usr/lib/systemd/systemd",
                "exe_path": "/usr/lib/systemd/systemd"
            }
        }
    },
    "network": {
        "interfaces": {
            "eth0": {
                "interface": "eth0",
                "rx_bytes_per_sec": 200000.5,
                "tx_bytes_per_sec": 50000.5,
                "throughput_per_sec": 200000.5,
                "rx_packets_per_sec": 200,
                "tx_packets_per_sec": 100,
                "collisions": 0,
                "multicast": 0,
                "rx_bytes": 9000000000,
                "rx_compressed": 0,
                "rx_crc_errors": 0,
                "rx_dropped": 0,
                "rx_errors": 0,
                "rx_fifo_errors": 0,
                "rx_frame_errors": 0,
                "rx_length_errors": 0,
                "rx_missed_errors": 0,
                "rx_nohandler": 0,
                "rx_over_errors": 0,
                "rx_packets": 100000000,
                "tx_aborted_errors": 0,
                "tx_bytes": 9000000000,
                "tx_carrier_errors": 0,
                "tx_compressed": 0,
                "tx_dropped": 0,
                "tx_errors": 0,
                "tx_fifo_errors": 0,
                "tx_heartbeat_errors": 0,
                "tx_packets": 100000000,
                "tx_window_errors": 0,
                "tx_timeout_per_sec": 10,
                "raw_stats": {
                    "stat0": 0
                },
                "queues": [
                    {
                        "interface": "eth0",
                        "queue_id": 0,
                        "rx_bytes_per_sec": 42,
                        "tx_bytes_per_sec": 1337,
                        "rx_count_per_sec": 10,
                        "tx_count_per_sec": 20,
                        "tx_missed_tx": 100,
                        "tx_unmask_interrupt": 200,
                        "raw_stats": {
                            "stat1": 1,
                            "stat2": 2
                        }
                    },
                    {
                        "interface": "eth0",
                        "queue_id": 1,
                        "rx_bytes_per_sec": 1337,
                        "tx_bytes_per_sec": 42,
                        "rx_count_per_sec": 20,
                        "tx_count_per_sec": 10,
                        "tx_missed_tx": 200,
                        "tx_unmask_interrupt": 100,
                        "raw_stats": {
                            "stat3": 3,
                            "stat4": 4
                        }
                    }
                ]
            },
            "lo": {
                "interface": "lo",
                "rx_bytes_per_sec": 10000000.5,
                "tx_bytes_per_sec": 10000000.5,
                "throughput_per_sec": 30000000.5,
                "rx_packets_per_sec": 1000,
                "tx_packets_per_sec": 1000,
                "collisions": 0,
                "multicast": 0,
                "rx_bytes": 100000000000,
                "rx_compressed": 0,
                "rx_crc_errors": 0,
                "rx_dropped": 0,
                "rx_errors": 0,
                "rx_fifo_errors": 0,
                "rx_frame_errors": 0,
                "rx_length_errors": 0,
                "rx_missed_errors": 0,
                "rx_nohandler": 0,
                "rx_over_errors": 0,
                "rx_packets": 60000000,
                "tx_aborted_errors": 0,
                "tx_bytes": 100000000000,
                "tx_carrier_errors": 0,
                "tx_compressed": 0,
                "tx_dropped": 0,
                "tx_errors": 0,
                "tx_fifo_errors": 0,
                "tx_heartbeat_errors": 0,
                "tx_packets": 60000000,
                "tx_window_errors": 0,
                "tx_timeout_per_sec": 1,
                "raw_stats": {
                    "stat0": 0
                },
                "queues": [
                    {
                        "interface": "lo",
                        "queue_id": 0,
                        "rx_bytes_per_sec": 24,
                        "tx_bytes_per_sec": 7331,
                        "rx_count_per_sec": 1,
                        "tx_count_per_sec": 2,
                        "tx_missed_tx": 3,
                        "tx_unmask_interrupt": 400,
                        "raw_stats": {
                            "stat1": 5,
                            "stat2": 6
                        }
                    },
                    {
                        "interface": "lo",
                        "queue_id": 1,
                        "rx_bytes_per_sec": 7331,
                        "tx_bytes_per_sec": 24,
                        "rx_count_per_sec": 2,
                        "tx_count_per_sec": 1,
                        "tx_missed_tx": 4,
                        "tx_unmask_interrupt": 3,
                        "raw_stats": {
                            "stat3": 7,
                            "stat4": 8
                        }
                    }
                ]
            }
        },
        "tcp": {
            "active_opens_per_sec": 10,
            "passive_opens_per_sec": 10,
            "attempt_fails_per_sec": 1,
            "estab_resets_per_sec": 1,
            "curr_estab_conn": 1000,
            "in_segs_per_sec": 1000,
            "out_segs_per_sec": 1000,
            "retrans_segs_per_sec": 0,
            "retrans_segs": 70000000,
            "in_errs": 5000,
            "out_rsts_per_sec": 10,
            "in_csum_errors": 100
        },
        "ip": {
            "forwarding_pkts_per_sec": 0,
            "in_receives_pkts_per_sec": 5,
            "forw_datagrams_per_sec": 0,
            "in_discards_pkts_per_sec": 0,
            "in_delivers_pkts_per_sec": 5,
            "out_requests_per_sec": 5,
            "out_discards_pkts_per_sec": 0,
            "out_no_routes_pkts_per_sec": 0,
            "in_mcast_pkts_per_sec": 0,
            "out_mcast_pkts_per_sec": 0,
            "in_bcast_pkts_per_sec": 0,
            "out_bcast_pkts_per_sec": 0,
            "in_octets_per_sec": 100000,
            "out_octets_per_sec": 100000,
            "in_mcast_octets_per_sec": 0,
            "out_mcast_octets_per_sec": 0,
            "in_bcast_octets_per_sec": 0,
            "out_bcast_octets_per_sec": 0,
            "in_no_ect_pkts_per_sec": 5
        },
        "ip6": {
            "in_receives_pkts_per_sec": 1000,
            "in_hdr_errors": 20000000,
            "in_no_routes_pkts_per_sec": 0,
            "in_addr_errors": 70,
            "in_discards_pkts_per_sec": 0,
            "in_delivers_pkts_per_sec": 1000,
            "out_forw_datagrams_per_sec": 0,
            "out_requests_per_sec": 1000,
            "out_no_routes_pkts_per_sec": 0,
            "in_mcast_pkts_per_sec": 70,
            "out_mcast_pkts_per_sec": 0,
            "in_octets_per_sec": 1000000,
            "out_octets_per_sec": 1000000,
            "in_mcast_octets_per_sec": 1000,
            "out_mcast_octets_per_sec": 10,
            "in_bcast_octets_per_sec": 0,
            "out_bcast_octets_per_sec": 0
        },
        "icmp": {
            "in_msgs_per_sec": 0,
            "in_errors": 70,
            "in_dest_unreachs": 70,
            "out_msgs_per_sec": 0,
            "out_errors": 0,
            "out_dest_unreachs": 70
        },
        "icmp6": {
            "in_msgs_per_sec": 2,
            "in_errors": 90,
            "in_dest_unreachs": 100,
            "out_msgs_per_sec": 2,
            "out_errors": 0,
            "out_dest_unreachs": 100
        },
        "udp": {
            "in_datagrams_pkts_per_sec": 0,
            "no_ports": 70,
            "in_errors": 1000,
            "out_datagrams_pkts_per_sec": 0,
            "rcvbuf_errors": 1000,
            "sndbuf_errors": 0,
            "ignored_multi": 30000
        },
        "udp6": {
            "in_datagrams_pkts_per_sec": 100,
            "no_ports": 90,
            "in_errors": 10000000,
            "out_datagrams_pkts_per_sec": 0,
            "rcvbuf_errors": 10000000,
            "sndbuf_errors": 0,
            "in_csum_errors": 0,
            "ignored_multi": 0
        }
    },
    "tc": {
        "tc": [
            {
                "interface": "eth0",
                "kind": "fq_codel",
                "qlen": 42,
                "bps": 420,
                "pps": 1337,
                "bytes_per_sec": 299792458,
                "packets_per_sec": 314,
                "backlog_per_sec": 2718281828,
                "drops_per_sec": 8675309,
                "requeues_per_sec": 12345,
                "overlimits_per_sec": 314159,
                "qdisc": {
                    "fq_codel": {
                        "target": 2701,
                        "limit": 7,
                        "interval": 3,
                        "ecn": 6,
                        "quantum": 42,
                        "ce_threshold": 101,
                        "drop_batch_size": 9000,
                        "memory_limit": 123456,
                        "flows_per_sec": 31415
                    }
                },
                "xstats": {
                    "fq_codel": {
                        "maxpacket": 8675309,
                        "ecn_mark": 299792458,
                        "new_flows_len": 314,
                        "old_flows_len": 1729,
                        "ce_mark": 42,
                        "drop_overlimit_per_sec": 420,
                        "new_flow_count_per_sec": 1337,
                        "memory_usage_per_sec": 2718281828,
                        "drop_overmemory_per_sec": 27182
                    }
                }
            }
        ]
    }
}
"#;
