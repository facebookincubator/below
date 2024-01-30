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

use model::SingleCgroupModelFieldId;
use model::SingleProcessModelFieldId;
use RenderFormat::Duration;
use RenderFormat::MaxOrDuration;
use RenderFormat::MaxOrReadableSize;
use RenderFormat::PageReadableSize;
use RenderFormat::Precision;
use RenderFormat::ReadableSize;
use RenderFormat::SectorReadableSize;

use super::*;

impl HasRenderConfig for model::SingleCgroupModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleCgroupModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Name => rc.title("Name").width(50),
            FullPath => rc.title("Full Path").width(50),
            InodeNumber => rc.title("Inode Number"),
            Cpu(field_id) => model::CgroupCpuModel::get_render_config_builder(field_id),
            Io(field_id) => model::CgroupIoModel::get_render_config_builder(field_id),
            IoDetails(field_id) => {
                model::CgroupIoModel::get_render_config_builder(&field_id.subquery_id)
            }
            Mem(field_id) => model::CgroupMemoryModel::get_render_config_builder(field_id),
            Pressure(field_id) => model::CgroupPressureModel::get_render_config_builder(field_id),
            CgroupStat(field_id) => model::CgroupStatModel::get_render_config_builder(field_id),
            MemNuma(field_id) => {
                model::CgroupMemoryNumaModel::get_render_config_builder(&field_id.subquery_id)
            }
            Props(field_id) => model::CgroupProperties::get_render_config_builder(field_id),
            Pids(field_id) => model::CgroupPidsModel::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::SingleCgroupModel {
    fn get_render_config_for_dump(field_id: &SingleCgroupModelFieldId) -> RenderConfig {
        use common::util::get_prefix;
        use model::CgroupCpuModelFieldId::ThrottledPct;
        use model::CgroupIoModelFieldId::CostIndebtPct;
        use model::CgroupIoModelFieldId::CostIndelayPct;
        use model::CgroupIoModelFieldId::CostUsagePct;
        use model::CgroupIoModelFieldId::CostWaitPct;
        use model::CgroupIoModelFieldId::DbytesPerSec;
        use model::CgroupIoModelFieldId::DiosPerSec;
        use model::CgroupIoModelFieldId::RbytesPerSec;
        use model::CgroupIoModelFieldId::RiosPerSec;
        use model::CgroupIoModelFieldId::RwbytesPerSec;
        use model::CgroupIoModelFieldId::WbytesPerSec;
        use model::CgroupIoModelFieldId::WiosPerSec;
        use model::CgroupMemoryModelFieldId::Anon;
        use model::CgroupMemoryModelFieldId::File;
        use model::CgroupMemoryModelFieldId::Pgactivate;
        use model::CgroupMemoryModelFieldId::Pgdeactivate;
        use model::CgroupMemoryModelFieldId::Pgfault;
        use model::CgroupMemoryModelFieldId::Pglazyfree;
        use model::CgroupMemoryModelFieldId::Pglazyfreed;
        use model::CgroupMemoryModelFieldId::Pgmajfault;
        use model::CgroupMemoryModelFieldId::Pgrefill;
        use model::CgroupMemoryModelFieldId::Pgscan;
        use model::CgroupMemoryModelFieldId::Pgsteal;
        use model::CgroupMemoryModelFieldId::Shmem;
        use model::CgroupMemoryModelFieldId::Slab;
        use model::CgroupMemoryModelFieldId::Sock;
        use model::CgroupMemoryModelFieldId::Swap;
        use model::CgroupMemoryModelFieldId::ThpCollapseAlloc;
        use model::CgroupMemoryModelFieldId::ThpFaultAlloc;
        use model::CgroupMemoryModelFieldId::Total;
        use model::CgroupMemoryModelFieldId::WorkingsetActivateAnon;
        use model::CgroupMemoryModelFieldId::WorkingsetActivateFile;
        use model::CgroupMemoryModelFieldId::WorkingsetNodereclaim;
        use model::CgroupMemoryModelFieldId::WorkingsetRefaultAnon;
        use model::CgroupMemoryModelFieldId::WorkingsetRefaultFile;
        use model::CgroupMemoryModelFieldId::WorkingsetRestoreAnon;
        use model::CgroupMemoryModelFieldId::WorkingsetRestoreFile;
        use model::CgroupMemoryModelFieldId::Zswap;
        use model::CgroupMemoryModelFieldId::Zswapped;
        use model::CgroupPressureModelFieldId::MemoryFullPct;
        use model::CgroupPressureModelFieldId::MemorySomePct;
        use model::SingleCgroupModelFieldId::Cpu;
        use model::SingleCgroupModelFieldId::Io;
        use model::SingleCgroupModelFieldId::Mem;
        use model::SingleCgroupModelFieldId::Name;
        use model::SingleCgroupModelFieldId::Pressure;

        let rc = model::SingleCgroupModel::get_render_config_builder(field_id);
        match field_id {
            Name => rc.indented_prefix(get_prefix(false)),
            Cpu(ThrottledPct) => rc.title("Throttled Pct"),
            Io(RbytesPerSec) => rc.title("RBytes"),
            Io(WbytesPerSec) => rc.title("WBytes"),
            Io(DbytesPerSec) => rc.title("DBytes"),
            Io(RiosPerSec) => rc.title("R I/O"),
            Io(WiosPerSec) => rc.title("W I/O"),
            Io(DiosPerSec) => rc.title("D I/O"),
            Io(RwbytesPerSec) => rc.title("RW Total"),
            Io(CostUsagePct) => rc.title("Cost Usage"),
            Io(CostWaitPct) => rc.title("Cost Wait"),
            Io(CostIndebtPct) => rc.title("Cost Indebt"),
            Io(CostIndelayPct) => rc.title("Cost Indelay"),
            Mem(Total) => rc.title("Mem Total"),
            Mem(Swap) => rc.title("Mem Swap"),
            Mem(Anon) => rc.title("Mem Anon"),
            Mem(File) => rc.title("Mem File"),
            Mem(Slab) => rc.title("Mem Slab"),
            Mem(Sock) => rc.title("Mem Sock"),
            Mem(Shmem) => rc.title("Mem Shmem"),
            Mem(Zswap) => rc.title("Mem Zswap"),
            Mem(Zswapped) => rc.title("Mem Zswapped"),
            Mem(Pgfault) => rc.title("Pgfault"),
            Mem(Pgmajfault) => rc.title("Pgmajfault"),
            Mem(WorkingsetRefaultAnon) => rc.title("Workingset Refault Anon"),
            Mem(WorkingsetRefaultFile) => rc.title("Workingset Refault File"),
            Mem(WorkingsetActivateAnon) => rc.title("Workingset Activate Anon"),
            Mem(WorkingsetActivateFile) => rc.title("Workingset Activate File"),
            Mem(WorkingsetRestoreAnon) => rc.title("Workingset Restore Anon"),
            Mem(WorkingsetRestoreFile) => rc.title("Workingset Restore File"),
            Mem(WorkingsetNodereclaim) => rc.title("Workingset Nodereclaim"),
            Mem(Pgrefill) => rc.title("Pgrefill"),
            Mem(Pgscan) => rc.title("Pgscan"),
            Mem(Pgsteal) => rc.title("Pgsteal"),
            Mem(Pgactivate) => rc.title("Pgactivate"),
            Mem(Pgdeactivate) => rc.title("Pgdeactivate"),
            Mem(Pglazyfree) => rc.title("Pglazyfree"),
            Mem(Pglazyfreed) => rc.title("Pglazyfreed"),
            Mem(ThpFaultAlloc) => rc.title("THP Fault Alloc"),
            Mem(ThpCollapseAlloc) => rc.title("THP Collapse Alloc"),
            Pressure(MemorySomePct) => rc.title("Mem Some Pressure"),
            Pressure(MemoryFullPct) => rc.title("Mem Pressure"),
            _ => rc,
        }
        .get()
    }

    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::CgroupCpuModelFieldId::*;
        use model::CgroupIoModelFieldId::*;
        use model::CgroupMemoryModelFieldId::*;
        use model::CgroupPidsModelFieldId::*;
        use model::CgroupPressureModelFieldId::*;
        use model::CgroupStatModelFieldId::*;
        use model::SingleCgroupModelFieldId::*;

        let counter = counter().label("cgroup", &self.full_path);
        let gauge = gauge().label("cgroup", &self.full_path);
        match field_id {
            // We only use full path for the label to avoid ambiguity
            Name => None,
            // We will label each metric with the full path
            FullPath => None,
            // Not sure what to do with static fields like inode number so leave out for now
            InodeNumber => None,
            Cpu(field_id) => match field_id {
                UsagePct => Some(gauge.unit("percent")),
                UserPct => Some(gauge.unit("percent")),
                SystemPct => Some(gauge.unit("percent")),
                NrPeriodsPerSec => Some(gauge),
                NrThrottledPerSec => Some(gauge),
                ThrottledPct => Some(gauge.unit("percent")),
            },
            Pids(field_id) => match field_id {
                TidsCurrent => Some(counter.unit("count")),
            },
            Io(field_id) => match field_id {
                RbytesPerSec => Some(gauge.unit("bytes_per_second")),
                WbytesPerSec => Some(gauge.unit("bytes_per_second")),
                RiosPerSec => Some(gauge),
                WiosPerSec => Some(gauge),
                DbytesPerSec => Some(gauge.unit("bytes_per_second")),
                DiosPerSec => Some(gauge),
                RwbytesPerSec => Some(gauge.unit("bytes_per_second")),
                CostUsagePct => Some(gauge.unit("percent")),
                CostWaitPct => Some(gauge.unit("percent")),
                CostIndebtPct => Some(gauge.unit("percent")),
                CostIndelayPct => Some(gauge.unit("percent")),
            },
            Mem(field_id) => match field_id {
                Total => Some(counter.unit("bytes")),
                Swap => Some(counter.unit("bytes")),
                // Not sure what to do about min/low/high/max values b/c they're neither
                // counters nor gauges. So leave out for now.
                EventsLow => None,
                EventsHigh => None,
                EventsMax => None,
                EventsOom => Some(counter),
                EventsOomKill => Some(counter),
                Anon => Some(gauge.unit("bytes")),
                File => Some(gauge.unit("bytes")),
                Kernel => Some(gauge.unit("bytes")),
                KernelStack => Some(gauge.unit("bytes")),
                Slab => Some(gauge.unit("bytes")),
                Sock => Some(gauge.unit("bytes")),
                Shmem => Some(gauge.unit("bytes")),
                Zswap => Some(counter.unit("bytes")),
                Zswapped => Some(gauge.unit("bytes")),
                FileMapped => Some(gauge.unit("bytes")),
                FileDirty => Some(gauge.unit("bytes")),
                FileWriteback => Some(gauge.unit("bytes")),
                AnonThp => Some(gauge.unit("bytes")),
                InactiveAnon => Some(gauge.unit("bytes")),
                ActiveAnon => Some(gauge.unit("bytes")),
                InactiveFile => Some(gauge.unit("bytes")),
                ActiveFile => Some(gauge.unit("bytes")),
                Unevictable => Some(gauge.unit("bytes")),
                SlabReclaimable => Some(gauge.unit("bytes")),
                SlabUnreclaimable => Some(gauge.unit("bytes")),
                Pgfault => Some(gauge.help("Page faults per second")),
                Pgmajfault => Some(gauge.help("Major page faults per second")),
                WorkingsetRefaultAnon => Some(gauge.help("Workingset refault anon per second")),
                WorkingsetRefaultFile => Some(gauge.help("Workingset refault file per second")),
                WorkingsetActivateAnon => Some(gauge.help("Workingset activate anon per second")),
                WorkingsetActivateFile => Some(gauge.help("Workingset activate file per second")),
                WorkingsetRestoreAnon => Some(gauge.help("Workingset restore anon per second")),
                WorkingsetRestoreFile => Some(gauge.help("Workingset restore file per second")),
                WorkingsetNodereclaim => Some(gauge.help("Workingset nodereclaim per second")),
                Pgrefill => Some(gauge.help("Pgrefill per second")),
                Pgscan => Some(gauge.help("Pgscan per second")),
                Pgsteal => Some(gauge.help("Pgsteal per second")),
                Pgactivate => Some(gauge.help("Pgactivate per second")),
                Pgdeactivate => Some(gauge.help("Pgdeactivate per second")),
                Pglazyfree => Some(gauge.help("Pglazyfree per second")),
                Pglazyfreed => Some(gauge.help("Pglazyfreed per second")),
                ThpFaultAlloc => Some(gauge.help("THP Fault Alloc per second")),
                ThpCollapseAlloc => Some(gauge.help("THP Collapse Alloc per second")),
            },
            Pressure(field_id) => match field_id {
                CpuSomePct => Some(gauge.unit("percent")),
                CpuFullPct => Some(gauge.unit("percent")),
                IoSomePct => Some(gauge.unit("percent")),
                IoFullPct => Some(gauge.unit("percent")),
                MemorySomePct => Some(gauge.unit("percent")),
                MemoryFullPct => Some(gauge.unit("percent")),
            },
            CgroupStat(field_id) => match field_id {
                NrDescendants => Some(counter),
                NrDyingDescendants => Some(counter),
            },
            // Unclear how to represent numa nodes. Doesn't seem super useful so leave out for now.
            MemNuma(_) => None,
            // These are all settings rather than counters/gauges, so not sure how to represent
            // these. Leave out for now.
            Props(_) => None,
            // Looks like these represent child IO data. Not sure it's necessary to report this
            // as dump does not even pretend to form a hierarchy.
            IoDetails(_) => None,
        }
    }
}

impl HasRenderConfig for model::CgroupCpuModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupCpuModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            UsagePct => rc.title("CPU Usage").suffix("%").format(Precision(2)),
            UserPct => rc.title("CPU User").suffix("%").format(Precision(2)),
            SystemPct => rc.title("CPU Sys").suffix("%").format(Precision(2)),
            NrPeriodsPerSec => rc.title("Nr Period").suffix("/s").format(Precision(2)),
            NrThrottledPerSec => rc.title("Nr Throttled").suffix("/s").format(Precision(2)),
            ThrottledPct => rc.title("Throttled").suffix("%").format(Precision(2)),
        }
    }
}

impl HasRenderConfig for model::CgroupPidsModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupPidsModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            TidsCurrent => rc.title("Tids Current").format(Precision(1)),
        }
    }
}

impl HasRenderConfig for model::CgroupIoModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupIoModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            RbytesPerSec => rc.title("Reads").suffix("/s").format(ReadableSize),
            WbytesPerSec => rc.title("Writes").suffix("/s").format(ReadableSize),
            RiosPerSec => rc.title("Read IOPS").format(Precision(1)),
            WiosPerSec => rc.title("Write IOPS").format(Precision(1)),
            DbytesPerSec => rc.title("Discards").suffix("/s").format(ReadableSize),
            DiosPerSec => rc.title("Discard IOPS").format(Precision(1)),
            RwbytesPerSec => rc.title("RW Total").suffix("/s").format(ReadableSize),
            CostUsagePct => rc.title("Cost Usage").suffix("%").format(Precision(2)),
            CostWaitPct => rc.title("Cost Wait").suffix("%").format(Precision(2)),
            CostIndebtPct => rc.title("Cost Indebt").suffix("%").format(Precision(2)),
            CostIndelayPct => rc.title("Cost Indelay").suffix("%").format(Precision(2)),
        }
    }
}

impl HasRenderConfig for model::CgroupMemoryModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupMemoryModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Total => rc.title("Mem").format(ReadableSize),
            Swap => rc.title("Mem Swap").format(ReadableSize),
            EventsLow => rc.title("Events Low"),
            EventsHigh => rc.title("Events High"),
            EventsMax => rc.title("Events Max"),
            EventsOom => rc.title("Events OOM"),
            EventsOomKill => rc.title("Events Kill"),
            Anon => rc.title("Anon").format(ReadableSize),
            File => rc.title("File").format(ReadableSize),
            Kernel => rc.title("Kernel").format(ReadableSize),
            KernelStack => rc.title("Kernel Stack").format(ReadableSize),
            Slab => rc.title("Slab").format(ReadableSize),
            Sock => rc.title("Sock").format(ReadableSize),
            Shmem => rc.title("Shmem").format(ReadableSize),
            Zswap => rc.title("Zswap").format(ReadableSize),
            Zswapped => rc.title("Zswapped").format(ReadableSize),
            FileMapped => rc.title("File Mapped").format(ReadableSize),
            FileDirty => rc.title("File Dirty").format(ReadableSize),
            FileWriteback => rc.title("File WB").format(ReadableSize),
            AnonThp => rc.title("Anon THP").format(ReadableSize),
            InactiveAnon => rc.title("Inactive Anon").format(ReadableSize),
            ActiveAnon => rc.title("Active Anon").format(ReadableSize),
            InactiveFile => rc.title("Inactive File").format(ReadableSize),
            ActiveFile => rc.title("Active File").format(ReadableSize),
            Unevictable => rc.title("Unevictable").format(ReadableSize),
            SlabReclaimable => rc.title("Slab Reclaimable").format(ReadableSize),
            SlabUnreclaimable => rc.title("Slab Unreclaimable").format(ReadableSize),
            Pgfault => rc.title("Pgfault/s"),
            Pgmajfault => rc.title("Pgmajfault/s"),
            WorkingsetRefaultAnon => rc.title("WS Rflt Anon/s"),
            WorkingsetRefaultFile => rc.title("WS Rflt File/s"),
            WorkingsetActivateAnon => rc.title("WS Actv Anon/s"),
            WorkingsetActivateFile => rc.title("WS Actv File/s"),
            WorkingsetRestoreAnon => rc.title("WS Rstr Anon/s"),
            WorkingsetRestoreFile => rc.title("WS Rstr File/s"),
            WorkingsetNodereclaim => rc.title("WS Nodereclaim/s"),
            Pgrefill => rc.title("Pgrefill/s"),
            Pgscan => rc.title("Pgscan/s"),
            Pgsteal => rc.title("Pgsteal/s"),
            Pgactivate => rc.title("Pgactivate/s"),
            Pgdeactivate => rc.title("Pgdeactivate/s"),
            Pglazyfree => rc.title("Pglazyfree/s"),
            Pglazyfreed => rc.title("Pglazyfreed/s"),
            ThpFaultAlloc => rc.title("THP Fault Alloc/s"),
            ThpCollapseAlloc => rc.title("THP Collapse Alloc/s"),
        }
    }
}

impl HasRenderConfig for model::CgroupPressureModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupPressureModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            CpuSomePct => rc
                .title("CPU Some Pressure")
                .suffix("%")
                .format(Precision(2)),
            CpuFullPct => rc.title("CPU Pressure").suffix("%").format(Precision(2)),
            IoSomePct => rc
                .title("I/O Some Pressure")
                .suffix("%")
                .format(Precision(2)),
            IoFullPct => rc.title("I/O Pressure").suffix("%").format(Precision(2)),
            MemorySomePct => rc
                .title("Mem Some Pressure")
                .suffix("%")
                .format(Precision(2)),
            MemoryFullPct => rc.title("Mem Pressure").suffix("%").format(Precision(2)),
        }
    }
}

impl HasRenderConfig for model::NetworkModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::NetworkModelFieldId::*;
        match field_id {
            Interfaces(field_id) => {
                model::SingleNetModel::get_render_config_builder(&field_id.subquery_id)
            }
            Tcp(field_id) => model::TcpModel::get_render_config_builder(field_id),
            Ip(field_id) => model::IpModel::get_render_config_builder(field_id),
            Ip6(field_id) => model::Ip6Model::get_render_config_builder(field_id),
            Icmp(field_id) => model::IcmpModel::get_render_config_builder(field_id),
            Icmp6(field_id) => model::Icmp6Model::get_render_config_builder(field_id),
            Udp(field_id) => model::UdpModel::get_render_config_builder(field_id),
            Udp6(field_id) => model::Udp6Model::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::NetworkModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::NetworkModelFieldId::*;
        match field_id {
            // Do not dump interfaces from network model b/c there is already the `iface` category.
            // It is also hard to figure out exactly which interface is being queried from here.
            // Meaning we cannot label the metric with the proper interface.
            Interfaces(_) => None,
            Tcp(field_id) => self.tcp.get_openmetrics_config_for_dump(field_id),
            Ip(field_id) => self.ip.get_openmetrics_config_for_dump(field_id),
            Ip6(field_id) => self.ip6.get_openmetrics_config_for_dump(field_id),
            Icmp(field_id) => self.icmp.get_openmetrics_config_for_dump(field_id),
            Icmp6(field_id) => self.icmp6.get_openmetrics_config_for_dump(field_id),
            Udp(field_id) => self.udp.get_openmetrics_config_for_dump(field_id),
            Udp6(field_id) => self.udp6.get_openmetrics_config_for_dump(field_id),
        }
    }
}

impl HasRenderConfig for model::TcpModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::TcpModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            ActiveOpensPerSec => rc.title("TcpActiveOpens/s"),
            PassiveOpensPerSec => rc.title("TcpPassiveOpens/s"),
            AttemptFailsPerSec => rc.title("TcpAttemptFails/s"),
            EstabResetsPerSec => rc.title("TcpEstabResets/s"),
            CurrEstabConn => rc.title("CurEstabConn"),
            InSegsPerSec => rc.title("TcpInSegs/s").suffix(" segs"),
            OutSegsPerSec => rc.title("TcpOutSegs/s").suffix(" segs"),
            RetransSegsPerSec => rc.title("TcpRetransSegs/s").suffix(" segs"),
            RetransSegs => rc.title("TcpRetransSegs").suffix(" segs"),
            InErrs => rc.title("TcpInErrors"),
            OutRstsPerSec => rc.title("TcpOutRsts/s"),
            InCsumErrors => rc.title("TcpInCsumErrors"),
        }
    }
}

impl HasRenderConfigForDump for model::TcpModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::TcpModelFieldId::*;
        match field_id {
            ActiveOpensPerSec => Some(gauge().help("Active opens per second")),
            PassiveOpensPerSec => Some(gauge().help("Passive opens per second")),
            AttemptFailsPerSec => Some(gauge().help("Failed attempts per second")),
            EstabResetsPerSec => Some(gauge()),
            CurrEstabConn => Some(gauge().help("Current established connections")),
            InSegsPerSec => Some(gauge()),
            OutSegsPerSec => Some(gauge()),
            RetransSegsPerSec => Some(gauge()),
            RetransSegs => Some(gauge()),
            InErrs => Some(counter()),
            OutRstsPerSec => Some(gauge()),
            InCsumErrors => Some(counter().help("Ingress checksum errors")),
        }
    }
}

impl HasRenderConfig for model::IpModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::IpModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            ForwardingPktsPerSec => rc.title("IpForwPkts/s").suffix(" pkts"),
            InReceivesPktsPerSec => rc.title("IpInPkts/s").suffix(" pkts"),
            ForwDatagramsPerSec => rc.title("IpForwDatagrams/s"),
            InDiscardsPktsPerSec => rc.title("IpInDiscardPkts/s").suffix(" pkts"),
            InDeliversPktsPerSec => rc.title("IpInDeliversPkts/s").suffix(" pkts"),
            OutRequestsPerSec => rc.title("IpOutReqs/s").suffix(" reqs"),
            OutDiscardsPktsPerSec => rc.title("IpOutDiscardPkts/s").suffix(" pkts"),
            OutNoRoutesPktsPerSec => rc.title("IpOutNoRoutesPkts/s").suffix(" pkts"),
            // IpExt
            InMcastPktsPerSec => rc.title("IpInMcastPkts/s").suffix(" pkts"),
            OutMcastPktsPerSec => rc.title("IpOutMcastPkts/s").suffix(" pkts"),
            InBcastPktsPerSec => rc.title("IpInBcastPkts/s").suffix(" pkts"),
            OutBcastPktsPerSec => rc.title("IpOutBcastPkts/s").suffix(" pkts"),
            InOctetsPerSec => rc.title("IpInOctets/s").suffix(" octets"),
            OutOctetsPerSec => rc.title("IpOutOctets/s").suffix(" octets"),
            InMcastOctetsPerSec => rc.title("IpInMcastOctets/s").suffix(" octets"),
            OutMcastOctetsPerSec => rc.title("IpOutMcastOctets/s").suffix(" octets"),
            InBcastOctetsPerSec => rc.title("IpInBcastOctets/s").suffix(" octets"),
            OutBcastOctetsPerSec => rc.title("IpOutBcastOctets/s").suffix(" octets"),
            InNoEctPktsPerSec => rc.title("IpInNoEctPkts/s").suffix(" pkts"),
        }
    }
}

impl HasRenderConfigForDump for model::IpModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::IpModelFieldId::*;
        match field_id {
            ForwardingPktsPerSec => Some(gauge().help("Forwarded packets per second")),
            InReceivesPktsPerSec => Some(gauge()),
            ForwDatagramsPerSec => Some(gauge().help("Forwarded datagrams per second")),
            InDiscardsPktsPerSec => Some(gauge()),
            InDeliversPktsPerSec => Some(gauge().help("Locally delivered packets per second")),
            OutRequestsPerSec => Some(gauge()),
            OutDiscardsPktsPerSec => Some(gauge()),
            OutNoRoutesPktsPerSec => Some(gauge()),
            InMcastPktsPerSec => Some(gauge()),
            OutMcastPktsPerSec => Some(gauge()),
            InBcastPktsPerSec => Some(gauge()),
            OutBcastPktsPerSec => Some(gauge()),
            InOctetsPerSec => Some(gauge()),
            OutOctetsPerSec => Some(gauge()),
            InMcastOctetsPerSec => Some(gauge()),
            OutMcastOctetsPerSec => Some(gauge()),
            InBcastOctetsPerSec => Some(gauge()),
            OutBcastOctetsPerSec => Some(gauge()),
            InNoEctPktsPerSec => Some(gauge()),
        }
    }
}

impl HasRenderConfig for model::Ip6Model {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::Ip6ModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            InReceivesPktsPerSec => rc.title("Ip6InPkts/s").suffix(" pkts"),
            InHdrErrors => rc.title("Ip6InHdrErrs"),
            InNoRoutesPktsPerSec => rc.title("Ip6InNoRoutesPkts/s").suffix(" pkts"),
            InAddrErrors => rc.title("Ip6InAddrErrs"),
            InDiscardsPktsPerSec => rc.title("Ip6InDiscardsPkts/s").suffix(" pkts"),
            InDeliversPktsPerSec => rc.title("Ip6InDeliversPkts/s").suffix(" pkts"),
            OutForwDatagramsPerSec => rc.title("Ip6ForwDatagrams/s"),
            OutRequestsPerSec => rc.title("Ip6OutReqs/s").suffix(" reqs"),
            OutNoRoutesPktsPerSec => rc.title("Ip6OutNoRoutesPkts/s").suffix(" pkts"),
            InMcastPktsPerSec => rc.title("Ip6InMcastPkts/s").suffix(" pkts"),
            OutMcastPktsPerSec => rc.title("Ip6OutMcastPkts/s").suffix(" pkts"),
            InOctetsPerSec => rc.title("Ip6InOctets/s").suffix(" octets"),
            OutOctetsPerSec => rc.title("Ip6OutOctets/s").suffix(" octets"),
            InMcastOctetsPerSec => rc.title("Ip6InMcastOctets/s").suffix(" octets"),
            OutMcastOctetsPerSec => rc.title("Ip6OutMcastOctets/s").suffix(" octets"),
            InBcastOctetsPerSec => rc.title("Ip6InBcastOctets/s").suffix(" octets"),
            OutBcastOctetsPerSec => rc.title("Ip6OutBcastOctets/s").suffix(" octets"),
        }
    }
}

impl HasRenderConfigForDump for model::Ip6Model {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::Ip6ModelFieldId::*;
        match field_id {
            InReceivesPktsPerSec => Some(gauge()),
            InHdrErrors => Some(counter()),
            InNoRoutesPktsPerSec => Some(gauge()),
            InAddrErrors => Some(counter()),
            InDiscardsPktsPerSec => Some(gauge()),
            InDeliversPktsPerSec => Some(gauge()),
            OutForwDatagramsPerSec => Some(gauge()),
            OutRequestsPerSec => Some(gauge()),
            OutNoRoutesPktsPerSec => Some(gauge()),
            InMcastPktsPerSec => Some(gauge()),
            OutMcastPktsPerSec => Some(gauge()),
            InOctetsPerSec => Some(gauge()),
            OutOctetsPerSec => Some(gauge()),
            InMcastOctetsPerSec => Some(gauge()),
            OutMcastOctetsPerSec => Some(gauge()),
            InBcastOctetsPerSec => Some(gauge()),
            OutBcastOctetsPerSec => Some(gauge()),
        }
    }
}

impl HasRenderConfig for model::IcmpModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::IcmpModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            InMsgsPerSec => rc.title("IcmpInMsg/s").suffix(" msgs"),
            InErrors => rc.title("IcmpInErrs"),
            InDestUnreachs => rc.title("IcmpInDestUnreachs"),
            OutMsgsPerSec => rc.title("IcmpOutMsg/s").suffix(" msgs"),
            OutErrors => rc.title("IcmpOutErrs"),
            OutDestUnreachs => rc.title("IcmpOutDestUnreachs"),
        }
    }
}

impl HasRenderConfigForDump for model::IcmpModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::IcmpModelFieldId::*;
        match field_id {
            InMsgsPerSec => Some(gauge()),
            InErrors => Some(counter()),
            InDestUnreachs => Some(counter()),
            OutMsgsPerSec => Some(gauge()),
            OutErrors => Some(counter()),
            OutDestUnreachs => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::Icmp6Model {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::Icmp6ModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            InMsgsPerSec => rc.title("Icmp6InMsg/s").suffix(" msgs"),
            InErrors => rc.title("Icmp6InErrs"),
            InDestUnreachs => rc.title("Icmp6InDestUnreachs"),
            OutMsgsPerSec => rc.title("Icmp6OutMsg/s").suffix(" msgs"),
            OutErrors => rc.title("Icmp6OutErrs"),
            OutDestUnreachs => rc.title("Icmp6OutDestUnreachs"),
        }
    }
}

impl HasRenderConfigForDump for model::Icmp6Model {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::Icmp6ModelFieldId::*;
        match field_id {
            InMsgsPerSec => Some(gauge()),
            InErrors => Some(counter()),
            InDestUnreachs => Some(counter()),
            OutMsgsPerSec => Some(gauge()),
            OutErrors => Some(counter()),
            OutDestUnreachs => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::UdpModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::UdpModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            InDatagramsPktsPerSec => rc.title("UdpInPkts/s").suffix(" pkts"),
            NoPorts => rc.title("UdpNoPorts"),
            InErrors => rc.title("UdpInErrs"),
            OutDatagramsPktsPerSec => rc.title("UdpOutPkts/s").suffix(" pkts"),
            RcvbufErrors => rc.title("UdpRcvbufErrs"),
            SndbufErrors => rc.title("UdpSndBufErrs"),
            IgnoredMulti => rc.title("UdpIgnoredMulti"),
        }
    }
}

impl HasRenderConfigForDump for model::UdpModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::UdpModelFieldId::*;
        match field_id {
            InDatagramsPktsPerSec => Some(gauge()),
            NoPorts => Some(counter()),
            InErrors => Some(counter()),
            OutDatagramsPktsPerSec => Some(gauge()),
            RcvbufErrors => Some(counter()),
            SndbufErrors => Some(counter()),
            IgnoredMulti => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::Udp6Model {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::Udp6ModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            InDatagramsPktsPerSec => rc.title("Udp6InPkts/s").suffix(" pkts"),
            NoPorts => rc.title("Udp6NoPorts"),
            InErrors => rc.title("Udp6InErrs"),
            OutDatagramsPktsPerSec => rc.title("Udp6OutPkts/s").suffix(" pkts"),
            RcvbufErrors => rc.title("Udp6RcvbufErrs"),
            SndbufErrors => rc.title("Udp6SndBufErrs"),
            InCsumErrors => rc.title("Udp6InCsumErrs"),
            IgnoredMulti => rc.title("Udp6IgnoredMulti"),
        }
    }
}

impl HasRenderConfigForDump for model::Udp6Model {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::Udp6ModelFieldId::*;
        match field_id {
            InDatagramsPktsPerSec => Some(gauge()),
            NoPorts => Some(counter()),
            InErrors => Some(counter()),
            OutDatagramsPktsPerSec => Some(gauge()),
            RcvbufErrors => Some(counter()),
            SndbufErrors => Some(counter()),
            InCsumErrors => Some(counter()),
            IgnoredMulti => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::SingleNetModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleNetModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Interface => rc.title("Interface"),
            RxBytesPerSec => rc.title("RX Bytes/s").format(ReadableSize),
            TxBytesPerSec => rc.title("TX Bytes/s").format(ReadableSize),
            ThroughputPerSec => rc.title("I/O Bytes/s").format(ReadableSize),
            RxPacketsPerSec => rc.title("RX Pkts/s"),
            TxPacketsPerSec => rc.title("TX Pkts/s"),
            Collisions => rc.title("Collisions"),
            Multicast => rc.title("Multicast"),
            RxBytes => rc.title("RX Bytes"),
            RxCompressed => rc.title("RX Compressed"),
            RxCrcErrors => rc.title("RX CRC Errors"),
            RxDropped => rc.title("RX Dropped"),
            RxErrors => rc.title("RX Errors"),
            RxFifoErrors => rc.title("RX Fifo Errors"),
            RxFrameErrors => rc.title("RX Frame Errors"),
            RxLengthErrors => rc.title("RX Length Errors"),
            RxMissedErrors => rc.title("RX Missed Errors"),
            RxNohandler => rc.title("RX Nohandler"),
            RxOverErrors => rc.title("RX Over Errors"),
            RxPackets => rc.title("RX Packets"),
            TxAbortedErrors => rc.title("TX Aborted Errors"),
            TxBytes => rc.title("TX Bytes"),
            TxCarrierErrors => rc.title("TX Carrier Errors"),
            TxCompressed => rc.title("TX Compressed"),
            TxDropped => rc.title("TX Dropped"),
            TxErrors => rc.title("TX Errors"),
            TxFifoErrors => rc.title("TX Fifo Errors"),
            TxHeartbeatErrors => rc.title("TX Heartbeat Errors"),
            TxPackets => rc.title("TX Packets"),
            TxWindowErrors => rc.title("TX Window Errors"),
            TxTimeoutPerSec => rc.title("TX Timeout").suffix("/s"),
            RawStats => rc.title("Raw Stats"),
            Queues(field_id) => Vec::<model::SingleQueueModel>::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::SingleNetModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleNetModelFieldId::*;
        let counter = counter().label("interface", &self.interface);
        let gauge = gauge().label("interface", &self.interface);
        match field_id {
            // We label all the other metrics with the interface name
            Interface => None,
            RxBytesPerSec => Some(gauge),
            TxBytesPerSec => Some(gauge),
            ThroughputPerSec => Some(gauge),
            RxPacketsPerSec => Some(gauge),
            TxPacketsPerSec => Some(gauge),
            Collisions => Some(counter),
            Multicast => Some(counter),
            RxBytes => Some(counter),
            RxCompressed => Some(counter),
            RxCrcErrors => Some(counter),
            RxDropped => Some(counter),
            RxErrors => Some(counter),
            RxFifoErrors => Some(counter),
            RxFrameErrors => Some(counter),
            RxLengthErrors => Some(counter),
            RxMissedErrors => Some(counter),
            RxNohandler => Some(counter),
            RxOverErrors => Some(counter),
            RxPackets => Some(counter),
            TxAbortedErrors => Some(counter),
            TxBytes => Some(counter),
            TxCarrierErrors => Some(counter),
            TxCompressed => Some(counter),
            TxDropped => Some(counter),
            TxErrors => Some(counter),
            TxFifoErrors => Some(counter),
            TxHeartbeatErrors => Some(counter),
            TxPackets => Some(counter),
            TxWindowErrors => Some(counter),
            TxTimeoutPerSec => Some(gauge),
            RawStats => Some(counter),
            Queues(field_id) => self.queues.get_openmetrics_config_for_dump(field_id),
        }
    }
}

impl HasRenderConfig for Vec<model::SingleQueueModel> {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        let mut rc =
            model::SingleQueueModel::get_render_config_builder(&field_id.subquery_id).get();
        rc.title = rc.title.map(|title| title.to_string());
        rc.into()
    }
}

impl HasRenderConfigForDump for Vec<model::SingleQueueModel> {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        let idx = field_id
            .idx
            .expect("VecFieldId without index should not have render config");
        self.get(idx)
            .map(|queue| queue.get_openmetrics_config_for_dump(&field_id.subquery_id))?
    }
}

impl HasRenderConfig for model::SingleQueueModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleQueueModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Interface => rc.title("Interface"),
            QueueId => rc.title("Queue"),
            RxBytesPerSec => rc.title("RxBytes").suffix("/s").format(ReadableSize),
            TxBytesPerSec => rc.title("TxBytes").suffix("/s").format(ReadableSize),
            RxCountPerSec => rc.title("RxCount").suffix("/s"),
            TxCountPerSec => rc.title("TxCount").suffix("/s"),
            TxMissedTx => rc.title("TxMissedTx"),
            TxUnmaskInterrupt => rc.title("TxUnmaskInterrupt"),
            RawStats => rc.title("RawStats"),
        }
    }
}

impl HasRenderConfigForDump for model::SingleQueueModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleQueueModelFieldId::*;
        let counter = counter()
            .label("interface", &self.interface)
            .label("queue", &self.queue_id.to_string());
        let gauge = gauge()
            .label("interface", &self.interface)
            .label("queue", &self.queue_id.to_string());

        match field_id {
            Interface => None,
            QueueId => None,
            RxBytesPerSec => Some(gauge.unit("bytes_per_second")),
            TxBytesPerSec => Some(gauge.unit("bytes_per_second")),
            RxCountPerSec => Some(gauge),
            TxCountPerSec => Some(gauge),
            TxMissedTx => Some(counter),
            TxUnmaskInterrupt => Some(counter),
            RawStats => Some(counter),
        }
    }
}

impl HasRenderConfig for model::SingleProcessModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleProcessModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Pid => rc.title("Pid"),
            Ppid => rc.title("Ppid"),
            NsTgid => rc.title("NStgid").width(12),
            Comm => rc.title("Comm").width(30),
            State => rc.title("State"),
            UptimeSecs => rc.title("Uptime(sec)"),
            Cgroup => rc.title("Cgroup").width(50).fold(FoldOption::Name),
            Io(field_id) => model::ProcessIoModel::get_render_config_builder(field_id),
            Mem(field_id) => model::ProcessMemoryModel::get_render_config_builder(field_id),
            Cpu(field_id) => model::ProcessCpuModel::get_render_config_builder(field_id),
            Cmdline => rc.title("Cmdline").width(50),
            ExePath => rc.title("Exe Path"),
        }
    }
}

impl HasRenderConfigForDump for model::SingleProcessModel {
    fn get_render_config_for_dump(field_id: &SingleProcessModelFieldId) -> RenderConfig {
        use model::ProcessCpuModelFieldId::SystemPct;
        use model::ProcessCpuModelFieldId::UserPct;
        use model::ProcessIoModelFieldId::RwbytesPerSec;
        use model::SingleProcessModelFieldId::Cpu;
        use model::SingleProcessModelFieldId::Io;

        let rc = model::SingleProcessModel::get_render_config_builder(field_id);
        match field_id {
            Cpu(UserPct) => rc.title("User CPU"),
            Cpu(SystemPct) => rc.title("Sys CPU"),
            Io(RwbytesPerSec) => rc.title("RW"),
            _ => rc,
        }
        .get()
    }

    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::ProcessCpuModelFieldId::*;
        use model::ProcessIoModelFieldId::*;
        use model::ProcessMemoryModelFieldId::*;
        use model::SingleProcessModelFieldId::*;
        let mut counter = counter();
        let mut gauge = gauge();
        if let Some(pid) = &self.pid {
            counter = counter.label("pid", &pid.to_string());
            gauge = gauge.label("pid", &pid.to_string());
        }
        if let Some(comm) = &self.comm {
            counter = counter.label("comm", comm);
            gauge = gauge.label("comm", comm);
        }
        match field_id {
            // We will label all the other metrics with the pid
            Pid => None,
            // Not sure what to do about static values like ppid. Omitting for now.
            Ppid => None,
            // Same as ppid
            NsTgid => None,
            // OpenMetrics does not support strings
            Comm => None,
            // OpenMetrics does not support strings
            State => None,
            UptimeSecs => Some(counter),
            // OpenMetrics does not support strings
            Cgroup => None,
            Io(field_id) => match field_id {
                RbytesPerSec => Some(gauge),
                WbytesPerSec => Some(gauge),
                RwbytesPerSec => Some(gauge),
            },
            Mem(field_id) => match field_id {
                MinorfaultsPerSec => Some(gauge),
                MajorfaultsPerSec => Some(gauge),
                RssBytes => Some(gauge.unit("bytes")),
                VmSize => Some(gauge.unit("bytes")),
                Lock => Some(gauge.unit("bytes")),
                Pin => Some(gauge.unit("bytes")),
                Anon => Some(gauge.unit("bytes")),
                File => Some(gauge.unit("bytes")),
                Shmem => Some(gauge.unit("bytes")),
                Pte => Some(gauge.unit("bytes")),
                Swap => Some(gauge.unit("bytes")),
                HugeTlb => Some(gauge.unit("bytes")),
            },
            Cpu(field_id) => match field_id {
                UsagePct => Some(gauge.unit("percent")),
                UserPct => Some(gauge.unit("percent")),
                SystemPct => Some(gauge.unit("percent")),
                NumThreads => Some(counter),
            },
            // OpenMetrics does not support strings
            Cmdline => None,
            // OpenMetrics does not support strings
            ExePath => None,
        }
    }
}

impl HasRenderConfig for model::ProcessIoModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::ProcessIoModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            RbytesPerSec => rc.title("Reads").suffix("/s").format(ReadableSize),
            WbytesPerSec => rc.title("Writes").suffix("/s").format(ReadableSize),
            RwbytesPerSec => rc.title("RW Total").suffix("/s").format(ReadableSize),
        }
    }
}

impl HasRenderConfig for model::ProcessMemoryModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::ProcessMemoryModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            MinorfaultsPerSec => rc.title("Minflt").format(Precision(2)).suffix("/s"),
            MajorfaultsPerSec => rc.title("Majflt").format(Precision(2)).suffix("/s"),
            RssBytes => rc.title("RSS").format(ReadableSize),
            VmSize => rc.title("VM Size").format(ReadableSize),
            Lock => rc.title("Lock").format(ReadableSize),
            Pin => rc.title("Pin").format(ReadableSize),
            Anon => rc.title("Anon").format(ReadableSize),
            File => rc.title("File").format(ReadableSize),
            Shmem => rc.title("Shmem").format(ReadableSize),
            Pte => rc.title("PTE").format(ReadableSize),
            Swap => rc.title("Swap").format(ReadableSize),
            HugeTlb => rc.title("Huge TLB").format(ReadableSize),
        }
    }
}

impl HasRenderConfig for model::ProcessCpuModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::ProcessCpuModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            UsagePct => rc.title("CPU").format(Precision(2)).suffix("%"),
            UserPct => rc.title("CPU User").format(Precision(2)).suffix("%"),
            SystemPct => rc.title("CPU System").format(Precision(2)).suffix("%"),
            NumThreads => rc.title("Threads"),
        }
    }
}

impl HasRenderConfig for model::SystemModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SystemModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Hostname => rc.title("Hostname").width(20),
            KernelVersion => rc.title("Kernel Version").width(50),
            OsRelease => rc.title("OS Release").width(50),
            Stat(field_id) => model::ProcStatModel::get_render_config_builder(field_id),
            Cpu(field_id) => model::SingleCpuModel::get_render_config_builder(field_id),
            Cpus(field_id) => {
                BTreeMap::<u32, model::SingleCpuModel>::get_render_config_builder(field_id)
            }
            Mem(field_id) => model::MemoryModel::get_render_config_builder(field_id),
            Vm(field_id) => model::VmModel::get_render_config_builder(field_id),
            Slab(field_id) => {
                model::SingleSlabModel::get_render_config_builder(&field_id.subquery_id)
            }
            Disks(field_id) => {
                model::SingleDiskModel::get_render_config_builder(&field_id.subquery_id)
            }
            Btrfs(field_id) => model::BtrfsModel::get_render_config_builder(&field_id.subquery_id),
        }
    }
}

impl HasRenderConfigForDump for model::SystemModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SystemModelFieldId::*;
        match field_id {
            // We tag all metrics with the hostname already
            Hostname => None,
            // OpenMetrics does not support strings
            KernelVersion => None,
            // OpenMetrics does not support strings
            OsRelease => None,
            Stat(field_id) => self.stat.get_openmetrics_config_for_dump(field_id),
            Cpu(field_id) => self.total_cpu.get_openmetrics_config_for_dump(field_id),
            Cpus(field_id) => self.cpus.get_openmetrics_config_for_dump(field_id),
            Mem(field_id) => self.mem.get_openmetrics_config_for_dump(field_id),
            Vm(field_id) => self.vm.get_openmetrics_config_for_dump(field_id),
            Slab(_) => None,
            // Same as with NetworkModel, we leave disk dumping to `disk` category
            Disks(_) => None,
            // Same as with above, we leave btrfs dumping to `btrfs` category
            Btrfs(_) => None,
        }
    }
}

impl HasRenderConfig for model::ProcStatModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::ProcStatModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            TotalInterruptCt => rc.title("Total Interrupts"),
            ContextSwitches => rc.title("Context Switches"),
            BootTimeEpochSecs => rc.title("Boot Time Epoch"),
            TotalProcesses => rc.title("Total Procs"),
            RunningProcesses => rc.title("Running Procs"),
            BlockedProcesses => rc.title("Blocked Procs"),
        }
    }
}

impl HasRenderConfigForDump for model::ProcStatModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::ProcStatModelFieldId::*;
        match field_id {
            TotalInterruptCt => Some(counter()),
            ContextSwitches => Some(counter()),
            BootTimeEpochSecs => Some(counter()),
            TotalProcesses => Some(gauge()),
            RunningProcesses => Some(gauge()),
            BlockedProcesses => Some(gauge()),
        }
    }
}

impl HasRenderConfig for model::SingleCpuModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleCpuModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Idx => rc.title("Idx"),
            UsagePct => rc.title("Usage").suffix("%").format(Precision(2)),
            UserPct => rc.title("User").suffix("%").format(Precision(2)),
            IdlePct => rc.title("Idle").suffix("%").format(Precision(2)),
            SystemPct => rc.title("System").suffix("%").format(Precision(2)),
            NicePct => rc.title("Nice").suffix("%").format(Precision(2)),
            IowaitPct => rc.title("IOWait").suffix("%").format(Precision(2)),
            IrqPct => rc.title("Irq").suffix("%").format(Precision(2)),
            SoftirqPct => rc.title("SoftIrq").suffix("%").format(Precision(2)),
            StolenPct => rc.title("Stolen").suffix("%").format(Precision(2)),
            GuestPct => rc.title("Guest").suffix("%").format(Precision(2)),
            GuestNicePct => rc.title("Guest Nice").suffix("%").format(Precision(2)),
        }
    }
}

impl HasRenderConfigForDump for model::SingleCpuModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleCpuModelFieldId::*;
        let gauge = gauge().label("cpu", &self.idx.to_string());
        match field_id {
            // We label each metric with the CPU index
            Idx => None,
            UsagePct => Some(gauge),
            UserPct => Some(gauge),
            IdlePct => Some(gauge),
            SystemPct => Some(gauge),
            NicePct => Some(gauge),
            IowaitPct => Some(gauge),
            IrqPct => Some(gauge),
            SoftirqPct => Some(gauge),
            StolenPct => Some(gauge),
            GuestPct => Some(gauge),
            GuestNicePct => Some(gauge),
        }
    }
}

impl HasRenderConfig for BTreeMap<u32, model::SingleCpuModel> {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        let mut rc = model::SingleCpuModel::get_render_config_builder(&field_id.subquery_id).get();
        rc.title = rc.title.map(|title| {
            format!(
                "CPU {} {}",
                field_id
                    .key
                    .expect("BTreeMapFieldId without key should not have render config"),
                title
            )
        });
        rc.into()
    }
}

impl HasRenderConfigForDump for BTreeMap<u32, model::SingleCpuModel> {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        let key = field_id
            .key
            .expect("BTreeMapFieldId without key should not have render config");
        self.get(&key)
            .map(|cpu| cpu.get_openmetrics_config_for_dump(&field_id.subquery_id))?
    }
}

impl HasRenderConfig for model::MemoryModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::MemoryModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Total => rc.title("Total").format(ReadableSize),
            Free => rc.title("Free").format(ReadableSize),
            Available => rc.title("Available").format(ReadableSize),
            Buffers => rc.title("Buffers").format(ReadableSize),
            Cached => rc.title("Cached").format(ReadableSize),
            SwapCached => rc.title("Swap Cached").format(ReadableSize),
            Active => rc.title("Active").format(ReadableSize),
            Inactive => rc.title("Inactive").format(ReadableSize),
            Anon => rc.title("Anon").format(ReadableSize),
            File => rc.title("File").format(ReadableSize),
            Unevictable => rc.title("Unevictable").format(ReadableSize),
            Mlocked => rc.title("Mlocked").format(ReadableSize),
            SwapTotal => rc.title("Swap Total").format(ReadableSize),
            SwapFree => rc.title("Swap Free").format(ReadableSize),
            Dirty => rc.title("Dirty").format(ReadableSize),
            Writeback => rc.title("Writeback").format(ReadableSize),
            AnonPages => rc.title("Anon Pages").format(ReadableSize),
            Mapped => rc.title("Mapped").format(ReadableSize),
            Shmem => rc.title("Shmem").format(ReadableSize),
            Kreclaimable => rc.title("Kreclaimable").format(ReadableSize),
            Slab => rc.title("Slab").format(ReadableSize),
            SlabReclaimable => rc.title("Slab Reclaimable").format(ReadableSize),
            SlabUnreclaimable => rc.title("Slab Unreclaimable").format(ReadableSize),
            KernelStack => rc.title("Kernel Stack").format(ReadableSize),
            PageTables => rc.title("Page Tables").format(ReadableSize),
            AnonHugePagesBytes => rc.title("Anon Huge Pages").format(ReadableSize),
            ShmemHugePagesBytes => rc.title("Shmem Huge Pages").format(ReadableSize),
            FileHugePagesBytes => rc.title("File Huge Pages").format(ReadableSize),
            Hugetlb => rc.title("Hugetlb").format(ReadableSize),
            CmaTotal => rc.title("Cma Total").format(ReadableSize),
            CmaFree => rc.title("Cma Free").format(ReadableSize),
            VmallocTotal => rc.title("Vmalloc Total").format(ReadableSize),
            VmallocUsed => rc.title("Vmalloc Used").format(ReadableSize),
            VmallocChunk => rc.title("Vmalloc Chunk").format(ReadableSize),
            DirectMap4k => rc.title("Direct Map 4K").format(ReadableSize),
            DirectMap2m => rc.title("Direct Map 2M").format(ReadableSize),
            DirectMap1g => rc.title("Direct Map 1G").format(ReadableSize),
        }
    }
}

impl HasRenderConfigForDump for model::MemoryModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::MemoryModelFieldId::*;
        match field_id {
            Total => Some(gauge().unit("bytes")),
            Free => Some(gauge().unit("bytes")),
            Available => Some(gauge().unit("bytes")),
            Buffers => Some(gauge().unit("bytes")),
            Cached => Some(gauge().unit("bytes")),
            SwapCached => Some(gauge().unit("bytes")),
            Active => Some(gauge().unit("bytes")),
            Inactive => Some(gauge().unit("bytes")),
            Anon => Some(gauge().unit("bytes")),
            File => Some(gauge().unit("bytes")),
            Unevictable => Some(gauge().unit("bytes")),
            Mlocked => Some(gauge().unit("bytes")),
            SwapTotal => Some(gauge().unit("bytes")),
            SwapFree => Some(gauge().unit("bytes")),
            Dirty => Some(gauge().unit("bytes")),
            Writeback => Some(gauge().unit("bytes")),
            AnonPages => Some(gauge().unit("bytes")),
            Mapped => Some(gauge().unit("bytes")),
            Shmem => Some(gauge().unit("bytes")),
            Kreclaimable => Some(gauge().unit("bytes")),
            Slab => Some(gauge().unit("bytes")),
            SlabReclaimable => Some(gauge().unit("bytes")),
            SlabUnreclaimable => Some(gauge().unit("bytes")),
            KernelStack => Some(gauge().unit("bytes")),
            PageTables => Some(gauge().unit("bytes")),
            AnonHugePagesBytes => Some(gauge().unit("bytes")),
            ShmemHugePagesBytes => Some(gauge().unit("bytes")),
            FileHugePagesBytes => Some(gauge().unit("bytes")),
            Hugetlb => Some(gauge().unit("bytes")),
            CmaTotal => Some(gauge().unit("bytes")),
            CmaFree => Some(gauge().unit("bytes")),
            VmallocTotal => Some(gauge().unit("bytes")),
            VmallocUsed => Some(gauge().unit("bytes")),
            VmallocChunk => Some(gauge().unit("bytes")),
            DirectMap4k => Some(gauge().unit("bytes")),
            DirectMap2m => Some(gauge().unit("bytes")),
            DirectMap1g => Some(gauge().unit("bytes")),
        }
    }
}

impl HasRenderConfig for model::VmModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::VmModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            PgpginPerSec => rc.title("Page In").format(SectorReadableSize).suffix("/s"),
            PgpgoutPerSec => rc.title("Page Out").format(SectorReadableSize).suffix("/s"),
            PswpinPerSec => rc.title("Swap In").format(PageReadableSize).suffix("/s"),
            PswpoutPerSec => rc.title("Swap Out").format(PageReadableSize).suffix("/s"),
            PgstealKswapd => rc.title("Pgsteal Kswapd").suffix(" pages/s"),
            PgstealDirect => rc.title("Pgsteal Direct").suffix(" pages/s"),
            PgscanKswapd => rc.title("Pgscan Kswapd").suffix(" pages/s"),
            PgscanDirect => rc.title("Pgscan Direct").suffix(" pages/s"),
            OomKill => rc.title("OOM Kills"),
        }
    }
}

impl HasRenderConfigForDump for model::VmModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::VmModelFieldId::*;
        match field_id {
            PgpginPerSec => Some(gauge()),
            PgpgoutPerSec => Some(gauge()),
            PswpinPerSec => Some(gauge()),
            PswpoutPerSec => Some(gauge()),
            PgstealKswapd => Some(counter()),
            PgstealDirect => Some(counter()),
            PgscanKswapd => Some(counter()),
            PgscanDirect => Some(counter()),
            OomKill => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::SingleSlabModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleSlabModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Name => rc.title("Name").width(25),
            ActiveObjs => rc.title("ActiveObjs"),
            NumObjs => rc.title("TotalObjs"),
            ObjSize => rc.title("ObjSize").format(ReadableSize),
            ObjPerSlab => rc.title("Obj/Slab"),
            NumSlabs => rc.title("Slabs"),
            ActiveCaches => rc.title("ActiveCaches"),
            NumCaches => rc.title("TotalCaches"),
            ActiveSize => rc.title("ActiveSize").format(ReadableSize),
            TotalSize => rc.title("TotalSize").format(ReadableSize),
        }
    }
}

impl HasRenderConfigForDump for model::SingleSlabModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleSlabModelFieldId::*;
        match field_id {
            Name => None,
            ActiveObjs => Some(counter()),
            NumObjs => Some(counter()),
            ObjSize => Some(counter()),
            ObjPerSlab => Some(counter()),
            NumSlabs => Some(counter()),
            ActiveCaches => Some(counter()),
            NumCaches => Some(counter()),
            ActiveSize => Some(counter()),
            TotalSize => Some(counter()),
        }
    }
}

impl HasRenderConfig for model::SingleDiskModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleDiskModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Name => rc.title("Name").width(15),
            ReadBytesPerSec => rc.title("Read").format(ReadableSize).suffix("/s"),
            WriteBytesPerSec => rc.title("Write").format(ReadableSize).suffix("/s"),
            DiscardBytesPerSec => rc.title("Discard").format(ReadableSize).suffix("/s"),
            DiskTotalBytesPerSec => rc.title("Disk").format(ReadableSize).suffix("/s"),
            ReadCompleted => rc.title("Read Completed"),
            ReadMerged => rc.title("Read Merged"),
            ReadSectors => rc.title("Read Sectors"),
            TimeSpendReadMs => rc.title("Time Spend Read").suffix(" ms"),
            WriteCompleted => rc.title("Write Completed"),
            WriteMerged => rc.title("Write Merged"),
            WriteSectors => rc.title("Write Sectors"),
            TimeSpendWriteMs => rc.title("Time Spend Write").suffix(" ms"),
            DiscardCompleted => rc.title("Discard Completed"),
            DiscardMerged => rc.title("Discard Merged"),
            DiscardSectors => rc.title("Discard Sectors"),
            TimeSpendDiscardMs => rc.title("Time Spend Discard").suffix(" ms"),
            Major => rc.title("Major").width(7),
            Minor => rc.title("Minor").width(7),
            DiskUsage => rc.title("Disk Usage").suffix("%").format(Precision(2)),
            PartitionSize => rc.title("Partition Size").format(ReadableSize),
            FilesystemType => rc.title("Filesystem Type"),
        }
    }
}

impl HasRenderConfigForDump for model::SingleDiskModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleDiskModelFieldId::*;
        let counter = if let Some(name) = &self.name {
            counter().label("disk", name)
        } else {
            counter()
        };
        let gauge = if let Some(name) = &self.name {
            gauge().label("disk", name)
        } else {
            gauge()
        };
        match field_id {
            // We label the other metrics with the disk name
            Name => None,
            ReadBytesPerSec => Some(gauge.unit("bytes_per_second")),
            WriteBytesPerSec => Some(gauge.unit("bytes_per_second")),
            DiscardBytesPerSec => Some(gauge.unit("bytes_per_second")),
            DiskTotalBytesPerSec => Some(gauge.unit("bytes_per_second")),
            ReadCompleted => Some(counter),
            ReadMerged => Some(counter),
            ReadSectors => Some(counter),
            TimeSpendReadMs => Some(counter.unit("milliseconds")),
            WriteCompleted => Some(counter),
            WriteMerged => Some(counter),
            WriteSectors => Some(counter),
            TimeSpendWriteMs => Some(counter.unit("milliseconds")),
            DiscardCompleted => Some(counter),
            DiscardMerged => Some(counter),
            DiscardSectors => Some(counter),
            TimeSpendDiscardMs => Some(counter.unit("milliseconds")),
            // Not sure what to do about static values like major/minor so leave them out for now
            Major => None,
            Minor => None,
            DiskUsage => Some(gauge.unit("percent")),
            PartitionSize => Some(gauge.unit("bytes")),
            FilesystemType => None,
        }
    }
}

impl HasRenderConfig for model::BtrfsModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::BtrfsModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Name => rc.title("Name").width(100).fold(FoldOption::Path),
            DiskFraction => rc
                .title("Approx Disk Usage")
                .format(Precision(1))
                .suffix("%"),
            DiskBytes => rc.title("Approx Disk Bytes").format(ReadableSize),
        }
    }
}

impl HasRenderConfigForDump for model::BtrfsModel {
    fn get_openmetrics_config_for_dump(
        &self,
        _field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        // Btrfs not supported in open source
        None
    }
}

impl HasRenderConfig for model::CgroupStatModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupStatModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            NrDescendants => rc.title("Nr Descendants"),
            NrDyingDescendants => rc.title("Nr Dying Descendants"),
        }
    }
}

impl HasRenderConfig for model::CgroupMemoryNumaModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupMemoryNumaModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Total => rc.title("Total").format(ReadableSize),
            Anon => rc.title("Anon").format(ReadableSize),
            File => rc.title("File").format(ReadableSize),
            KernelStack => rc.title("KernelStack").format(ReadableSize),
            Pagetables => rc.title("Pagetables").format(ReadableSize),
            Shmem => rc.title("Shmem").format(ReadableSize),
            FileMapped => rc.title("FileMapped").format(ReadableSize),
            FileDirty => rc.title("FileDirty").format(ReadableSize),
            FileWriteback => rc.title("FileWriteback").format(ReadableSize),
            Swapcached => rc.title("Swapcached").format(ReadableSize),
            AnonThp => rc.title("AnonThp").format(ReadableSize),
            FileThp => rc.title("FileThp").format(ReadableSize),
            ShmemThp => rc.title("ShmemThp").format(ReadableSize),
            InactiveAnon => rc.title("InactiveAnon").format(ReadableSize),
            ActiveAnon => rc.title("ActiveAnon").format(ReadableSize),
            InactiveFile => rc.title("InactiveFile").format(ReadableSize),
            ActiveFile => rc.title("ActiveFile").format(ReadableSize),
            Unevictable => rc.title("Unevictable").format(ReadableSize),
            SlabReclaimable => rc.title("SlabReclaimable").format(ReadableSize),
            SlabUnreclaimable => rc.title("SlabUnreclaimable").format(ReadableSize),
            WorkingsetRefaultAnon => rc
                .title("Workingset Refaults Anon")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetRefaultFile => rc
                .title("Workingset Refaults File")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetActivateAnon => rc
                .title("Workingset Activates Anon")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetActivateFile => rc
                .title("Workingset Activates File")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetRestoreAnon => rc
                .title("Workingset Restores Anon")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetRestoreFile => rc
                .title("Workingset Restores File")
                .suffix("/s")
                .format(Precision(1)),
            WorkingsetNodereclaim => rc
                .title("Workingset Nodereclaims")
                .suffix("/s")
                .format(Precision(1)),
        }
    }
}

impl HasRenderConfig for model::CgroupProperties {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::CgroupPropertiesFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            // "cpu cpuset hugetlb io memory pids" is 33 chars
            CgroupControllers => rc.title("Controllers").width(35),
            CgroupSubtreeControl => rc.title("SubtreeControl").width(35),
            TidsMax => rc.title("Tids Max").format(MaxOrReadableSize),
            MemoryMin => rc.title("Mem Min").format(MaxOrReadableSize),
            MemoryLow => rc.title("Mem Low").format(MaxOrReadableSize),
            MemoryHigh => rc.title("Mem High").format(MaxOrReadableSize),
            MemoryMax => rc.title("Mem Max").format(MaxOrReadableSize),
            MemorySwapMax => rc.title("Swap Max").format(MaxOrReadableSize),
            MemoryZswapMax => rc.title("Zswap Max").format(MaxOrReadableSize),
            CpuWeight => rc.title("CPU Weight"),
            CpusetCpus => rc.title("Allowed CPUs"),
            CpusetCpusEffective => rc.title("Effective CPUs"),
            CpusetMems => rc.title("Allowed Mem Nodes"),
            CpusetMemsEffective => rc.title("Effective Mem Nodes"),
            CpuMaxUsec => rc.title("CPU Max").format(MaxOrDuration),
            CpuMaxPeriodUsec => rc.title("CPU Max Period").format(Duration),
        }
    }
}

impl HasRenderConfig for model::SingleTcModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::SingleTcModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Interface => rc.title("Interface"),
            Kind => rc.title("Kind"),
            Qlen => rc.title("Queue Length"),
            Bps => rc.title("Bps").format(ReadableSize).suffix("/s"),
            Pps => rc.title("Pps").suffix("/s"),
            BytesPerSec => rc.title("Bytes").format(ReadableSize).suffix("/s"),
            PacketsPerSec => rc.title("Packets").suffix("/s"),
            BacklogPerSec => rc.title("Backlog").suffix("/s"),
            DropsPerSec => rc.title("Drops").suffix("/s"),
            RequeuesPerSec => rc.title("Requeues").suffix("/s"),
            OverlimitsPerSec => rc.title("Overlimits").suffix("/s"),
            Qdisc(field_id) => model::QDiscModel::get_render_config_builder(field_id),
            Xstats(field_id) => model::XStatsModel::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::SingleTcModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::SingleTcModelFieldId::*;
        let gauge = gauge()
            .label("interface", &self.interface)
            .label("qdisc", &self.kind);
        match field_id {
            Interface => None,
            Kind => None,
            Qlen => Some(gauge),
            Bps => Some(gauge.unit("bytes_per_second")),
            Pps => Some(gauge.unit("packets_per_second")),
            BytesPerSec => Some(gauge.unit("bytes_per_second")),
            PacketsPerSec => Some(gauge.unit("packets_per_second")),
            BacklogPerSec => Some(gauge.unit("packets_per_second")),
            DropsPerSec => Some(gauge.unit("packets_per_second")),
            RequeuesPerSec => Some(gauge.unit("packets_per_second")),
            OverlimitsPerSec => Some(gauge.unit("packets_per_second")),
            Qdisc(field_id) => self
                .qdisc
                .as_ref()
                .and_then(|qdisc| qdisc.get_openmetrics_config_for_dump(field_id)),
            Xstats(field_id) => self
                .xstats
                .as_ref()
                .and_then(|xstats| xstats.get_openmetrics_config_for_dump(field_id)),
        }
    }
}

impl HasRenderConfig for model::QDiscModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::QDiscModelFieldId::*;
        match field_id {
            FqCodel(field_id) => model::FqCodelQDiscModel::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::QDiscModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::QDiscModelFieldId::*;
        match field_id {
            FqCodel(field_id) => self
                .fq_codel
                .as_ref()
                .and_then(|fq_codel| fq_codel.get_openmetrics_config_for_dump(field_id)),
        }
    }
}

impl HasRenderConfig for model::FqCodelQDiscModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::FqCodelQDiscModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Target => rc.title("Target"),
            Limit => rc.title("Limit"),
            Interval => rc.title("Interval"),
            Ecn => rc.title("Ecn"),
            Quantum => rc.title("Quantum"),
            CeThreshold => rc.title("CeThreshold"),
            DropBatchSize => rc.title("DropBatchSize"),
            MemoryLimit => rc.title("MemoryLimit"),
            FlowsPerSec => rc.title("Flows").suffix("/s"),
        }
    }
}

impl HasRenderConfig for model::XStatsModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::XStatsModelFieldId::*;
        match field_id {
            FqCodel(field_id) => model::FqCodelXStatsModel::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfigForDump for model::XStatsModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::XStatsModelFieldId::*;
        match field_id {
            FqCodel(field_id) => self
                .fq_codel
                .as_ref()
                .and_then(|fq_codel| fq_codel.get_openmetrics_config_for_dump(field_id)),
        }
    }
}

impl HasRenderConfigForDump for model::FqCodelQDiscModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::FqCodelQDiscModelFieldId::*;
        match field_id {
            Target => Some(gauge()),
            Limit => Some(gauge()),
            Interval => Some(gauge()),
            Ecn => Some(gauge()),
            Quantum => Some(gauge()),
            CeThreshold => Some(gauge()),
            DropBatchSize => Some(gauge()),
            MemoryLimit => Some(gauge()),
            FlowsPerSec => Some(gauge()),
        }
    }
}

impl HasRenderConfig for model::FqCodelXStatsModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::FqCodelXStatsModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Maxpacket => rc.title("MaxPacket"),
            EcnMark => rc.title("EcnMark"),
            NewFlowsLen => rc.title("NewFlowsLen"),
            OldFlowsLen => rc.title("OldFlowsLen"),
            CeMark => rc.title("CeMark"),
            DropOverlimitPerSec => rc.title("DropOverlimit").suffix("/s"),
            NewFlowCountPerSec => rc.title("NewFlowCount").suffix("/s"),
            MemoryUsagePerSec => rc.title("MemoryUsage").suffix("/s"),
            DropOvermemoryPerSec => rc.title("DropOvermemory").suffix("/s"),
        }
    }
}

impl HasRenderConfigForDump for model::FqCodelXStatsModel {
    fn get_openmetrics_config_for_dump(
        &self,
        field_id: &Self::FieldId,
    ) -> Option<RenderOpenMetricsConfigBuilder> {
        use model::FqCodelXStatsModelFieldId::*;
        let gauge = gauge();
        match field_id {
            Maxpacket => Some(gauge),
            EcnMark => Some(gauge),
            NewFlowsLen => Some(gauge),
            OldFlowsLen => Some(gauge),
            CeMark => Some(gauge),
            DropOverlimitPerSec => Some(gauge),
            NewFlowCountPerSec => Some(gauge),
            MemoryUsagePerSec => Some(gauge),
            DropOvermemoryPerSec => Some(gauge),
        }
    }
}
