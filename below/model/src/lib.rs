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
use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use common::open_source_shim;
use enum_iterator::Sequence;
use serde::Deserialize;
use serde::Serialize;

#[macro_use]
pub mod collector;
pub mod cgroup;
pub mod collector_plugin;
#[cfg(test)]
mod common_field_ids;
pub mod network;
pub mod process;
pub mod resctrl;
pub mod sample;
mod sample_model;
pub mod system;
pub mod tc_model;

open_source_shim!(pub);

pub use cgroup::*;
pub use collector::*;
pub use network::*;
pub use process::*;
pub use resctrl::*;
pub use sample::*;
pub use system::*;
pub use tc_model::*;

/// A wrapper for different field types used in Models. By this way we can query
/// different fields in a single function without using Box.
#[derive(Clone, Debug)]
pub enum Field {
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Str(String),
    PidState(procfs::PidState),
    VecU32(Vec<u32>),
    StrSet(BTreeSet<String>),
    StrU64Map(BTreeMap<String, u64>),
    Cpuset(cgroupfs::Cpuset),
    MemNodes(cgroupfs::MemNodes),
    ResctrlCpuset(resctrlfs::Cpuset),
    ResctrlGroupMode(resctrlfs::GroupMode),
}

impl From<Field> for u64 {
    fn from(field: Field) -> u64 {
        match field {
            Field::U32(v) => v as u64,
            Field::U64(v) => v as u64,
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl From<Field> for i64 {
    fn from(field: Field) -> i64 {
        match field {
            Field::I32(v) => v as i64,
            Field::I64(v) => v as i64,
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl From<Field> for f32 {
    fn from(field: Field) -> f32 {
        let result: f64 = field.into();
        result as f32
    }
}

impl From<Field> for f64 {
    fn from(field: Field) -> f64 {
        match field {
            Field::U32(v) => v as f64,
            Field::U64(v) => v as f64,
            Field::I32(v) => v as f64,
            Field::I64(v) => v as f64,
            Field::F32(v) => v as f64,
            Field::F64(v) => v,
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl From<Field> for String {
    fn from(field: Field) -> String {
        match field {
            Field::Str(v) => v,
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl From<u32> for Field {
    fn from(v: u32) -> Self {
        Field::U32(v)
    }
}

impl From<u64> for Field {
    fn from(v: u64) -> Self {
        Field::U64(v)
    }
}

impl From<i32> for Field {
    fn from(v: i32) -> Self {
        Field::I32(v)
    }
}

impl From<i64> for Field {
    fn from(v: i64) -> Self {
        Field::I64(v)
    }
}

impl From<f32> for Field {
    fn from(v: f32) -> Self {
        Field::F32(v)
    }
}

impl From<f64> for Field {
    fn from(v: f64) -> Self {
        Field::F64(v)
    }
}

impl From<String> for Field {
    fn from(v: String) -> Self {
        Field::Str(v)
    }
}

impl From<procfs::PidState> for Field {
    fn from(v: procfs::PidState) -> Self {
        Field::PidState(v)
    }
}

impl From<Vec<u32>> for Field {
    fn from(v: Vec<u32>) -> Self {
        Field::VecU32(v)
    }
}

impl From<BTreeSet<String>> for Field {
    fn from(v: BTreeSet<String>) -> Self {
        Field::StrSet(v)
    }
}

impl From<BTreeMap<String, u64>> for Field {
    fn from(v: BTreeMap<String, u64>) -> Self {
        Field::StrU64Map(v)
    }
}

impl From<cgroupfs::Cpuset> for Field {
    fn from(v: cgroupfs::Cpuset) -> Self {
        Field::Cpuset(v)
    }
}

impl From<cgroupfs::MemNodes> for Field {
    fn from(v: cgroupfs::MemNodes) -> Self {
        Field::MemNodes(v)
    }
}

impl From<resctrlfs::Cpuset> for Field {
    fn from(v: resctrlfs::Cpuset) -> Self {
        Field::ResctrlCpuset(v)
    }
}

impl From<resctrlfs::GroupMode> for Field {
    fn from(v: resctrlfs::GroupMode) -> Self {
        Field::ResctrlGroupMode(v)
    }
}

impl<T: Into<Field> + Clone> From<&T> for Field {
    fn from(v: &T) -> Self {
        v.clone().into()
    }
}

impl std::ops::Add for Field {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Field::U32(s), Field::U32(o)) => (s + o).into(),
            (Field::U64(s), Field::U64(o)) => (s + o).into(),
            (Field::I32(s), Field::I32(o)) => (s + o).into(),
            (Field::I64(s), Field::I64(o)) => (s + o).into(),
            (Field::F32(s), Field::F32(o)) => (s + o).into(),
            (Field::F64(s), Field::F64(o)) => (s + o).into(),
            (Field::Str(s), Field::Str(o)) => (s + &o).into(),
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Field::U32(s), Field::U32(o)) => s == o,
            (Field::U64(s), Field::U64(o)) => s == o,
            (Field::I32(s), Field::I32(o)) => s == o,
            (Field::I64(s), Field::I64(o)) => s == o,
            (Field::F32(s), Field::F32(o)) => s == o,
            (Field::F64(s), Field::F64(o)) => s == o,
            (Field::Str(s), Field::Str(o)) => s == o,
            (Field::PidState(s), Field::PidState(o)) => s == o,
            (Field::VecU32(s), Field::VecU32(o)) => s == o,
            (Field::StrU64Map(s), Field::StrU64Map(o)) => s == o,
            _ => false,
        }
    }
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Field::U32(s), Field::U32(o)) => s.partial_cmp(o),
            (Field::U64(s), Field::U64(o)) => s.partial_cmp(o),
            (Field::I32(s), Field::I32(o)) => s.partial_cmp(o),
            (Field::I64(s), Field::I64(o)) => s.partial_cmp(o),
            (Field::F32(s), Field::F32(o)) => s.partial_cmp(o),
            (Field::F64(s), Field::F64(o)) => s.partial_cmp(o),
            (Field::Str(s), Field::Str(o)) => s.partial_cmp(o),
            (Field::PidState(s), Field::PidState(o)) => s.partial_cmp(o),
            (Field::VecU32(s), Field::VecU32(o)) => s.partial_cmp(o),
            _ => None,
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Field::U32(v) => v.fmt(f),
            Field::U64(v) => v.fmt(f),
            Field::I32(v) => v.fmt(f),
            Field::I64(v) => v.fmt(f),
            Field::F32(v) => v.fmt(f),
            Field::F64(v) => v.fmt(f),
            Field::Str(v) => v.fmt(f),
            Field::PidState(v) => v.fmt(f),
            Field::VecU32(v) => f.write_fmt(format_args!("{:?}", v)),
            Field::StrSet(v) => f.write_fmt(format_args!(
                "{}",
                v.iter()
                    .cloned()
                    .collect::<Vec<String>>()
                    .as_slice()
                    .join(" ")
            )),
            Field::StrU64Map(v) => f.write_fmt(format_args!(
                "{}",
                v.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>()
                    .as_slice()
                    .join(", ")
            )),
            Field::Cpuset(v) => v.fmt(f),
            Field::MemNodes(v) => v.fmt(f),
            Field::ResctrlCpuset(v) => v.fmt(f),
            Field::ResctrlGroupMode(v) => v.fmt(f),
        }
    }
}

/// Each Model is composed of Fields and optionally sub-Models. The Queriable
/// trait let us query() a Model for a particular Field within the hierarchy
/// with the given FieldId.
pub trait Queriable {
    type FieldId: FieldId<Queriable = Self>;
    fn query(&self, field_id: &Self::FieldId) -> Option<Field>;
}

/// Marker trait to bind FieldId back to Queriable for type inference.
pub trait FieldId: Sized {
    type Queriable: Queriable<FieldId = Self> + ?Sized;
}

pub fn sort_queriables<T: Queriable>(queriables: &mut [&T], field_id: &T::FieldId, reverse: bool) {
    queriables.sort_by(|lhs, rhs| {
        let order = lhs
            .query(field_id)
            .partial_cmp(&rhs.query(field_id))
            .unwrap_or(std::cmp::Ordering::Equal);
        if reverse { order.reverse() } else { order }
    });
}

/// Models containing sub-Models with its own type, similar to a node in a tree.
/// Such Model has a depth value for illustrating the tree hierarchy.
pub trait Recursive {
    fn get_depth(&self) -> usize;
}

/// A `Model` that can be named
pub trait Nameable {
    /// The name of the model (for example "cgroup" or "system")
    fn name() -> &'static str;
}

/// Sequence that wraps a delegate sequence and owns no variants.
trait DelegatedSequence {
    type Delegate: Sequence;
    fn get_delegate(&self) -> &Self::Delegate;
    // Not using From trait as conversion should only be used for Sequence
    fn from_delegate(delegate: Self::Delegate) -> Self;
}

/// Implements Sequence for DelegatedSequence. Must be a macro due to orphan
/// rule. See https://github.com/rust-lang/rfcs/issues/1124
macro_rules! impl_sequence_for_delegated_sequence {
    () => {
        const CARDINALITY: usize = <Self as DelegatedSequence>::Delegate::CARDINALITY;
        fn next(&self) -> Option<Self> {
            self.get_delegate().next().map(Self::from_delegate)
        }
        fn previous(&self) -> Option<Self> {
            self.get_delegate().previous().map(Self::from_delegate)
        }
        fn first() -> Option<Self> {
            <Self as DelegatedSequence>::Delegate::first().map(Self::from_delegate)
        }
        fn last() -> Option<Self> {
            <Self as DelegatedSequence>::Delegate::last().map(Self::from_delegate)
        }
    };
}
pub(crate) use impl_sequence_for_delegated_sequence;

/// Type that makes Vec Queriable if Vec's inner type is Queriable. Uses `idx`
/// to query into a Vec. Uses `subquery_id` to query into the selected item.
#[derive(Clone, Debug, PartialEq)]
pub struct VecFieldId<F: FieldId> {
    /// None is only for listing variants and otherwise invalid.
    pub idx: Option<usize>,
    pub subquery_id: F,
}

impl<F: FieldId> FieldId for VecFieldId<F>
where
    <F as FieldId>::Queriable: Sized,
{
    type Queriable = Vec<F::Queriable>;
}

impl<F: FieldId + Sequence> DelegatedSequence for VecFieldId<F> {
    type Delegate = F;
    fn get_delegate(&self) -> &Self::Delegate {
        &self.subquery_id
    }
    fn from_delegate(delegate: Self::Delegate) -> Self {
        Self {
            idx: None,
            subquery_id: delegate,
        }
    }
}

impl<F: FieldId + Sequence> Sequence for VecFieldId<F> {
    impl_sequence_for_delegated_sequence!();
}

impl<F: FieldId + ToString> ToString for VecFieldId<F> {
    fn to_string(&self) -> String {
        match self.idx {
            Some(idx) => format!("{}.{}", idx, self.subquery_id.to_string()),
            None => format!("<idx>.{}", self.subquery_id.to_string()),
        }
    }
}

impl<F: FieldId + FromStr> FromStr for VecFieldId<F>
where
    <F as FromStr>::Err: Into<anyhow::Error>,
{
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(dot_idx) = s.find('.') {
            Ok(Self {
                idx: Some(s[..dot_idx].parse()?),
                subquery_id: F::from_str(&s[dot_idx + 1..]).map_err(Into::into)?,
            })
        } else {
            Err(anyhow!(
                "Unable to find a variant of the given enum matching string `{}`.",
                s,
            ))
        }
    }
}

impl<Q: Queriable> Queriable for Vec<Q> {
    type FieldId = VecFieldId<Q::FieldId>;
    fn query(&self, field_id: &Self::FieldId) -> Option<Field> {
        self.get(field_id.idx?)
            .and_then(|f| f.query(&field_id.subquery_id))
    }
}

/// Type that makes BTreeMap Queriable if its value is Queriable. Uses `key`
/// to query into a map. Uses `subquery_id` to query into the selected value.
#[derive(Clone, Debug, PartialEq)]
pub struct BTreeMapFieldId<K, F: FieldId> {
    /// None is only for listing variants and otherwise invalid.
    pub key: Option<K>,
    pub subquery_id: F,
}

impl<K: Ord, F: FieldId> FieldId for BTreeMapFieldId<K, F>
where
    <F as FieldId>::Queriable: Sized,
{
    type Queriable = BTreeMap<K, F::Queriable>;
}

impl<K, F: FieldId + Sequence> DelegatedSequence for BTreeMapFieldId<K, F> {
    type Delegate = F;
    fn get_delegate(&self) -> &Self::Delegate {
        &self.subquery_id
    }
    fn from_delegate(delegate: Self::Delegate) -> Self {
        Self {
            key: None,
            subquery_id: delegate,
        }
    }
}

impl<K, F: FieldId + Sequence> Sequence for BTreeMapFieldId<K, F> {
    impl_sequence_for_delegated_sequence!();
}

impl<K: ToString, F: FieldId + ToString> ToString for BTreeMapFieldId<K, F> {
    fn to_string(&self) -> String {
        match &self.key {
            Some(key) => format!("{}.{}", key.to_string(), self.subquery_id.to_string()),
            None => format!("<key>.{}", self.subquery_id.to_string()),
        }
    }
}

impl<K: FromStr, F: FieldId + FromStr> FromStr for BTreeMapFieldId<K, F>
where
    <K as FromStr>::Err: Into<anyhow::Error>,
    <F as FromStr>::Err: Into<anyhow::Error>,
{
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Only works with keys that don't contain dot
        if let Some(dot_idx) = s.find('.') {
            Ok(Self {
                key: Some(K::from_str(&s[..dot_idx]).map_err(Into::into)?),
                subquery_id: F::from_str(&s[dot_idx + 1..]).map_err(Into::into)?,
            })
        } else {
            Err(anyhow!(
                "Unable to find a variant of the given enum matching string `{}`.",
                s,
            ))
        }
    }
}

impl<K: Ord, Q: Queriable> Queriable for BTreeMap<K, Q> {
    type FieldId = BTreeMapFieldId<K, Q::FieldId>;
    fn query(&self, field_id: &Self::FieldId) -> Option<Field> {
        self.get(field_id.key.as_ref()?)
            .and_then(|f| f.query(&field_id.subquery_id))
    }
}

pub struct NetworkStats<'a> {
    net: &'a procfs::NetStat,
    ethtool: &'a Option<ethtool::EthtoolStats>,
}

#[derive(Serialize, Deserialize, below_derive::Queriable)]
pub struct Model {
    #[queriable(ignore)]
    pub time_elapsed: Duration,
    #[queriable(ignore)]
    pub timestamp: SystemTime,
    #[queriable(subquery)]
    pub system: SystemModel,
    #[queriable(subquery)]
    pub cgroup: CgroupModel,
    #[queriable(subquery)]
    pub process: ProcessModel,
    #[queriable(subquery)]
    pub network: NetworkModel,
    #[queriable(subquery)]
    pub gpu: Option<GpuModel>,
    #[queriable(subquery)]
    pub resctrl: Option<ResctrlModel>,
    #[queriable(subquery)]
    pub tc: Option<TcModel>,
}

impl Model {
    /// Construct a `Model` from a Sample and optionally, the last
    /// `CumulativeSample` as well as the `Duration` since it was
    /// collected.
    pub fn new(timestamp: SystemTime, sample: &Sample, last: Option<(&Sample, Duration)>) -> Self {
        Model {
            time_elapsed: last.map(|(_, d)| d).unwrap_or_default(),
            timestamp,
            system: SystemModel::new(&sample.system, last.map(|(s, d)| (&s.system, d))),
            cgroup: CgroupModel::new(
                "<root>".to_string(),
                String::new(),
                0,
                &sample.cgroup,
                last.map(|(s, d)| (&s.cgroup, d)),
            )
            .aggr_top_level_val(),
            process: ProcessModel::new(&sample.processes, last.map(|(s, d)| (&s.processes, d))),
            network: {
                let sample = NetworkStats {
                    net: &sample.netstats,
                    ethtool: &sample.ethtool,
                };
                let network_stats: NetworkStats;

                let last = if let Some((s, d)) = last {
                    network_stats = NetworkStats {
                        net: &s.netstats,
                        ethtool: &s.ethtool,
                    };
                    Some((&network_stats, d))
                } else {
                    None
                };

                NetworkModel::new(&sample, last)
            },
            gpu: sample.gpus.as_ref().map(|gpus| {
                GpuModel::new(&gpus.gpu_map, {
                    if let Some((s, d)) = last {
                        s.gpus.as_ref().map(|g| (&g.gpu_map, d))
                    } else {
                        None
                    }
                })
            }),
            resctrl: sample.resctrl.as_ref().map(|r| {
                ResctrlModel::new(
                    r,
                    if let Some((s, d)) = last {
                        s.resctrl.as_ref().map(|r| (r, d))
                    } else {
                        None
                    },
                )
            }),
            tc: sample.tc.as_ref().map(|tc| {
                TcModel::new(
                    tc,
                    if let Some((s, d)) = last {
                        s.tc.as_ref().map(|tc| (tc, d))
                    } else {
                        None
                    },
                )
            }),
        }
    }
}

/// Get a sample `Model`. There are no guarantees internal consistency of the
/// model, neither are values in the model supposed to be realistic.
pub fn get_sample_model() -> Model {
    serde_json::from_str(sample_model::SAMPLE_MODEL_JSON)
        .expect("Failed to deserialize sample model JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_field_ids() {
        // Ensure COMMON_MODEL_FIELD_IDS is update to date.
        let all_variants: BTreeSet<String> = enum_iterator::all::<ModelFieldId>()
            .map(|v| v.to_string())
            .collect();
        let expected_field_ids: BTreeSet<String> = field_ids::MODEL_FIELD_IDS
            .iter()
            .map(|v| v.to_string())
            .collect();

        assert_eq!(
            all_variants,
            expected_field_ids,
            "new fields: {:?}. missing fields: {:?}",
            expected_field_ids
                .difference(&all_variants)
                .collect::<Vec<_>>(),
            all_variants
                .difference(&expected_field_ids)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_deserialize_sample_model_json() {
        get_sample_model();
    }

    #[derive(Clone, Default, Debug, below_derive::Queriable)]
    pub struct TestModel {
        pub msg: String,
    }

    #[test]
    fn test_vec_field_id() {
        let query_str = "1.msg";
        let query = <VecFieldId<TestModelFieldId>>::from_str(query_str).expect("bad query str");
        assert_eq!(
            query,
            VecFieldId {
                idx: Some(1),
                subquery_id: TestModelFieldId::Msg,
            }
        );
        assert_eq!(query.to_string(), query_str);
    }

    #[test]
    fn test_query_vec() {
        let data = vec![
            TestModel {
                msg: "hello".to_owned(),
            },
            TestModel {
                msg: "world".to_owned(),
            },
        ];
        assert_eq!(
            data.query(&VecFieldId {
                idx: Some(1),
                subquery_id: TestModelFieldId::Msg,
            }),
            Some(Field::Str("world".to_owned()))
        );
    }

    #[test]
    fn test_btreemap_field_id() {
        let query_str = "hello.msg";
        let query = <BTreeMapFieldId<String, TestModelFieldId>>::from_str(query_str)
            .expect("bad query str");
        assert_eq!(
            query,
            BTreeMapFieldId {
                key: Some("hello".to_owned()),
                subquery_id: TestModelFieldId::Msg,
            }
        );
        assert_eq!(query.to_string(), query_str);
    }

    #[test]
    fn test_query_btreemap() {
        let mut data = <BTreeMap<String, TestModel>>::new();
        data.insert(
            "hello".to_owned(),
            TestModel {
                msg: "world".to_owned(),
            },
        );
        data.insert(
            "foo".to_owned(),
            TestModel {
                msg: "bar".to_owned(),
            },
        );
        assert_eq!(
            data.query(&BTreeMapFieldId {
                key: Some("hello".to_owned()),
                subquery_id: TestModelFieldId::Msg,
            }),
            Some(Field::Str("world".to_owned()))
        );
    }

    #[test]
    fn test_query_models() {
        let model = get_sample_model();
        for (field_id, expected) in &[
            (
                "system.hostname",
                Some(Field::Str("hostname.example.com".to_owned())),
            ),
            (
                "cgroup.path:/init.scope/.cpu.usage_pct",
                Some(Field::F64(0.01)),
            ),
            (
                "network.interfaces.eth0.interface",
                Some(Field::Str("eth0".to_owned())),
            ),
            (
                "process.processes.1.comm",
                Some(Field::Str("systemd".to_owned())),
            ),
        ] {
            assert_eq!(
                &model.query(
                    &ModelFieldId::from_str(field_id)
                        .map_err(|e| format!("Failed to parse field id {}: {:?}", field_id, e))
                        .unwrap()
                ),
                expected
            );
        }
    }
}
