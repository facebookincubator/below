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

#![deny(clippy::all)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use common::open_source_shim;
use enum_iterator::Sequence;
use serde::Deserialize;
use serde::Serialize;

#[macro_use]
pub mod collector;
pub mod cgroup;
pub mod collector_plugin;
mod common_field_ids;
pub mod network;
pub mod process;
pub mod resctrl;
pub mod sample;
mod sample_model;
pub mod system;
pub mod tc_collector_plugin;
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
    VecString(Vec<String>),
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
            Field::U64(v) => v,
            _ => panic!("Operation for unsupported types"),
        }
    }
}

impl From<Field> for i64 {
    fn from(field: Field) -> i64 {
        match field {
            Field::I32(v) => v as i64,
            Field::I64(v) => v,
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

impl From<Vec<String>> for Field {
    fn from(v: Vec<String>) -> Self {
        Field::VecString(v)
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
            (Field::VecString(s), Field::VecString(o)) => s.partial_cmp(o),
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
            Field::VecString(v) => {
                if v.is_empty() {
                    f.write_str("?")
                } else if v.len() == 1 {
                    f.write_str(&v[0])
                } else {
                    f.write_fmt(format_args!("{} …", v[0]))
                }
            }
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

/// Type that contains sub-queriables of the same type, individually retrieveable
/// by some index. It is itself a Queriable.
pub trait QueriableContainer {
    type Idx;
    type SubqueryId: FieldId;
    const IDX_PLACEHOLDER: &'static str = "<idx>.";
    fn split(s: &str) -> Option<(&str, &str)> {
        s.split_once('.')
    }
    fn get_item(&self, idx: &Self::Idx) -> Option<&<Self::SubqueryId as FieldId>::Queriable>;
}

impl<C: QueriableContainer> Queriable for C {
    type FieldId = QueriableContainerFieldId<C>;
    fn query(&self, field_id: &<C as Queriable>::FieldId) -> Option<Field> {
        self.get_item(field_id.idx.as_ref()?)
            .and_then(|sub| sub.query(&field_id.subquery_id.0))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueriableContainerFieldId<C: QueriableContainer> {
    /// None is only for listing variants and otherwise invalid.
    /// If None, shows up as C::IDX_PLACEHOLDER
    pub idx: Option<C::Idx>,
    // Wraps inside a tuple so we can #[derive] traits without adding type constraints
    pub subquery_id: (C::SubqueryId,),
    phantom: PhantomData<C>,
}

impl<C: QueriableContainer> FieldId for QueriableContainerFieldId<C> {
    type Queriable = C;
}

impl<C: QueriableContainer> QueriableContainerFieldId<C> {
    pub fn new(idx: Option<C::Idx>, subquery_id: C::SubqueryId) -> Self {
        Self {
            idx,
            subquery_id: (subquery_id,),
            phantom: PhantomData,
        }
    }
}

impl<C: QueriableContainer> Sequence for QueriableContainerFieldId<C>
where
    C::SubqueryId: Sequence,
{
    const CARDINALITY: usize = C::SubqueryId::CARDINALITY;
    fn next(&self) -> Option<Self> {
        self.subquery_id.0.next().map(|s| Self::new(None, s))
    }
    fn previous(&self) -> Option<Self> {
        self.subquery_id.0.previous().map(|s| Self::new(None, s))
    }
    fn first() -> Option<Self> {
        C::SubqueryId::first().map(|s| Self::new(None, s))
    }
    fn last() -> Option<Self> {
        C::SubqueryId::last().map(|s| Self::new(None, s))
    }
}

impl<C: QueriableContainer> std::fmt::Display for QueriableContainerFieldId<C>
where
    C::Idx: ToString,
    C::SubqueryId: ToString,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.idx.as_ref() {
            Some(idx) => write!(f, "{}.{}", idx.to_string(), self.subquery_id.0.to_string()),
            None => write!(
                f,
                "{}{}",
                C::IDX_PLACEHOLDER,
                self.subquery_id.0.to_string()
            ),
        }
    }
}

impl<C: QueriableContainer> FromStr for QueriableContainerFieldId<C>
where
    C::Idx: FromStr,
    C::SubqueryId: FromStr,
    <C::Idx as FromStr>::Err: Into<anyhow::Error>,
    <C::SubqueryId as FromStr>::Err: Into<anyhow::Error>,
{
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some((idx_str, subquery_id_str)) = C::split(s) {
            Ok(Self::new(
                Some(C::Idx::from_str(idx_str).map_err(Into::into)?),
                C::SubqueryId::from_str(subquery_id_str).map_err(Into::into)?,
            ))
        } else {
            Err(anyhow!(
                "Unable to find a variant of the given enum matching string `{}`.",
                s,
            ))
        }
    }
}

impl<Q: Queriable> QueriableContainer for Vec<Q> {
    type Idx = usize;
    type SubqueryId = Q::FieldId;
    fn get_item(&self, idx: &usize) -> Option<&Q> {
        self.get(*idx)
    }
}

pub type VecFieldId<Q> = QueriableContainerFieldId<Vec<Q>>;

impl<K: Ord, Q: Queriable> QueriableContainer for BTreeMap<K, Q> {
    type Idx = K;
    type SubqueryId = Q::FieldId;
    const IDX_PLACEHOLDER: &'static str = "<key>.";
    fn get_item(&self, idx: &K) -> Option<&Q> {
        self.get(idx)
    }
}

pub type BTreeMapFieldId<K, Q> = QueriableContainerFieldId<BTreeMap<K, Q>>;

pub struct NetworkStats<'a> {
    net: &'a procfs::NetStat,
    ethtool: &'a Option<ethtool::EthtoolStats>,
}

#[derive(Clone, Serialize, Deserialize, below_derive::Queriable)]
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

    #[::below_derive::queriable_derives]
    pub struct TestModel {
        pub msg: String,
    }

    #[test]
    fn test_vec_field_id() {
        let query_str = "1.msg";
        let query = <VecFieldId<TestModel>>::from_str(query_str).expect("bad query str");
        assert_eq!(query, VecFieldId::new(Some(1), TestModelFieldId::Msg),);
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
            data.query(&VecFieldId::new(Some(1), TestModelFieldId::Msg,)),
            Some(Field::Str("world".to_owned()))
        );
    }

    #[test]
    fn test_btreemap_field_id() {
        let query_str = "hello.msg";
        let query =
            <BTreeMapFieldId<String, TestModel>>::from_str(query_str).expect("bad query str");
        assert_eq!(
            query,
            BTreeMapFieldId::new(Some("hello".to_owned()), TestModelFieldId::Msg)
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
            data.query(&BTreeMapFieldId::new(
                Some("hello".to_owned()),
                TestModelFieldId::Msg,
            )),
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
