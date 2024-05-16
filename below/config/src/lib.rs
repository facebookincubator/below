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

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::bail;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

#[cfg(test)]
mod test;

pub const BELOW_DEFAULT_CONF: &str = "/etc/below/below.conf";
const BELOW_DEFAULT_LOG: &str = "/var/log/below";
const BELOW_DEFAULT_STORE: &str = "/var/log/below/store";

/// Global below config
pub static BELOW_CONFIG: OnceLock<BelowConfig> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug)]
// If value is missing during deserialization, use the Default::default()
#[serde(default)]
pub struct BelowConfig {
    pub log_dir: PathBuf,
    pub store_dir: PathBuf,
    pub cgroup_root: PathBuf,
    pub cgroup_filter_out: String,
    pub enable_gpu_stats: bool,
    pub use_rgpu_for_gpu_stats: bool,
    pub enable_btrfs_stats: bool,
    pub btrfs_samples: u64,
    pub btrfs_min_pct: f64,
    pub enable_ethtool_stats: bool,
    pub enable_resctrl_stats: bool,
    pub enable_tc_stats: bool,
}

impl Default for BelowConfig {
    fn default() -> Self {
        BelowConfig {
            log_dir: BELOW_DEFAULT_LOG.into(),
            store_dir: BELOW_DEFAULT_STORE.into(),
            cgroup_root: cgroupfs::DEFAULT_CG_ROOT.into(),
            cgroup_filter_out: String::new(),
            enable_gpu_stats: false,
            use_rgpu_for_gpu_stats: true,
            enable_btrfs_stats: false,
            btrfs_samples: btrfs::DEFAULT_SAMPLES,
            btrfs_min_pct: btrfs::DEFAULT_MIN_PCT,
            enable_ethtool_stats: false,
            enable_resctrl_stats: false,
            enable_tc_stats: false,
        }
    }
}

impl BelowConfig {
    pub fn load(path: &Path) -> Result<Self> {
        match path.exists() {
            true if !path.is_file() => bail!("{} exists and is not a file", path.to_string_lossy()),
            true => BelowConfig::load_exists(path),
            false if path.to_string_lossy() == BELOW_DEFAULT_CONF => Ok(Default::default()),
            false => bail!("No such file or directory: {}", path.to_string_lossy()),
        }
    }

    fn load_exists(path: &Path) -> Result<Self> {
        let string_config = match fs::read_to_string(path) {
            Ok(sc) => sc,
            Err(e) => {
                bail!(
                    "Failed to read from config file {}: {}",
                    path.to_string_lossy(),
                    e
                );
            }
        };

        match toml::from_str(string_config.as_str()) {
            Ok(bc) => Ok(bc),
            Err(e) => {
                bail!(
                    "Failed to parse config file {}: {}\n{}",
                    path.to_string_lossy(),
                    e,
                    string_config
                );
            }
        }
    }
}
