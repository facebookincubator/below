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

use model::SystemModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct SystemData {
    // Syslevel cpu
    #[bttr(dfill_struct = "System")]
    #[bttr(tag = "SysField::Hostname")]
    #[blink("SystemModel$get_hostname")]
    pub hostname: String,
    #[bttr(tag = "SysField::KernelVersion", class = "SysField::Stat")]
    #[blink("SystemModel$")]
    pub kernel_version: Option<String>,
    #[bttr(tag = "SysField::TotalInterruptCt", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_total_interrupt_ct")]
    pub total_interrupt_ct: Option<i64>,
    #[bttr(tag = "SysField::ContextSwitches", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_context_switches")]
    pub context_switches: Option<i64>,
    #[bttr(tag = "SysField::BootTimeEpochSecs", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_boot_time_epoch_secs")]
    pub boot_time_epoch_secs: Option<i64>,
    #[bttr(tag = "SysField::TotalProcs", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_total_processes")]
    pub total_processes: Option<i64>,
    #[bttr(tag = "SysField::RunningProcs", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_running_processes")]
    pub running_processes: Option<i32>,
    #[bttr(tag = "SysField::BlockedProcs", class = "SysField::Stat")]
    #[blink("SystemModel$cpu.get_blocked_processes")]
    pub blocked_processes: Option<i32>,
    // Cpu
    #[bttr(tag = "SysField::CpuUsagePct", class = "SysField::Cpu")]
    #[blink("SystemModel$cpu.total_cpu?.get_usage_pct")]
    pub usage_pct: Option<f64>,
    #[bttr(tag = "SysField::CpuUserPct", class = "SysField::Cpu")]
    #[blink("SystemModel$cpu.total_cpu?.get_user_pct")]
    pub user_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuIdlePct",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_idle_pct")]
    pub idle_pct: Option<f64>,
    #[bttr(tag = "SysField::CpuSystemPct", class = "SysField::Cpu")]
    #[blink("SystemModel$cpu.total_cpu?.get_system_pct")]
    pub system_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuNicePct",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_nice_pct")]
    pub nice_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuIowaitPct",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_iowait_pct")]
    pub iowait_pct: Option<f64>,
    #[bttr(tag = "SysField::CpuIrq", class = "SysField::Cpu", class_detail = true)]
    #[blink("SystemModel$cpu.total_cpu?.get_irq_pct")]
    pub irq_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuSoftIrq",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_softirq_pct")]
    pub softirq_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuStolen",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_stolen_pct")]
    pub stolen_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuGuest",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_guest_pct")]
    pub guest_pct: Option<f64>,
    #[bttr(
        tag = "SysField::CpuGuestNice",
        class = "SysField::Cpu",
        class_detail = true
    )]
    #[blink("SystemModel$cpu.total_cpu?.get_guest_nice_pct")]
    pub guest_nice_pct: Option<f64>,
    // Mem stats
    #[bttr(tag = "SysField::MemTotal", class = "SysField::Mem")]
    #[blink("SystemModel$mem.get_total")]
    pub total: Option<u64>,
    #[bttr(tag = "SysField::MemFree", class = "SysField::Mem")]
    #[blink("SystemModel$mem.get_free")]
    pub free: Option<u64>,
    #[bttr(
        tag = "SysField::MemAvailable",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_available")]
    pub available: Option<u64>,
    #[bttr(
        tag = "SysField::MemBuffers",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_buffers")]
    pub buffers: Option<u64>,
    #[bttr(
        tag = "SysField::MemCached",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_cached")]
    pub cached: Option<u64>,
    #[bttr(
        tag = "SysField::MemSwapCached",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_swap_cached")]
    pub swap_cached: Option<u64>,
    #[bttr(
        tag = "SysField::MemActive",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_active")]
    pub active: Option<u64>,
    #[bttr(
        tag = "SysField::MemInactive",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_inactive")]
    pub inactive: Option<u64>,
    #[bttr(
        tag = "SysField::MemAnon",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_anon")]
    pub anon: Option<u64>,
    #[bttr(
        tag = "SysField::MemFile",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_file")]
    pub file: Option<u64>,
    #[bttr(
        tag = "SysField::MemUnevictable",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_unevictable")]
    pub unevictable: Option<u64>,
    #[bttr(
        tag = "SysField::MemMlocked",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_mlocked")]
    pub mlocked: Option<u64>,
    #[bttr(
        tag = "SysField::MemSwapTotal",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_swap_total")]
    pub swap_total: Option<u64>,
    #[bttr(
        tag = "SysField::MemSwapFree",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_swap_free")]
    pub swap_free: Option<u64>,
    #[bttr(
        tag = "SysField::MemDirty",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_dirty")]
    pub dirty: Option<u64>,
    #[bttr(
        tag = "SysField::MemWriteback",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_writeback")]
    pub writeback: Option<u64>,
    #[bttr(
        tag = "SysField::MemAnonPages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_anon_pages")]
    pub anon_pages: Option<u64>,
    #[bttr(
        tag = "SysField::MemMapped",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_mapped")]
    pub mapped: Option<u64>,
    #[bttr(
        tag = "SysField::MemShmem",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_shmem")]
    pub shmem: Option<u64>,
    #[bttr(
        tag = "SysField::MemKreclaimable",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_kreclaimable")]
    pub kreclaimable: Option<u64>,
    #[bttr(
        tag = "SysField::MemSlab",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_slab")]
    pub slab: Option<u64>,
    #[bttr(
        tag = "SysField::MemSlabReclaimable",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_slab_reclaimable")]
    pub slab_reclaimable: Option<u64>,
    #[bttr(
        tag = "SysField::MemSlabUnreclaimable",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_slab_unreclaimable")]
    pub slab_unreclaimable: Option<u64>,
    #[bttr(
        tag = "SysField::MemKernelStack",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_kernel_stack")]
    pub kernel_stack: Option<u64>,
    #[bttr(
        tag = "SysField::MemPageTables",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_page_tables")]
    pub page_tables: Option<u64>,
    #[bttr(
        tag = "SysField::MemAnonHugePages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_anon_huge_pages_bytes")]
    pub anon_huge_pages_bytes: Option<u64>,
    #[bttr(
        tag = "SysField::MemShmemHugePages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_shmem_huge_pages_bytes")]
    pub shmem_huge_pages_bytes: Option<u64>,
    #[bttr(
        tag = "SysField::MemFileHugePages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_file_huge_pages_bytes")]
    pub file_huge_pages_bytes: Option<u64>,
    #[bttr(
        tag = "SysField::MemTotalHugePages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_total_huge_pages_bytes")]
    pub total_huge_pages_bytes: Option<u64>,
    #[bttr(
        tag = "SysField::MemFreeHugePages",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_free_huge_pages_bytes")]
    pub free_huge_pages_bytes: Option<u64>,
    #[bttr(
        tag = "SysField::MemHugePageSize",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_huge_page_size")]
    pub huge_page_size: Option<u64>,
    #[bttr(
        tag = "SysField::MemCmaTotal",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_cma_total")]
    pub cma_total: Option<u64>,
    #[bttr(
        tag = "SysField::MemCmaFree",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_cma_free")]
    pub cma_free: Option<u64>,
    #[bttr(
        tag = "SysField::MemVmallocTotal",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_vmalloc_total")]
    pub vmalloc_total: Option<u64>,
    #[bttr(
        tag = "SysField::MemVmallocUsed",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_vmalloc_used")]
    pub vmalloc_used: Option<u64>,
    #[bttr(
        tag = "SysField::MemVmallocChunk",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_vmalloc_chunk")]
    pub vmalloc_chunk: Option<u64>,
    #[bttr(
        tag = "SysField::MemDirectMap4k",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_direct_map_4k")]
    pub direct_map_4k: Option<u64>,
    #[bttr(
        tag = "SysField::MemDirectMap2m",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_direct_map_2m")]
    pub direct_map_2m: Option<u64>,
    #[bttr(
        tag = "SysField::MemDirectMap1g",
        class = "SysField::Mem",
        class_detail = true
    )]
    #[blink("SystemModel$mem.get_direct_map_1g")]
    pub direct_map_1g: Option<u64>,
    // vm stats
    #[bttr(tag = "SysField::VmPgpgin", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgpgin_per_sec")]
    pub pgpgin_per_sec: Option<f64>,
    #[bttr(tag = "SysField::VmPgpgout", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgpgout_per_sec")]
    pub pgpgout_per_sec: Option<f64>,
    #[bttr(tag = "SysField::VmPswpin", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pswpin_per_sec")]
    pub pswpin_per_sec: Option<f64>,
    #[bttr(tag = "SysField::VmPswpout", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pswpout_per_sec")]
    pub pswpout_per_sec: Option<f64>,
    #[bttr(tag = "SysField::VmPstealKswapd", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgsteal_kswapd")]
    pub pgsteal_kswapd: Option<u64>,
    #[bttr(tag = "SysField::VmPstealDirect", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgsteal_direct")]
    pub pgsteal_direct: Option<u64>,
    #[bttr(tag = "SysField::VmPscanKswapd", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgscan_kswapd")]
    pub pgscan_kswapd: Option<u64>,
    #[bttr(tag = "SysField::VmPscanDirect", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_pgscan_direct")]
    pub pgscan_direct: Option<u64>,
    #[bttr(tag = "SysField::VmOomKill", class = "SysField::Vm")]
    #[blink("SystemModel$vm.get_oom_kill")]
    pub oom_kill: Option<u64>,
    // time and aggr
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "SysField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "SysField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&SystemData, &SystemModel) -> String>;
type FieldFtype = Box<dyn Fn(&SystemData, &SystemModel) -> String>;

pub struct System {
    data: SystemData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for System {
    type Model = SystemModel;
    type FieldsType = SysField;
    type DataType = SystemData;
}

make_dget!(
    System,
    SysField::Hostname,
    SysField::Datetime,
    SysField::Cpu,
    SysField::Mem,
    SysField::Vm,
    SysField::Stat,
    SysField::Timestamp,
);

impl Dprint for System {}

impl Dump for System {
    fn new(opts: GeneralOpt, advance: Advance, time_end: SystemTime, _: Option<SysField>) -> Self {
        Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
            title_fns: vec![],
            field_fns: vec![],
        }
    }

    fn advance_timestamp(&mut self, model: &model::Model) -> Result<()> {
        self.data.timestamp = match model.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(t) => t.as_secs() as i64,
            Err(e) => bail!("Fail to convert system time: {}", e),
        };
        self.data.datetime = self.data.timestamp;

        Ok(())
    }

    fn iterate_exec<T: Write>(
        &self,
        model: &model::Model,
        output: &mut T,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        match self.opts.output_format {
            Some(OutputFormat::Raw) | None => self.do_print_raw(&model.system, output, *round)?,
            Some(OutputFormat::Csv) => self.do_print_csv(&model.system, output, *round)?,
            Some(OutputFormat::KeyVal) => self.do_print_kv(&model.system, output)?,
            Some(OutputFormat::Json) => {
                let par = self.do_print_json(&model.system);
                if comma_flag {
                    write!(output, ",{}", par.to_string())?;
                } else {
                    write!(output, "{}", par.to_string())?;
                }
            }
        };

        *round += 1;

        Ok(IterExecResult::Success)
    }
}
