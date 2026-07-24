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
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use common::util::every_n;
use dynolog_service_clients::DynoLogService;
use dynolog_service_srclients::make_DynoLogService_srclient;
use fbinit::FacebookInit;
use futures::TryFutureExt;
use maplit::hashmap;
use rgpu_service_clients::RgpuService;
use rgpu_service_srclients::make_RgpuService_srclient;
use serde::Deserialize;
use serde::Serialize;
use slog::debug;
use slog::error;
use slog::warn;

#[cfg(test)]
mod test;

const LOCALHOST: &str = "127.0.0.1";
const DYNOLOG_SERVICE_PORT: u16 = 1777;
const RGPU_SERVICE_PORT: u16 = 5829;

const KB_PER_MB: u64 = 1024;
const MB: u64 = 1024 * 1024;

pub type GpuMap = BTreeMap<MajMin, GpuInfo>;

#[serde_with::serde_as]
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpuSample {
    pub timestamp: u64,
    // TODO(T116929506): Remove this when we don't need backward
    //                   compatibility.
    // Serialize MajMin with Display. Deserialize with FromStr or
    // default serde implementation.
    #[serde_as(as = "serde_with::PickFirst<(BTreeMap<serde_with::DisplayFromStr, _>, _)>")]
    pub gpu_map: GpuMap,
}

// TODO(T116929506): Use DeserializeFromStr when we don't need
//                   backward compatibility.
// Deserialize with default serde implementation. All structs that
// need to be deserialized should attempt to deserialize with FromStr
// as well as the the default serde implementation.
#[derive(
    Default,
    Clone,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Deserialize,
    serde_with::SerializeDisplay
)]
pub struct MajMin {
    pub major_id: u64,
    pub minor_id: u64,
}

impl std::fmt::Display for MajMin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.major_id, self.minor_id)
    }
}

impl std::str::FromStr for MajMin {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(colon_idx) = s.find(':') {
            Ok(Self {
                major_id: (s[..colon_idx])
                    .parse::<u64>()
                    .with_context(|| format!("Failed for convert {} to u64", &s[..colon_idx]))?,
                minor_id: (s[colon_idx + 1..]).parse::<u64>().with_context(|| {
                    format!("Failed for convert {} to u64", &s[colon_idx + 1..])
                })?,
            })
        } else {
            Err(anyhow!("No colon in 'maj:min'"))
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Memory {
    #[serde(flatten)]
    pub remapped_rows: RemappedRows,
    #[serde(flatten)]
    pub retired_pages: RetiredPages,
    #[serde(flatten)]
    pub ecc: ECC,
    pub utilization_memory: Option<u64>,
    // The following are in MB but we keep naming consistent with
    // gpumon. Fields will be named correctly with units in model.
    pub memory_free: Option<u64>,  // in MB
    pub memory_total: Option<u64>, // in MB
    pub memory_used: Option<u64>,  // in MB
    pub hbm_mem_bw_gbps: Option<f64>,
    pub hbm_mem_bw_util: Option<f64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mig {
    pub mig_current: Option<String>, // e.g. enabled, disabled
    pub mig_pending: Option<String>, // e.g. enabled, disabled
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pcie {
    pub pcie_link_generation: Option<u64>,
    pub pcie_link_width: Option<u64>,
    pub pcie_replay_counter: Option<u64>,
    // The number of bytes of active PCIe rx (read) data including both header
    // and payload. Note that this is from the perspective of the GPU, so
    // copying data from host to device (HtoD) would be reflected in this
    // metric.
    // i.e. DCGM_FI_PROF_PCIE_RX_BYTES
    pub pcie_rx_bytes: Option<u64>,
    pub pcie_rx_kbps: Option<u64>,
    // The number of bytes of active PCIe tx (transmit) data including both
    // header and payload. Note that this is from the perspective of the GPU,
    // so copying data from device to host (DtoH) would be reflected in this
    // metric.
    // i.e. DCGM_FI_PROF_PCIE_TX_BYTES
    pub pcie_tx_bytes: Option<u64>,
    pub pcie_tx_kbps: Option<u64>,
    // Total PCIe rx + tx throughput. AMD reports total throughput without
    // directional rx/tx split.
    pub pcie_total_mbps: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DeviceMetadata {
    pub name: Option<String>, // e.g. A100-PG509-200
    pub uuid: Option<String>,
    pub driver_version: Option<String>,
    pub vbios_version: Option<String>,
    pub serial: Option<String>,
    pub num_streaming_multiprocessors: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    #[serde(flatten)]
    pub fp: FloatingPoint,
    pub performance_mode: Option<u64>,
    pub persistence_enabled: Option<u64>,
    pub pids: Option<Vec<u32>>,
    pub inforom_corrupted: Option<u64>,
    // Percent of time over the past sample period during which one or more
    // kernels was executing on the GPU. The sample period may be between 1
    // second and 1/6 second depending on the product.
    pub utilization_gpu: Option<u64>,
    // TODO(brianc118): add utilization_memory from memory_util_pct

    // The ratio of cycles an SM has at least 1 warp assigned (computed from the
    // number of cycles and elapsed cycles).
    // i.e. DCGM_FI_PROF_SM_ACTIVE
    //
    // Also known as SM Efficiency: https://fburl.com/wiki/eqp51qec
    //
    // From libasicmon/DCGM - can be collected at time different to update_ts
    pub sm_utilization_pct: Option<u32>,
    // Average number of warps per SM. i.e.
    //   sm_occupancy_pct * (max warps per SM)
    //
    // From libasicmon/DCGM - can be collected at time different to update_ts
    pub sm_occupancy: Option<u32>,
    // The ratio of number of warps resident on an SM. (number of resident as a
    // ratio of the theoretical maximum number of warps per elapsed cycle).
    // i.e. DCGM_FI_PROF_SM_OCCUPANCY
    //
    // From libasicmon/DCGM - can be collected at time different to update_ts
    pub sm_occupancy_pct: Option<u32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RetiredPages {
    pub retired_pages_count_single_bit: Option<u64>,
    pub retired_pages_count_double_bit: Option<u64>,
    // bool
    pub retired_pages_pending_reboot: Option<u64>,
    pub retired_pages_timestamps_single_bit: Option<Vec<u64>>,
    pub retired_pages_timestamps_double_bit: Option<Vec<u64>>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RemappedRows {
    pub remapped_rows_correctable_errors: Option<u64>,
    pub remapped_rows_uncorrectable_errors: Option<u64>,
    // bool
    pub remapped_rows_failures: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Clock {
    pub clock_graphics_application_current: Option<u64>,
    pub clock_graphics_application_default: Option<u64>,
    pub clock_graphics_current: Option<u64>,
    pub clock_graphics_max: Option<u64>,
    pub clock_memory_application_current: Option<u64>,
    pub clock_memory_application_default: Option<u64>,
    pub clock_memory_current: Option<u64>,
    pub clock_memory_max: Option<u64>,
    // TODO(brianc118): This should be a list of reasons not just one from rgpu
    pub clocks_throttle_reasons: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ECC {
    pub ecc_enabled: Option<u64>,
    pub ecc_errors_corrected: Option<u64>,
    pub ecc_errors_corrected_dram: Option<u64>,
    pub ecc_errors_uncorrected: Option<u64>,
    pub ecc_errors_uncorrected_dram: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Encoder {
    pub encoder_capacity: Option<u64>,
    pub encoder_fps: Option<u64>,
    // This is actually in microseconds
    pub encoder_latency_ms: Option<u64>,
    pub encoder_session_count: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FloatingPoint {
    // From libasicmon
    pub fp16_active_pct: Option<u32>,
    pub fp32_active_pct: Option<u32>,
    pub fp64_active_pct: Option<u32>,
    pub tensorcore_active_pct: Option<u32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Nvlink {
    // The number of bytes of active NvLink tx (transmit) data
    // including both header and payload.
    // i.e. DCGM_FI_PROF_NVLINK_TX_BYTES
    pub nvlink_tx_mb: Option<u32>,
    // The number of bytes of active NvLink rx (read) data including
    // both header and payload.
    // i.e. DCGM_FI_PROF_NVLINK_RX_BYTES
    pub nvlink_rx_mb: Option<u32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Power {
    pub power_draw: Option<u64>,
    pub power_management_limit: Option<u64>,
    pub power_management_max: Option<u64>,
    pub power_management_min: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Temperature {
    pub temperature: Option<u64>,
    pub temperature_shutdown: Option<u64>,
    pub temperature_slowdown: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    // Required but not enforced at deserialization time
    //
    // TODO(brianc118): Deprecate this as this is now per request not device in rgpu
    pub update_ts: Option<u64>,
    // Required but not enforced at deserialization time
    pub major_id: Option<u64>,
    // Required but not enforced at deserialization time
    pub minor_id: Option<u64>,
    // Not to be used except in the case of error
    pub index: Option<u64>,
    pub vendor: Option<GpuVendor>,
    // None if there is no error
    pub error: Option<String>,
    #[serde(flatten)]
    pub device_metadata: DeviceMetadata,
    #[serde(flatten)]
    pub clock: Clock,
    #[serde(flatten)]
    pub memory: Memory,
    #[serde(flatten)]
    pub encoder: Encoder,
    #[serde(flatten)]
    pub device_info: DeviceInfo,
    #[serde(flatten)]
    pub power: Power,
    #[serde(flatten)]
    pub temperature: Temperature,
    #[serde(flatten)]
    pub pcie: Pcie,
    #[serde(flatten)]
    pub nvlink: Nvlink,
    #[serde(flatten)]
    pub mig: Mig,
}

fn u64_from_i64(v: i64) -> Option<u64> {
    v.try_into().ok()
}

fn u64_from_i32(v: i32) -> Option<u64> {
    v.try_into().ok()
}

fn mb_from_bytes(v: i64) -> Option<u64> {
    u64_from_i64(v).map(|v| v / MB)
}

fn mb_from_kb(v: u64) -> u64 {
    v / KB_PER_MB
}

fn error_string<T: Serialize + std::fmt::Debug>(errors: &T) -> String {
    serde_json::to_string(errors).unwrap_or_else(|_| format!("{errors:?}"))
}

fn nvidia_pcie_total_mbps(pcie_throughput: Option<&nvidia::PcieThroughput>) -> Option<u64> {
    let pcie = pcie_throughput?;
    let rx_kb = pcie.rx_kbytes.and_then(u64_from_i64)?;
    let tx_kb = pcie.tx_kbytes.and_then(u64_from_i64)?;
    rx_kb.checked_add(tx_kb).map(mb_from_kb)
}

fn amd_power_draw_watts(power_info: &amd::PowerInfo) -> Option<u64> {
    if power_info.socket_power_watts > 0 {
        u64_from_i64(power_info.socket_power_watts)
    } else if power_info.current_socket_power > 0 {
        u64_from_i32(power_info.current_socket_power)
    } else {
        None
    }
}

fn amd_device_name(gpu_info_amd: &amd::GPUInfoAmd) -> Option<String> {
    gpu_info_amd
        .name
        .clone()
        .or_else(|| {
            gpu_info_amd
                .asic_info
                .as_ref()
                .and_then(|asic_info| asic_info.market_name.clone())
        })
        .or_else(|| {
            gpu_info_amd
                .board_info
                .as_ref()
                .and_then(|board_info| board_info.product_name.clone())
        })
}

fn amd_serial(gpu_info_amd: &amd::GPUInfoAmd) -> Option<String> {
    gpu_info_amd
        .serial_number
        .clone()
        .or_else(|| {
            gpu_info_amd
                .board_info
                .as_ref()
                .and_then(|board_info| board_info.product_serial.clone())
        })
        .or_else(|| {
            gpu_info_amd
                .asic_info
                .as_ref()
                .and_then(|asic_info| asic_info.asic_serial.clone())
        })
}

fn amd_errors(gpu_info_amd: &amd::GPUInfoAmd) -> Option<String> {
    if gpu_info_amd.errors.is_empty() {
        None
    } else {
        Some(error_string(&gpu_info_amd.errors))
    }
}

fn amd_memory_free_mb(memory_info: &amd::MemoryInfo) -> Option<u64> {
    let total = memory_info
        .total
        .or(memory_info.vram_size)
        .and_then(u64_from_i64)?;
    let used = memory_info.used.and_then(u64_from_i64)?;
    total.checked_sub(used).map(|free| free / MB)
}

fn rgpu_gpu_info_amd_to_gpu_info(gpu_info_amd: &amd::GPUInfoAmd) -> GpuInfo {
    let frequency_info = gpu_info_amd.frequency_info.as_ref();
    let memory_info = gpu_info_amd.vram_memory_info.as_ref();
    let power_info = gpu_info_amd.power_info.as_ref();
    let pcie_throughput = gpu_info_amd.pcie_throughput.as_ref();

    GpuInfo {
        update_ts: None,
        major_id: gpu_info_amd.major_id.and_then(u64_from_i64),
        minor_id: gpu_info_amd.minor_id.and_then(u64_from_i64),
        index: gpu_info_amd.index.and_then(u64_from_i64),
        vendor: Some(GpuVendor::Amd),
        error: amd_errors(gpu_info_amd),
        device_metadata: DeviceMetadata {
            name: amd_device_name(gpu_info_amd),
            uuid: None,
            driver_version: None,
            vbios_version: gpu_info_amd
                .vbios_info
                .as_ref()
                .and_then(|vbios_info| vbios_info.version.clone()),
            serial: amd_serial(gpu_info_amd),
            num_streaming_multiprocessors: None,
        },
        clock: Clock {
            clock_graphics_application_current: None,
            clock_graphics_application_default: None,
            clock_graphics_current: frequency_info
                .and_then(|frequency_info| frequency_info.gpu_frequency_mhz)
                .and_then(u64_from_i64),
            clock_graphics_max: frequency_info
                .and_then(|frequency_info| {
                    frequency_info.system_clock_frequency_limits_mhz.as_ref()
                })
                .and_then(|range| u64_from_i64(range.upper_bound)),
            clock_memory_application_current: None,
            clock_memory_application_default: None,
            clock_memory_current: frequency_info
                .and_then(|frequency_info| frequency_info.memory_clock_mhz)
                .and_then(u64_from_i64),
            clock_memory_max: frequency_info
                .and_then(|frequency_info| {
                    frequency_info.memory_clock_frequency_limits_mhz.as_ref()
                })
                .and_then(|range| u64_from_i64(range.upper_bound)),
            clocks_throttle_reasons: gpu_info_amd
                .metrics_info
                .as_ref()
                .and_then(|metrics_info| metrics_info.throttle_status)
                .and_then(u64_from_i32),
        },
        memory: Memory {
            remapped_rows: Default::default(),
            retired_pages: Default::default(),
            ecc: ECC {
                ecc_enabled: None,
                ecc_errors_corrected: gpu_info_amd
                    .ecc_error_count
                    .as_ref()
                    .and_then(|ecc| u64_from_i64(ecc.correctable)),
                ecc_errors_corrected_dram: None,
                ecc_errors_uncorrected: gpu_info_amd
                    .ecc_error_count
                    .as_ref()
                    .and_then(|ecc| u64_from_i64(ecc.uncorrectable)),
                ecc_errors_uncorrected_dram: None,
            },
            utilization_memory: gpu_info_amd.mem_busy_pct.and_then(u64_from_i64),
            memory_free: memory_info.and_then(amd_memory_free_mb),
            memory_total: memory_info
                .and_then(|memory_info| memory_info.total.or(memory_info.vram_size))
                .and_then(mb_from_bytes),
            memory_used: memory_info
                .and_then(|memory_info| memory_info.used)
                .and_then(mb_from_bytes),
            hbm_mem_bw_gbps: None,
            hbm_mem_bw_util: None,
        },
        encoder: Default::default(),
        device_info: DeviceInfo {
            fp: Default::default(),
            performance_mode: None,
            persistence_enabled: None,
            pids: None,
            inforom_corrupted: None,
            utilization_gpu: gpu_info_amd.gpu_busy_pct.and_then(u64_from_i64),
            sm_utilization_pct: None,
            sm_occupancy: None,
            sm_occupancy_pct: None,
        },
        power: Power {
            power_draw: power_info.and_then(amd_power_draw_watts),
            power_management_limit: power_info.and_then(|power_info| {
                power_info
                    .power_cap_info
                    .power_cap_watts
                    .and_then(u64_from_i64)
                    .or_else(|| u64_from_i32(power_info.power_limit_watts))
            }),
            power_management_max: power_info.and_then(|power_info| {
                power_info
                    .power_cap_info
                    .max_power_cap_watts
                    .and_then(u64_from_i64)
            }),
            power_management_min: power_info.and_then(|power_info| {
                power_info
                    .power_cap_info
                    .min_power_cap_watts
                    .and_then(u64_from_i64)
            }),
        },
        temperature: Default::default(),
        pcie: Pcie {
            pcie_link_generation: pcie_throughput
                .and_then(|pcie| pcie.pcie_interface_version)
                .and_then(u64_from_i32),
            pcie_link_width: pcie_throughput
                .and_then(|pcie| pcie.pcie_width.or(pcie.max_pcie_width))
                .and_then(u64_from_i32),
            pcie_replay_counter: pcie_throughput
                .and_then(|pcie| pcie.pcie_replay_count)
                .and_then(u64_from_i64),
            pcie_rx_bytes: None,
            pcie_rx_kbps: pcie_throughput
                .and_then(|pcie| pcie.rx_kbytes)
                .and_then(u64_from_i64),
            pcie_tx_bytes: None,
            pcie_tx_kbps: pcie_throughput
                .and_then(|pcie| pcie.tx_kbytes)
                .and_then(u64_from_i64),
            pcie_total_mbps: pcie_throughput
                .and_then(|pcie| pcie.bandwidth_mbytes)
                .and_then(u64_from_i64),
        },
        nvlink: Default::default(),
        mig: Default::default(),
    }
}

// Asic stats from libasicmon. May contain duplicate information with
// gpumon.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AsicBaseStats {
    pub update_ts: Option<u64>, // Should be present for GPU device
    pub current_temp_c: Option<i32>,
    pub max_temp_c: Option<i32>,
    pub throttle_temp_lower_c: Option<i32>,
    pub throttle_temp_upper_c: Option<i32>,
    pub correctable_dram_err_cnt: Option<u64>,
    pub uncorrectable_dram_err_cnt: Option<u64>,
    pub correctable_internal_mem_err_cnt: Option<u64>,
    pub uncorrectable_internal_mem_err_cnt: Option<u64>,
    pub pcie_ce_total: Option<u64>,
    pub pcie_uce_nonfatal_total: Option<u64>,
    pub pcie_uce_fatal_total: Option<u64>,
    // For GPU, this is SM occupancy as avg(warps / max warps / SM).
    // i.e. DCGM_FI_PROF_SM_OCCUPANCY * 100
    pub device_utilization_pct: Option<u32>,
    pub mega_ops_per_second: Option<u32>,
    pub pcie_bw_util_pct: Option<u32>,
    pub mem_bw_util_pct: Option<u64>,
    pub mem_capacity_util_pct: Option<u32>,
    pub max_core_frequency_mhz: Option<u32>,
    pub core_frequency_mhz: Option<u32>,
    pub max_mem_frequency_mhz: Option<u32>,
    pub mem_frequency_mhz: Option<u32>,
    pub noc_frequency_mhz: Option<u32>,
    pub max_noc_frequency_mhz: Option<u32>,
    pub cur_tot_power_mw: Option<u32>, // Should be present for GPU device
    pub max_tot_power_mw: Option<u32>,
    pub voltage_rail_1p8_mv: Option<u32>,
    pub voltage_rail_3p3_mv: Option<u32>,
    pub power_state: Option<u32>,
    pub core_voltage_mv: Option<u32>,
    pub total_power_mw: Option<u32>,
    pub uptime: Option<u64>,
    pub error_info_list: Option<Vec<String>>,
    pub is_throttling: Option<u32>,
    pub dram_read_mb: Option<u32>,
    pub dram_write_mb: Option<u32>,
    pub mem_size_mb: Option<u32>,
    pub read_bw_mbps: Option<u32>,
    pub write_bw_mbps: Option<u32>,
    pub ready_status: Option<u32>,
    pub mce_fatal: Option<u32>,
    pub mce_nonfatal: Option<u32>,
}

// Generic struct for Nvidia GPU. Response is guaranteed to return a
// subset of the fields below.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpuAsicBaseStats {
    pub major_id: Option<u64>,
    pub minor_id: Option<u64>,
    #[serde(flatten)]
    pub asic_base_stats: AsicBaseStats,
    // Additional stats that are part of base stats for Nvidia GPU
    // See https://fburl.com/code/p5mlf2m0
    pub mem_free_mb: Option<u32>,
    pub mem_used_mb: Option<u32>,
    // The ratio of cycles an SM has at least 1 warp assigned (computed from the
    // number of cycles and elapsed cycles)
    // i.e. DCGM_FI_PROF_SM_ACTIVE * 100
    //
    // Also known as SM Efficiency: https://fburl.com/wiki/eqp51qec
    pub sm_utilization_pct: Option<u32>,
    // Average number of warps per SM.
    //
    // From libasicmon/DCGM - can be collected at time different to update_ts
    pub sm_occupancy: Option<u32>,
    // Ratio of cycles the fp16 pipe is active. This does not include
    // HMMA.
    // i.e. DCGM_FI_PROF_PIPE_FP16_ACTIVE
    pub fp16_active_pct: Option<u32>,
    // Ratio of cycles the fp32 pipe is active.
    // i.e. DCGM_FI_PROF_PIPE_FP32_ACTIVE
    pub fp32_active_pct: Option<u32>,
    // Ratio of cycles the fp64 pipe is active.
    // i.e. DCGM_FI_PROF_PIPE_FP64_ACTIVE
    pub fp64_active_pct: Option<u32>,
    // The ratio of cycles the tensor (HMMA) pipe is active (off the
    // peak sustained elapsed cycles)
    // i.e. DCGM_FI_PROF_PIPE_TENSOR_ACTIVE
    pub tensorcore_active_pct: Option<u32>,
    // The number of bytes of active PCIe tx (transmit) data including both
    // header and payload. Note that this is from the perspective of the GPU,
    // so copying data from device to host (DtoH) would be reflected in this
    // metric.
    // i.e. DCGM_FI_PROF_PCIE_TX_BYTES
    pub pcie_tx_mb: Option<u32>,
    // The number of bytes of active PCIe rx (read) data including both header
    // and payload. Note that this is from the perspective of the GPU, so
    // copying data from host to device (HtoD) would be reflected in this
    // metric.
    // i.e. DCGM_FI_PROF_PCIE_RX_BYTES
    pub pcie_rx_mb: Option<u32>,
    // Total PCIe bandwidth in MB/s. AMD reports total bandwidth without a
    // directional rx/tx split.
    pub pcie_total_bw_mbps: Option<u32>,
    // The number of bytes of active NvLink tx (transmit) data
    // including both header and payload.
    // i.e. DCGM_FI_PROF_NVLINK_TX_BYTES
    pub nvlink_tx_mb: Option<u32>,
    // The number of bytes of active NvLink rx (read) data including
    // both header and payload.
    // i.e. DCGM_FI_PROF_NVLINK_RX_BYTES
    pub nvlink_rx_mb: Option<u32>,
    // GPU device utilization. i.e. same as utilization_gpu
    pub gpu_active_time_pct: Option<u32>,
    // DCGM error code if there's an error
    pub dcgm_error: Option<u32>,
}

fn rgpu_gpu_info_nvidia_to_gpu_info(gpu_info_nvidia: &nvidia::GPUInfoNvidia) -> GpuInfo {
    let pcie_throughput = gpu_info_nvidia.pcie_throughput.as_ref();

    GpuInfo {
        // TODO(brianc118): this is per request - need to move it elsewhere
        update_ts: None,
        major_id: gpu_info_nvidia.major_id.map(|v| v as u64),
        minor_id: gpu_info_nvidia.minor_id.map(|v| v as u64),
        index: Some(gpu_info_nvidia.index as u64),
        vendor: Some(GpuVendor::Nvidia),
        error: if gpu_info_nvidia.nvml_api_failures.is_empty() {
            None
        } else {
            Some(error_string(&gpu_info_nvidia.nvml_api_failures))
        },
        device_metadata: DeviceMetadata {
            name: gpu_info_nvidia.model.clone(),
            uuid: gpu_info_nvidia.uuid.clone(),
            // From GPUInfo - we will populate this after
            driver_version: None,
            vbios_version: gpu_info_nvidia.vbios_version.clone(),
            serial: gpu_info_nvidia.serial_number.clone(),
            // TODO(brianc118): Evaluate if we still need this as it does not
            //                  come from rgpu/NVML
            num_streaming_multiprocessors: None,
        },
        clock: Clock {
            clock_graphics_application_current: gpu_info_nvidia
                .app_clocks
                .as_ref()
                .and_then(|v| v.graphics)
                .map(|v| v as u64),
            clock_graphics_application_default: gpu_info_nvidia
                .default_app_clocks
                .as_ref()
                .and_then(|v| v.graphics)
                .map(|v| v as u64),
            clock_graphics_current: gpu_info_nvidia
                .device_clocks
                .as_ref()
                .and_then(|v| v.graphics)
                .map(|v| v as u64),
            clock_graphics_max: gpu_info_nvidia
                .max_clocks
                .as_ref()
                .and_then(|v| v.graphics)
                .map(|v| v as u64),
            clock_memory_application_current: gpu_info_nvidia
                .app_clocks
                .as_ref()
                .and_then(|v| v.mem)
                .map(|v| v as u64),
            clock_memory_application_default: gpu_info_nvidia
                .default_app_clocks
                .as_ref()
                .and_then(|v| v.mem)
                .map(|v| v as u64),
            clock_memory_current: gpu_info_nvidia
                .device_clocks
                .as_ref()
                .and_then(|v| v.mem)
                .map(|v| v as u64),
            clock_memory_max: gpu_info_nvidia
                .max_clocks
                .as_ref()
                .and_then(|v| v.mem)
                .map(|v| v as u64),
            // TODO(brianc118): This should be a list of reasons not just one from rgpu
            clocks_throttle_reasons: None,
        },
        memory: Memory {
            remapped_rows: RemappedRows {
                remapped_rows_correctable_errors: gpu_info_nvidia
                    .remapped_rows
                    .as_ref()
                    .and_then(|v| v.corr_rows)
                    .map(|v| v as u64),
                remapped_rows_uncorrectable_errors: gpu_info_nvidia
                    .remapped_rows
                    .as_ref()
                    .and_then(|v| v.unc_rows)
                    .map(|v| v as u64),
                remapped_rows_failures: gpu_info_nvidia
                    .remapped_rows
                    .as_ref()
                    .and_then(|v| v.failure_occurred)
                    .map(|v| v as u64),
            },
            retired_pages: RetiredPages {
                retired_pages_count_single_bit: gpu_info_nvidia
                    .retired_pages
                    .as_ref()
                    .and_then(|v| v.multiple_single_bit_retired_page_count)
                    .map(|v| v as u64),
                retired_pages_count_double_bit: gpu_info_nvidia
                    .retired_pages
                    .as_ref()
                    .and_then(|v| v.double_bit_retired_page_count)
                    .map(|v| v as u64),
                retired_pages_pending_reboot: gpu_info_nvidia
                    .retired_pages
                    .as_ref()
                    .and_then(|v| v.is_pending)
                    .map(|v| v as u64),
                retired_pages_timestamps_single_bit: gpu_info_nvidia
                    .retired_pages
                    .as_ref()
                    .and_then(|v| v.multiple_single_bit_retired_pages.as_ref())
                    .map(|l| l.iter().map(|v| v.timestamp as u64).collect()),
                retired_pages_timestamps_double_bit: gpu_info_nvidia
                    .retired_pages
                    .as_ref()
                    .and_then(|v| v.double_bit_retired_pages.as_ref())
                    .map(|l| l.iter().map(|v| v.timestamp as u64).collect()),
            },
            ecc: ECC {
                ecc_enabled: gpu_info_nvidia.ecc_enabled_current.map(|v| v as u64),
                ecc_errors_corrected: gpu_info_nvidia
                    .ecc_info
                    .as_ref()
                    .and_then(|v| v.total_ecc_errors_corrected_aggregate)
                    .map(|v| v as u64),
                ecc_errors_corrected_dram: gpu_info_nvidia
                    .ecc_info
                    .as_ref()
                    .and_then(|v| v.dram_ecc_errors_corrected_aggregate)
                    .map(|v| v as u64),
                ecc_errors_uncorrected: gpu_info_nvidia
                    .ecc_info
                    .as_ref()
                    .and_then(|v| v.total_ecc_errors_uncorrected_aggregate)
                    .map(|v| v as u64),
                ecc_errors_uncorrected_dram: gpu_info_nvidia
                    .ecc_info
                    .as_ref()
                    .and_then(|v| v.dram_ecc_errors_uncorrected_aggregate)
                    .map(|v| v as u64),
            },
            utilization_memory: gpu_info_nvidia.memory_util_pct.map(|v| v as u64),
            memory_free: gpu_info_nvidia
                .memory
                .as_ref()
                .and_then(|m| m.free)
                .map(|v| v as u64),
            memory_total: gpu_info_nvidia
                .memory
                .as_ref()
                .and_then(|m| m.total)
                .map(|v| v as u64),
            memory_used: gpu_info_nvidia
                .memory
                .as_ref()
                .and_then(|m| m.used)
                .map(|v| v as u64),
            hbm_mem_bw_gbps: None,
            hbm_mem_bw_util: None,
        },
        encoder: Encoder {
            encoder_capacity: gpu_info_nvidia
                .encoder_info
                .as_ref()
                .and_then(|i| i.h264_encoder_capacity)
                .map(|v| v as u64),
            encoder_fps: gpu_info_nvidia
                .encoder_info
                .as_ref()
                .and_then(|i| i.average_fps)
                .map(|v| v as u64),
            encoder_latency_ms: gpu_info_nvidia
                .encoder_info
                .as_ref()
                .and_then(|i| i.average_latency)
                .map(|v| v as u64),
            encoder_session_count: gpu_info_nvidia
                .encoder_info
                .as_ref()
                .and_then(|i| i.session_count)
                .map(|v| v as u64),
        },
        device_info: DeviceInfo {
            fp: FloatingPoint {
                // From libasicmon
                fp16_active_pct: None,
                fp32_active_pct: None,
                fp64_active_pct: None,
                tensorcore_active_pct: None,
            },
            // TODO(brianc118): Consider deprecating if not needed
            performance_mode: None,
            persistence_enabled: gpu_info_nvidia.persistence.map(|v| v as u64),
            pids: gpu_info_nvidia
                .compute_running_processes
                .as_ref()
                .map(|v| v.iter().filter_map(|p| p.pid).map(|v| v as u32).collect()),
            inforom_corrupted: gpu_info_nvidia.inforom_corrupted.map(|v| v as u64),
            utilization_gpu: gpu_info_nvidia.gpu_util_pct.map(|v| v as u64),
            // These are from dynolog DCGM
            sm_utilization_pct: None,
            sm_occupancy: None,
            sm_occupancy_pct: None,
        },
        power: Power {
            power_draw: gpu_info_nvidia.power_usage.map(|v| v as u64),
            power_management_limit: gpu_info_nvidia.power_management_limit.map(|v| v as u64),
            power_management_max: gpu_info_nvidia.power_management_max_limit.map(|v| v as u64),
            power_management_min: gpu_info_nvidia.power_management_min_limit.map(|v| v as u64),
        },
        temperature: Temperature {
            temperature: gpu_info_nvidia.temperature.map(|v| v as u64),
            temperature_shutdown: gpu_info_nvidia
                .temperature_threshold_shutdown
                .map(|v| v as u64),
            temperature_slowdown: gpu_info_nvidia
                .temperature_threshold_slowdown
                .map(|v| v as u64),
        },
        pcie: Pcie {
            pcie_link_generation: gpu_info_nvidia.pcie_link_generation.map(|v| v as u64),
            pcie_link_width: gpu_info_nvidia.pcie_link_width.map(|v| v as u64),
            pcie_replay_counter: gpu_info_nvidia.pcie_replay_counter.map(|v| v as u64),
            // From DCGM
            pcie_rx_bytes: None,
            // TODO(brianc118): Review whether this is really kbps
            pcie_rx_kbps: pcie_throughput.and_then(|v| v.rx_kbytes).map(|v| v as u64),
            // From DCGM
            pcie_tx_bytes: None,
            pcie_tx_kbps: pcie_throughput.and_then(|v| v.tx_kbytes).map(|v| v as u64),
            pcie_total_mbps: nvidia_pcie_total_mbps(pcie_throughput),
        },
        nvlink: Nvlink {
            // From DCGM
            nvlink_tx_mb: None,
            // From DCGM
            nvlink_rx_mb: None,
        },
        mig: Mig {
            mig_current: gpu_info_nvidia.mig_enabled_current.map(|v| {
                if v {
                    "enabled".to_owned()
                } else {
                    "disable".to_owned()
                }
            }),
            mig_pending: gpu_info_nvidia.mig_enabled_pending.map(|v| {
                if v {
                    "enabled".to_owned()
                } else {
                    "disable".to_owned()
                }
            }),
        },
    }
}

fn rgpu_gpu_info_response_to_gpu_map(
    logger: &slog::Logger,
    gpu_info_response: rgpu_service::GetGPUInfoResponse,
) -> Result<GpuMap> {
    let mut ret = BTreeMap::new();
    for device_info in gpu_info_response.gpu_info.gpu_details {
        let mut gpu_info = match device_info.detail {
            gpu_info::GPUDetail::nvidia_info(nvidia_info) => {
                rgpu_gpu_info_nvidia_to_gpu_info(&nvidia_info)
            }
            gpu_info::GPUDetail::amd_info(amd_info) => rgpu_gpu_info_amd_to_gpu_info(&amd_info),
            gpu_info::GPUDetail::intel_info(_) => {
                // TODO(brianc118): Support Intel GPU
                error!(logger, "Got Intel GPU Info for GPUDetail");
                GpuInfo {
                    vendor: Some(GpuVendor::Intel),
                    ..Default::default()
                }
            }
            gpu_info::GPUDetail::UnknownField(_) => {
                bail!("Got UnknownField for GPUDetail")
            }
        };
        gpu_info
            .device_metadata
            .driver_version
            .clone_from(&gpu_info_response.gpu_info.driver_version);
        gpu_info.update_ts = gpu_info_response.timestamp.map(|t| t as u64);

        match (gpu_info.major_id, gpu_info.minor_id) {
            (Some(major_id), Some(minor_id)) => {
                let maj_min = MajMin { major_id, minor_id };
                // TODO: Use try_insert when stabilized
                if ret.contains_key(&maj_min) {
                    bail!("rgpu: Duplicate maj min numbers for device: {:?}", maj_min);
                }
                ret.insert(maj_min, gpu_info);
            }
            _ => {
                bail!(
                    "rgpu: Missing maj min for device (index: {})",
                    gpu_info.index.map_or("?".to_owned(), |v| v.to_string())
                );
            }
        }
    }
    Ok(ret)
}

fn gpu_map_to_gpu_sample(logger: &slog::Logger, gpu_map: GpuMap) -> Result<Option<GpuSample>> {
    if gpu_map.is_empty() {
        return Ok(None);
    }
    let mut oldest_ts = u64::MAX;
    let mut latest_ts = u64::MIN;
    for dev in gpu_map.values() {
        if let Some(ts) = dev.update_ts {
            oldest_ts = std::cmp::min(oldest_ts, ts);
            latest_ts = std::cmp::max(latest_ts, ts);
        } else {
            bail!(
                "Device sample missing update_ts. {:?}:{:?}",
                dev.major_id,
                dev.minor_id
            );
        }
    }
    const GPU_COLLECTION_SKEW_THRESHOLD: u64 = 2;
    let skew = latest_ts - oldest_ts;
    if skew > GPU_COLLECTION_SKEW_THRESHOLD {
        warn!(
            logger,
            "Gpu collection threshold skew {} > {}", skew, GPU_COLLECTION_SKEW_THRESHOLD
        );
    }
    Ok(Some(GpuSample {
        timestamp: oldest_ts,
        gpu_map,
    }))
}

// Convert raw response from dyno DCGM data to a map of GpuAsicBaseStats. A
// return value of `None` means there was a DCGM (recoverable) error.
fn raw_map_to_gpu_base_stats(
    logger: &slog::Logger,
    responses: Vec<BTreeMap<String, i64>>,
) -> Result<Option<BTreeMap<MajMin, GpuAsicBaseStats>>> {
    let mut ret = BTreeMap::new();

    for response in responses {
        match response.get("dcgm_error") {
            Some(error) if *error != 0 => {
                // Don't treat DCGM error as hard failure that results in entire sample being
                // dropped. Dynolog team says some failures on occasion are expected. See:
                // https://fburl.com/workplace/es49hgvw
                warn!(
                    logger,
                    "getBaseStatsForDevice: DCGM error: {:?}. Not treated as hard failure",
                    response
                );
                return Ok(None);
            }
            _ => {}
        }
        let v: serde_json::Value = serde_json::json!(response);
        let stats: GpuAsicBaseStats = serde_json::from_value(v).with_context(|| {
            format!(
                "Failed to convert response for to GpuAsicBaseStats: {:?}",
                response
            )
        })?;
        if let (Some(major_id), Some(minor_id)) = (stats.major_id, stats.minor_id) {
            let maj_min = MajMin { major_id, minor_id };
            // TODO: Use try_insert when stabilized
            if ret.contains_key(&maj_min) {
                bail!(
                    "getBaseStatsForDevice: Duplicate maj min numbers for device: {:?}",
                    maj_min
                );
            }
            if let Some(error_list) = &stats.asic_base_stats.error_info_list
                && !error_list.is_empty()
            {
                // If there's an error we can expect it to be very spammy
                every_n!(
                    20,
                    error!(
                        logger,
                        "getBaseStatsForDevice: Device ({:?}:{:?}) had errors {:?}",
                        stats.major_id,
                        stats.minor_id,
                        error_list
                    )
                );
                continue;
            }
            ret.insert(maj_min, stats);
        } else {
            bail!("getBaseStatsForDevice: Missing maj min for device")
        }
    }
    Ok(Some(ret))
}

fn add_gpu_base_stats_to_gpu_map(
    gpu_map: &mut GpuMap,
    asicmon_base_stats: BTreeMap<MajMin, GpuAsicBaseStats>,
) -> Result<()> {
    // Set of keys in gpumap should be a subset of those managed by
    // libasicmon.
    for (maj_min, gpu_info) in gpu_map.iter_mut() {
        let base_stats = asicmon_base_stats
            .get(maj_min)
            .ok_or_else(|| anyhow!("Could not find device {:?} in base stats", maj_min))?;
        let is_amd = gpu_info.vendor == Some(GpuVendor::Amd);

        // Not available from gpumon so take from libasicmon. AMD computes this
        // from CU active cycles over elapsed cycles, analogous to NVIDIA's SM
        // active metric.
        gpu_info.device_info.sm_utilization_pct = base_stats.sm_utilization_pct;
        if is_amd {
            // AMD reports occupancy as mean CU occupancy percentage. Below uses
            // SM field names, but this is the same kind of accelerator-block
            // occupancy metric as NVIDIA SM occupancy percentage.
            gpu_info.device_info.sm_occupancy_pct = base_stats.sm_occupancy;
            if gpu_info.device_info.utilization_gpu.is_none() {
                gpu_info.device_info.utilization_gpu = base_stats
                    .asic_base_stats
                    .device_utilization_pct
                    .map(|v| v as u64);
            }
        } else {
            gpu_info.device_info.sm_occupancy = base_stats.sm_occupancy;
            gpu_info.device_info.sm_occupancy_pct =
                base_stats.asic_base_stats.device_utilization_pct;
        }
        gpu_info.device_info.fp.fp16_active_pct = base_stats.fp16_active_pct;
        gpu_info.device_info.fp.fp32_active_pct = base_stats.fp32_active_pct;
        gpu_info.device_info.fp.fp64_active_pct = base_stats.fp64_active_pct;
        gpu_info.device_info.fp.tensorcore_active_pct = base_stats.tensorcore_active_pct;
        gpu_info.nvlink.nvlink_tx_mb = base_stats.nvlink_tx_mb;
        gpu_info.nvlink.nvlink_rx_mb = base_stats.nvlink_rx_mb;

        // Available from gpumon, but available from libasicmon at
        // higher frequency
        if let Some(mem_free_mb) = base_stats.mem_free_mb {
            gpu_info.memory.memory_free = Some(mem_free_mb as u64);
        }
        if let Some(mem_used_mb) = base_stats.mem_used_mb {
            gpu_info.memory.memory_used = Some(mem_used_mb as u64);
        }
        if let Some(mem_size_mb) = base_stats.asic_base_stats.mem_size_mb {
            gpu_info.memory.memory_total = Some(mem_size_mb as u64);
        }
        gpu_info.pcie.pcie_tx_bytes = base_stats.pcie_tx_mb.map(|v| v as u64 * MB);
        gpu_info.pcie.pcie_rx_bytes = base_stats.pcie_rx_mb.map(|v| v as u64 * MB);
        if let Some(pcie_total_bw_mbps) = base_stats.pcie_total_bw_mbps {
            gpu_info.pcie.pcie_total_mbps = Some(pcie_total_bw_mbps as u64);
        }

        if let Some(cur_tot_power_mw) = base_stats.asic_base_stats.cur_tot_power_mw {
            gpu_info.power.power_draw = Some((cur_tot_power_mw / 1000) as u64);
        }

        if gpu_info.temperature.temperature.is_none() {
            gpu_info.temperature.temperature = base_stats
                .asic_base_stats
                .current_temp_c
                .and_then(u64_from_i32);
        }
    }
    Ok(())
}

macro_rules! handle_fbthrift_error {
    ( $result:expr, $logger:expr, $error_type:ty ) => {{
        type T = $error_type;
        match $result {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                if let T::ThriftError(e) = &e {
                    if let Some(sre) =
                        e.downcast_ref::<dynolog_service_srclients::srclient::TServiceRouterException>()
                    {
                        // Request is safe to try if it never made it to
                        // the server. See https://fburl.com/code/1fvbyil4
                        if sre.is_retry_safe() {
                            warn!($logger, "{:?}", sre);
                            return Ok(None);
                        }
                    }
                }
                Err(anyhow!(e))
            }
        }
    }};
}

// If error is recoverable, log error message and return None. If
// error is not recoverable return an anyhow error of itself.
fn handle_fbthrift_error<T>(
    result: Result<T, fbthrift::NonthrowingFunctionError>,
    logger: &slog::Logger,
) -> Result<Option<T>, anyhow::Error> {
    handle_fbthrift_error!(result, logger, fbthrift::NonthrowingFunctionError)
}

pub struct GpuStatsCollector {
    logger: slog::Logger,
    dynolog_client: Arc<dyn DynoLogService + Send + Sync>,
    rgpu_client: Arc<dyn RgpuService + Send + Sync>,
}

impl GpuStatsCollector {
    pub fn new(fb: FacebookInit, logger: slog::Logger) -> Result<Self> {
        // We are talking to local dynolog. That said, we should
        // still use ServiceRouter rather than raw thrift client. See
        // https://fburl.com/v48pmqn1
        let dynolog_addr = match (LOCALHOST, DYNOLOG_SERVICE_PORT).to_socket_addrs()?.next() {
            Some(addr) => addr,
            None => bail!("Could not get socket"),
        };
        let dynolog_service_opts = hashmap! {
            "single_host".to_owned() => vec![dynolog_addr.ip().to_string(), dynolog_addr.port().to_string()],
        };
        let rgpu_addr = match (LOCALHOST, RGPU_SERVICE_PORT).to_socket_addrs()?.next() {
            Some(addr) => addr,
            None => bail!("Could not get socket"),
        };
        let rgpu_service_opts = hashmap! {
            "single_host".to_owned() => vec![rgpu_addr.ip().to_string(), rgpu_addr.port().to_string()],
        };
        // Override default config found here:
        // https://www.internalfb.com/intern/wiki/ServiceRouter/Configuration/
        let conn_config = hashmap! {
            "retries_per_reason".to_owned() => "all=0".into(),
        };
        let dynolog_client = make_DynoLogService_srclient!(
            fb,
            tiername = "",
            with_service_options = &dynolog_service_opts,
            with_conn_config = &conn_config,
        )?;
        let rgpu_client = make_RgpuService_srclient!(
            fb,
            tiername = "",
            with_service_options = &rgpu_service_opts,
            with_conn_config = &conn_config
        )?;
        Ok(Self {
            logger,
            dynolog_client,
            rgpu_client,
        })
    }

    pub fn new_with_client(
        logger: slog::Logger,
        dynolog_client: Arc<dyn DynoLogService + Send + Sync>,
        rgpu_client: Arc<dyn RgpuService + Send + Sync>,
    ) -> Self {
        Self {
            logger,
            dynolog_client,
            rgpu_client,
        }
    }

    // Should match `try_collect()` signature of `model::AsyncCollectorPlugin`.
    // Returning `None` means there was a recoverable error.
    pub async fn try_collect(&self) -> Result<Option<GpuSample>> {
        let read_ts = common::util::get_unix_timestamp(SystemTime::now());
        let asicmon_future = self.dynolog_client.listAsicIDs().and_then(|asic_ids| {
            // listAsicIDs() will return something like:
            // [
            //   "3.gi2",
            //   "3.gi1",
            //   "3",
            //   "2.gi2",
            //   "2.gi1",
            //   "2",
            //   ...
            // for MIG machines. We only care about entire devices not MIG
            // instances.
            futures::future::try_join_all(
                asic_ids
                    .iter()
                    .filter(|id| id.parse::<u32>().is_ok())
                    .map(|id| self.dynolog_client.getBaseStatsForDevice(id)),
            )
        });

        let gpu_info_response = match handle_fbthrift_error!(
            self.rgpu_client
                .getGPUInfoCache(&rgpu_service::GetGPUInfoCacheRequest {
                    ..Default::default()
                })
                .await,
            &self.logger,
            rgpu_service_clients::errors::GetGPUInfoCacheError
        )? {
            // return early on recoverable error
            None => return Ok(None),
            Some(r) => r,
        };
        let mut gpu_map = rgpu_gpu_info_response_to_gpu_map(&self.logger, gpu_info_response)?;

        let maybe_asicmon_stats_response =
            handle_fbthrift_error(asicmon_future.await, &self.logger)?;

        if let Some(asicmon_stats_response) = maybe_asicmon_stats_response {
            if let Some(base_stats_map) =
                raw_map_to_gpu_base_stats(&self.logger, asicmon_stats_response)?
            {
                add_gpu_base_stats_to_gpu_map(&mut gpu_map, base_stats_map)?;
            } else {
                return Ok(None); // Recoverable error
            }
        }
        let sample = if let Some(sample) = gpu_map_to_gpu_sample(&self.logger, gpu_map)? {
            debug!(self.logger, "Collected GPU sample: {:?}", sample);
            const STALE_WARNING_THRESHOLD: u64 = 60; // 60s which is the gpumon refresh rate
            if sample.timestamp < read_ts && read_ts - sample.timestamp > STALE_WARNING_THRESHOLD {
                warn!(
                    self.logger,
                    "GPU data stale {} > {}",
                    read_ts - sample.timestamp,
                    STALE_WARNING_THRESHOLD
                );
            }
            sample
        } else {
            every_n!(
                20,
                warn!(self.logger, "dynolog did not list any GPU devices")
            );
            GpuSample {
                timestamp: read_ts,
                gpu_map: Default::default(),
            }
        };
        Ok(Some(sample))
    }
}
