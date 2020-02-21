include "resctl/common/cgroupfs/if/cgroupfs.thrift"
include "resctl/common/procfs/if/procfs.thrift"

struct DataFrame {
  1: Sample sample,
}

struct Sample {
  1: CgroupSample cgroup,
  2: procfs.PidMap processes,
  3: SystemSample system,
}

struct CgroupSample {
  1: optional cgroupfs.CpuStat cpu_stat,
  2: optional map<string, cgroupfs.IoStat> io_stat,
  3: optional i64 memory_current,
  4: optional cgroupfs.MemoryStat memory_stat,
  5: optional cgroupfs.Pressure pressure,
  6: optional map<string, CgroupSample> children,
}

struct SystemSample {
  1: procfs.Stat stat,
  2: procfs.MemInfo meminfo,
  3: procfs.VmStat vmstat
  4: string hostname,
}
