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

//! CPU set representation and parsing utilities.
//!
//! This module provides the [`Cpuset`] type for representing a set of CPUs,
//! commonly used for parsing CPU lists from procfs and cgroupfs.

use std::collections::BTreeSet;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

/// A set of CPU IDs, typically parsed from kernel CPU list format (e.g., "0-3,5,7-9").
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Cpuset {
    pub cpus: BTreeSet<u32>,
}

impl Cpuset {
    /// Creates a new empty Cpuset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a Cpuset from an iterator of CPU IDs.
    pub fn from_cpus(cpus: impl IntoIterator<Item = u32>) -> Self {
        Cpuset {
            cpus: cpus.into_iter().collect(),
        }
    }
}

impl std::iter::FromIterator<u32> for Cpuset {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        Cpuset {
            cpus: iter.into_iter().collect(),
        }
    }
}

fn parse_node_range(s: &str) -> std::result::Result<BTreeSet<u32>, String> {
    fn parse_node(s: &str) -> std::result::Result<u32, String> {
        s.parse()
            .map_err(|_| format!("id must be non-negative int: {s}"))
    }
    match s.split_once('-') {
        Some((first, last)) => {
            let first = parse_node(first)?;
            let last = parse_node(last)?;
            if first > last {
                return Err(format!("Invalid range: {s}"));
            }
            Ok((first..(last + 1)).collect())
        }
        None => Ok(BTreeSet::from([parse_node(s)?])),
    }
}

fn nodes_from_str(s: &str) -> std::result::Result<BTreeSet<u32>, String> {
    let mut nodes = BTreeSet::new();
    if s.is_empty() {
        return Ok(nodes);
    }
    for range_str in s.split(',') {
        let mut to_append = parse_node_range(range_str)?;
        nodes.append(&mut to_append);
    }
    Ok(nodes)
}

fn fmt_nodes(f: &mut std::fmt::Formatter<'_>, nodes: &BTreeSet<u32>) -> std::fmt::Result {
    fn print_range(
        f: &mut std::fmt::Formatter<'_>,
        range_start: u32,
        range_end: u32,
    ) -> std::fmt::Result {
        if range_start == range_end {
            write!(f, "{range_start}")
        } else {
            write!(f, "{range_start}-{range_end}")
        }
    }

    let mut range_start = *nodes.iter().next().unwrap_or(&u32::MAX);
    let mut range_end = range_start;
    for cpu in nodes {
        if range_end + 1 == *cpu || range_end == *cpu {
            range_end = *cpu;
        } else {
            print_range(f, range_start, range_end)?;
            write!(f, ",")?;
            range_start = *cpu;
            range_end = *cpu;
        }
    }
    if !nodes.is_empty() {
        print_range(f, range_start, range_end)?;
    }
    Ok(())
}

impl FromStr for Cpuset {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        Ok(Cpuset {
            cpus: nodes_from_str(s)?,
        })
    }
}

impl std::fmt::Display for Cpuset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_nodes(f, &self.cpus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuset_from_str_empty() {
        assert_eq!(
            Cpuset::from_str("").unwrap(),
            Cpuset {
                cpus: BTreeSet::new()
            }
        );
    }

    #[test]
    fn test_cpu_set_from_string() {
        assert_eq!(
            Cpuset::from_str("5").unwrap(),
            Cpuset {
                cpus: BTreeSet::from([5])
            }
        );
        assert_eq!(
            Cpuset::from_str("0-3").unwrap(),
            Cpuset {
                cpus: BTreeSet::from([0, 1, 2, 3])
            }
        );
        assert_eq!(
            Cpuset::from_str("0-3,5,7-9,11").unwrap(),
            Cpuset {
                cpus: BTreeSet::from([0, 1, 2, 3, 5, 7, 8, 9, 11])
            }
        );
    }

    #[test]
    fn test_cpuset_from_str_invalid() {
        assert!(Cpuset::from_str("a").is_err());
        assert!(Cpuset::from_str("-1").is_err());
        assert!(Cpuset::from_str("-1-2").is_err());
        assert!(Cpuset::from_str("0--2").is_err());
        assert!(Cpuset::from_str("0-2-").is_err());
    }

    #[test]
    fn test_cpu_set_display() {
        for s in ["0", "0-12", "0-1,3-4", "0,2-3,5-9999", "0-1,3,5-7,9,11-12"] {
            assert_eq!(Cpuset::from_str(s).unwrap().to_string(), s);
        }
    }

    #[test]
    fn test_cpuset_from_iter() {
        let cpuset: Cpuset = (0..=3).collect();
        assert_eq!(
            cpuset,
            Cpuset {
                cpus: BTreeSet::from([0, 1, 2, 3])
            }
        );
    }
}
