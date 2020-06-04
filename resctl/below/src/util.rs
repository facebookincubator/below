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

/// This file contains various helpers
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::model::CgroupModel;

/// Convert `timestamp` from `SystemTime` to seconds since epoch
pub fn get_unix_timestamp(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs()
}

/// Convert `timestamp` from seconds since epoch to `SystemTime`
pub fn get_system_time(timestamp: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(timestamp)
}

/// Convert `val` bytes into a human friendly string
pub fn convert_bytes(val: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if val < 1_f64 {
        return format!("{:.1} B", val);
    }
    let delimiter = 1000_f64;
    let exponent = std::cmp::min(
        (val.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = format!("{:.1}", val / delimiter.powi(exponent))
        .parse::<f64>()
        .unwrap()
        * 1_f64;
    let unit = units[exponent as usize];
    format!("{} {}", pretty_bytes, unit)
}

pub fn get_prefix(collapsed: bool) -> &'static str {
    if collapsed {
        "└+ "
    } else {
        "└─ "
    }
}

/// Fold a long string to a fixed size string and keep the front and back
///
/// The effective length of the string will be `width - 3` since this fn will use
/// 3 '.'s to replace the omitted content. If the width is less than 3, it will
/// return the original string. This function will also take a stop_filter closure
/// as an indicator of where to split the string. Please note that, this function
/// will take the minial value of (width - 3)/2 and first index that hit the stop_filter
/// as the front half cutting point.
///
/// # Arguments
///
/// * `val` -- The string that needs to be folded.
/// * `width` -- The final target string length.
/// * `start_idx` -- From which index should we start apply the stop_filter
/// * `stop_filter` -- The first half will be cut at the first index that returns false
///    after apply the filter if the index is less than (width - 3)/2
pub fn fold_string<F>(val: &str, width: usize, start_idx: usize, stop_filter: F) -> String
where
    F: FnMut(char) -> bool,
{
    let str_len = val.len();
    if start_idx >= str_len || val[start_idx..].len() <= width || width <= 3 {
        return val.into();
    }

    let first_symbo_pos = val[start_idx..].find(stop_filter).unwrap_or(str_len) + 1;
    let mid_str_len = (width - 3) >> 1;
    let front_len = std::cmp::min(first_symbo_pos, mid_str_len);
    let front_string = val[..front_len].to_string();
    let back_string = val[str_len - width + front_len + 3..].to_string();
    format!("{}...{}", front_string, back_string)
}

/// Returns a set of full cgroup paths that should be filtered out.
///
/// Note that this algorithm recursively whitelists parents of cgroups that are
/// whitelisted. The reason for this is because cgroups are inherently tree-like
/// and displaying a lone cgroup without its ancestors doesn't make much sense.
pub fn calculate_filter_out_set(cgroup: &CgroupModel, filter: &str) -> HashSet<String> {
    fn should_filter_out(cgroup: &CgroupModel, filter: &str, set: &mut HashSet<String>) -> bool {
        // No children
        if cgroup.count == 1 {
            if !cgroup.full_path.contains(filter) {
                set.insert(cgroup.full_path.clone());
                return true;
            }
            return false;
        }

        let mut filter_cgroup = true;
        for child in &cgroup.children {
            if should_filter_out(&child, &filter, set) {
                set.insert(child.full_path.clone());
            } else {
                // We found a child that's not filtered out. That means
                // we have to keep this (the parent cgroup) too.
                filter_cgroup = false;
            }
        }

        if filter_cgroup {
            set.insert(cgroup.full_path.clone());
        }

        filter_cgroup
    }

    let mut set = HashSet::new();
    should_filter_out(&cgroup, &filter, &mut set);
    set
}
