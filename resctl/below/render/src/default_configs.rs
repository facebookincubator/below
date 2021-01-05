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

use crate::render_config as rc;

use RenderFormat::{MaxOrReadableSize, Precision, ReadableSize};

impl HasRenderConfig for model::CgroupModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupModelFieldId::*;
        match field_id {
            Name => rc!(title("Name"), width(50)),
            FullPath => rc!(title("Full Path"), width(50)),
            InodeNumber => rc!(title("Inode Number")),
            Cpu(field_id) => model::CgroupCpuModel::get_render_config(field_id),
            Io(field_id) => model::CgroupIoModel::get_render_config(field_id),
            Mem(field_id) => model::CgroupMemoryModel::get_render_config(field_id),
            Pressure(field_id) => model::CgroupPressureModel::get_render_config(field_id),
        }
    }
}

impl HasRenderConfig for model::CgroupCpuModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupCpuModelFieldId::*;
        match field_id {
            UsagePct => rc!(title("CPU Usage"), suffix("%"), format(Precision(2))),
            UserPct => rc!(title("CPU User"), suffix("%"), format(Precision(2))),
            SystemPct => rc!(title("CPU Sys"), suffix("%"), format(Precision(2))),
            NrPeriodsPerSec => rc!(title("Nr Period"), suffix("/s"), format(Precision(2))),
            NrThrottledPerSec => rc!(title("Nr Throttled"), suffix("/s"), format(Precision(2))),
            ThrottledPct => rc!(title("Throttled"), suffix("%"), format(Precision(2))),
        }
    }
}

impl HasRenderConfig for model::CgroupIoModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupIoModelFieldId::*;
        match field_id {
            RbytesPerSec => rc!(title("Reads"), suffix("/s"), format(ReadableSize)),
            WbytesPerSec => rc!(title("Writes"), suffix("/s"), format(ReadableSize)),
            RiosPerSec => rc!(title("Read IOPS"), format(Precision(1))),
            WiosPerSec => rc!(title("Write IOPS"), format(Precision(1))),
            DbytesPerSec => rc!(title("Discards"), suffix("/s"), format(ReadableSize)),
            DiosPerSec => rc!(title("Discard IOPS"), format(Precision(1))),
            RwbytesPerSec => rc!(title("RW Total"), suffix("/s"), format(ReadableSize)),
        }
    }
}

impl HasRenderConfig for model::CgroupMemoryModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupMemoryModelFieldId::*;
        match field_id {
            Total => rc!(title("Memory"), format(ReadableSize)),
            Swap => rc!(title("Memory Swap"), format(ReadableSize)),
            MemoryHigh => rc!(title("Memory High"), format(MaxOrReadableSize)),
            EventsLow => rc!(title("Events Low")),
            EventsHigh => rc!(title("Events High")),
            EventsMax => rc!(title("Events Max")),
            EventsOom => rc!(title("Events OOM")),
            EventsOomKill => rc!(title("Events Kill")),
            Anon => rc!(title("Anon"), format(ReadableSize)),
            File => rc!(title("File"), format(ReadableSize)),
            KernelStack => rc!(title("Kernel Stack"), format(ReadableSize)),
            Slab => rc!(title("Slab"), format(ReadableSize)),
            Sock => rc!(title("Sock"), format(ReadableSize)),
            Shmem => rc!(title("Shmem"), format(ReadableSize)),
            FileMapped => rc!(title("File Mapped"), format(ReadableSize)),
            FileDirty => rc!(title("File Dirty"), format(ReadableSize)),
            FileWriteback => rc!(title("File WB"), format(ReadableSize)),
            AnonThp => rc!(title("Anon THP"), format(ReadableSize)),
            InactiveAnon => rc!(title("Inactive Anon"), format(ReadableSize)),
            ActiveAnon => rc!(title("Active Anon"), format(ReadableSize)),
            InactiveFile => rc!(title("Inactive File"), format(ReadableSize)),
            ActiveFile => rc!(title("Active File"), format(ReadableSize)),
            Unevictable => rc!(title("Unevictable"), format(ReadableSize)),
            SlabReclaimable => rc!(title("Slab Reclaimable"), format(ReadableSize)),
            SlabUnreclaimable => rc!(title("Slab Unreclaimable"), format(ReadableSize)),
            Pgfault => rc!(title("Pgfault/s")),
            Pgmajfault => rc!(title("Pgmajfault/s")),
            WorkingsetRefault => rc!(title("Workingset Refault/s")),
            WorkingsetActivate => rc!(title("Workingset Activate/s")),
            WorkingsetNodereclaim => rc!(title("Workingset Nodereclaim/s")),
            Pgrefill => rc!(title("Pgrefill/s")),
            Pgscan => rc!(title("Pgscan/s")),
            Pgsteal => rc!(title("Pgsteal/s")),
            Pgactivate => rc!(title("Pgactivate/s")),
            Pgdeactivate => rc!(title("Pgdeactivate/s")),
            Pglazyfree => rc!(title("Pglazyfree/s")),
            Pglazyfreed => rc!(title("Pglazyfreed/s")),
            ThpFaultAlloc => rc!(title("THP Fault Alloc/s")),
            ThpCollapseAlloc => rc!(title("THP Collapse Alloc/s")),
        }
    }
}

impl HasRenderConfig for model::CgroupPressureModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupPressureModelFieldId::*;
        match field_id {
            CpuSomePct => rc!(title("CPU Pressure"), suffix("%"), format(Precision(2))),
            IoSomePct => rc!(
                title("I/O Some Pressure"),
                suffix("%"),
                format(Precision(2)),
            ),
            IoFullPct => rc!(title("I/O Pressure"), suffix("%"), format(Precision(2))),
            MemorySomePct => rc!(
                title("Mem Some Pressure"),
                suffix("%"),
                format(Precision(2)),
            ),
            MemoryFullPct => rc!(title("Mem Pressure"), suffix("%"), format(Precision(2))),
        }
    }
}
impl HasRenderConfig for model::NetworkModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::NetworkModelFieldId::*;
        match field_id {
            Tcp(field_id) => model::TcpModel::get_render_config(field_id),
            Ip(field_id) => model::IpModel::get_render_config(field_id),
            Ip6(field_id) => model::Ip6Model::get_render_config(field_id),
            Icmp(field_id) => model::IcmpModel::get_render_config(field_id),
            Icmp6(field_id) => model::Icmp6Model::get_render_config(field_id),
            Udp(field_id) => model::UdpModel::get_render_config(field_id),
            Udp6(field_id) => model::Udp6Model::get_render_config(field_id),
        }
    }
}

impl HasRenderConfig for model::TcpModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::TcpModelFieldId::*;
        match field_id {
            ActiveOpensPerSec => rc!(title("TcpActiveOpens/s")),
            PassiveOpensPerSec => rc!(title("TcpPassiveOpens/s")),
            AttemptFailsPerSec => rc!(title("TcpAttemptFails/s")),
            EstabResetsPerSec => rc!(title("TcpEstabResets/s")),
            CurrEstabConn => rc!(title("CurEstabConn")),
            InSegsPerSec => rc!(title("TcpInSegs/s"), suffix(" segs")),
            OutSegsPerSec => rc!(title("TcpOutSegs/s"), suffix(" segs")),
            RetransSegsPerSec => rc!(title("TcpRetransSegs/s"), suffix(" segs")),
            RetransSegs => rc!(title("TcpRetransSegs"), suffix(" segs")),
            InErrs => rc!(title("TcpInErrors")),
            OutRstsPerSec => rc!(title("TcpOutRsts/s")),
            InCsumErrors => rc!(title("TcpInCsumErrors")),
        }
    }
}

impl HasRenderConfig for model::IpModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::IpModelFieldId::*;
        match field_id {
            ForwardingPktsPerSec => rc!(title("IpForwPkts/s"), suffix(" pkts")),
            InReceivesPktsPerSec => rc!(title("IpInPkts/s"), suffix(" pkts")),
            ForwDatagramsPerSec => rc!(title("IpForwDatagrams/s")),
            InDiscardsPktsPerSec => rc!(title("IpInDiscardPkts/s"), suffix(" pkts")),
            InDeliversPktsPerSec => rc!(title("IpInDeliversPkts/s"), suffix(" pkts")),
            OutRequestsPerSec => rc!(title("IpOutReqs/s"), suffix(" reqs")),
            OutDiscardsPktsPerSec => rc!(title("IpOutDiscardPkts/s"), suffix(" pkts")),
            OutNoRoutesPktsPerSec => rc!(title("IpOutNoRoutesPkts/s"), suffix(" pkts")),
            // IpExt
            InMcastPktsPerSec => rc!(title("IpInMcastPkts/s"), suffix(" pkts")),
            OutMcastPktsPerSec => rc!(title("IpOutMcastPkts/s"), suffix(" pkts")),
            InBcastPktsPerSec => rc!(title("IpInBcastPkts/s"), suffix(" pkts")),
            OutBcastPktsPerSec => rc!(title("IpOutBcastPkts/s"), suffix(" pkts")),
            InOctetsPerSec => rc!(title("IpInOctets/s"), suffix(" octets")),
            OutOctetsPerSec => rc!(title("IpOutOctets/s"), suffix(" octets")),
            InMcastOctetsPerSec => rc!(title("IpInMcastOctets/s"), suffix(" octets")),
            OutMcastOctetsPerSec => rc!(title("IpOutMcastOctets/s"), suffix(" octets")),
            InBcastOctetsPerSec => rc!(title("IpInBcastOctets/s"), suffix(" octets")),
            OutBcastOctetsPerSec => rc!(title("IpOutBcastOctets/s"), suffix(" octets")),
            InNoEctPktsPerSec => rc!(title("IpInNoEctPkts/s"), suffix(" pkts")),
        }
    }
}

impl HasRenderConfig for model::Ip6Model {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::Ip6ModelFieldId::*;
        match field_id {
            InReceivesPktsPerSec => rc!(title("Ip6InPkts/s"), suffix(" pkts")),
            InHdrErrors => rc!(title("Ip6InHdrErrs")),
            InNoRoutesPktsPerSec => rc!(title("Ip6InNoRoutesPkts/s"), suffix(" pkts")),
            InAddrErrors => rc!(title("Ip6InAddrErrs")),
            InDiscardsPktsPerSec => rc!(title("Ip6InDiscardsPkts/s"), suffix(" pkts")),
            InDeliversPktsPerSec => rc!(title("Ip6InDeliversPkts/s"), suffix(" pkts")),
            OutForwDatagramsPerSec => rc!(title("Ip6ForwDatagrams/s")),
            OutRequestsPerSec => rc!(title("Ip6OutReqs/s"), suffix(" reqs")),
            OutNoRoutesPktsPerSec => rc!(title("Ip6OutNoRoutesPkts/s"), suffix(" pkts")),
            InMcastPktsPerSec => rc!(title("Ip6InMcastPkts/s"), suffix(" pkts")),
            OutMcastPktsPerSec => rc!(title("Ip6OutMcastPkts/s"), suffix(" pkts")),
            InOctetsPerSec => rc!(title("Ip6InOctets/s"), suffix(" octets")),
            OutOctetsPerSec => rc!(title("Ip6OutOctets/s"), suffix(" octets")),
            InMcastOctetsPerSec => rc!(title("Ip6InMcastOctets/s"), suffix(" octets")),
            OutMcastOctetsPerSec => rc!(title("Ip6OutMcastOctets/s"), suffix(" octets")),
            InBcastOctetsPerSec => rc!(title("Ip6InBcastOctets/s"), suffix(" octets")),
            OutBcastOctetsPerSec => rc!(title("Ip6OutBcastOctets/s"), suffix(" octets")),
        }
    }
}

impl HasRenderConfig for model::IcmpModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::IcmpModelFieldId::*;
        match field_id {
            InMsgsPerSec => rc!(title("IcmpInMsg/s"), suffix(" msgs")),
            InErrors => rc!(title("IcmpInErrs")),
            InDestUnreachs => rc!(title("IcmpInDestUnreachs")),
            OutMsgsPerSec => rc!(title("IcmpOutMsg/s"), suffix(" msgs")),
            OutErrors => rc!(title("IcmpOutErrs")),
            OutDestUnreachs => rc!(title("IcmpOutDestUnreachs")),
        }
    }
}

impl HasRenderConfig for model::Icmp6Model {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::Icmp6ModelFieldId::*;
        match field_id {
            InMsgsPerSec => rc!(title("Icmp6InMsg/s"), suffix(" msgs")),
            InErrors => rc!(title("Icmp6InErrs")),
            InDestUnreachs => rc!(title("Icmp6InDestUnreachs")),
            OutMsgsPerSec => rc!(title("Icmp6OutMsg/s"), suffix(" msgs")),
            OutErrors => rc!(title("Icmp6OutErrs")),
            OutDestUnreachs => rc!(title("Icmp6OutDestUnreachs")),
        }
    }
}

impl HasRenderConfig for model::UdpModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::UdpModelFieldId::*;
        match field_id {
            InDatagramsPktsPerSec => rc!(title("UdpInPkts/s"), suffix(" pkts")),
            NoPorts => rc!(title("UdpNoPorts")),
            InErrors => rc!(title("UdpInErrs")),
            OutDatagramsPktsPerSec => rc!(title("UdpOutPkts/s"), suffix(" pkts")),
            RcvbufErrors => rc!(title("UdpRcvbufErrs")),
            SndbufErrors => rc!(title("UdpSndBufErrs")),
            IgnoredMulti => rc!(title("UdpIgnoredMulti")),
        }
    }
}

impl HasRenderConfig for model::Udp6Model {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::Udp6ModelFieldId::*;
        match field_id {
            InDatagramsPktsPerSec => rc!(title("Udp6InPkts/s"), suffix(" pkts")),
            NoPorts => rc!(title("Udp6NoPorts")),
            InErrors => rc!(title("Udp6InErrs")),
            OutDatagramsPktsPerSec => rc!(title("Udp6OutPkts/s"), suffix(" pkts")),
            RcvbufErrors => rc!(title("Udp6RcvbufErrs")),
            SndbufErrors => rc!(title("Udp6SndBufErrs")),
            InCsumErrors => rc!(title("Udp6InCsumErrs")),
            IgnoredMulti => rc!(title("Udp6IgnoredMulti")),
        }
    }
}

impl HasRenderConfig for model::SingleNetModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::SingleNetModelFieldId::*;
        match field_id {
            Interface => rc!(title("Interface")),
            RxBytesPerSec => rc!(title("RX Bytes/s"), format(ReadableSize)),
            TxBytesPerSec => rc!(title("TX Bytes/s"), format(ReadableSize)),
            ThroughputPerSec => rc!(title("I/O Bytes/s"), format(ReadableSize)),
            RxPacketsPerSec => rc!(title("RX Pkts/s")),
            TxPacketsPerSec => rc!(title("TX Pkts/s")),
            Collisions => rc!(title("Collisions")),
            Multicast => rc!(title("Multicast")),
            RxBytes => rc!(title("RX Bytes")),
            RxCompressed => rc!(title("RX Compressed")),
            RxCrcErrors => rc!(title("RX CRC Errors")),
            RxDropped => rc!(title("RX Dropped")),
            RxErrors => rc!(title("RX Errors")),
            RxFifoErrors => rc!(title("RX Fifo Errors")),
            RxFrameErrors => rc!(title("RX Frame Errors")),
            RxLengthErrors => rc!(title("RX Length Errors")),
            RxMissedErrors => rc!(title("RX Missed Errors")),
            RxNohandler => rc!(title("RX Nohandler")),
            RxOverErrors => rc!(title("RX Over Errors")),
            RxPackets => rc!(title("RX Packets")),
            TxAbortedErrors => rc!(title("TX Aborted Errors")),
            TxBytes => rc!(title("TX Bytes")),
            TxCarrierErrors => rc!(title("TX Carrier Errors")),
            TxCompressed => rc!(title("TX Compressed")),
            TxDropped => rc!(title("TX Dropped")),
            TxErrors => rc!(title("TX Errors")),
            TxFifoErrors => rc!(title("TX Fifo Errors")),
            TxHeartbeatErrors => rc!(title("TX Heartbeat Errors")),
            TxPackets => rc!(title("TX Packets")),
            TxWindowErrors => rc!(title("TX Window Errors")),
        }
    }
}
