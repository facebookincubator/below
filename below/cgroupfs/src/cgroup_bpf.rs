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

//! BPF-based cgroup stats driver.
//!
//! Owns the `cgroup_bpf` skeleton (a cgroup iterator) and drives it on its own
//! thread, mirroring [`crate::exitstat`]. It is *pull* based: for each request it
//! flushes the cgroup rstat and runs the in-kernel traversal, then replies with a
//! fresh [`CgroupBpfSnapshot`]. The collector requests one snapshot per sample and
//! blocks for it, so the overlaid data is always fresh. This file is the libbpf
//! part: load the skeleton, attach the iterator, drain the map. The record/stat
//! types and their pure decoding live in [`types`]; checking what this host
//! supports and deciding what to read from that lives in [`coverage`].

use std::fs::File;
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::fd::AsFd;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use anyhow::Context;
use anyhow::Result;
use libbpf_rs::CgroupIterOpts;
use libbpf_rs::CgroupIterOrder;
use libbpf_rs::Iter;
use libbpf_rs::IterOpts;
use libbpf_rs::Link;
use libbpf_rs::MapCore as _;
use libbpf_rs::MapFlags;
use libbpf_rs::MapMut;
use libbpf_rs::skel::OpenSkel as _;
use libbpf_rs::skel::SkelBuilder as _;

use crate::CgroupBpfSkelBuilder;

mod coverage;
pub(crate) use coverage::*;

mod types;
pub use types::*;

// bindgen-generated `cgroup_bpf_record` for the open-source build, produced at
// compile time by build.rs (so it can never go stale). The fbcode build gets the
// same struct from the `cgroup_bpf_record` rust_bindgen_library instead.
#[cfg(not(fbcode_build))]
// #[allow], not #[expect]: this wraps bindgen output whose exact triggered-lint
// set is version/output-dependent (e.g. non_upper_case_globals is not tripped --
// the generated consts are UPPER_CASE), so #[expect] would emit
// unfulfilled-expectation warnings. This mirrors rust_bindgen_library, which
// injects #[allow] for the fbcode path.
#[allow(non_camel_case_types, non_upper_case_globals, clippy::all)]
mod open_source {
    include!(concat!(env!("OUT_DIR"), "/cgroup_bpf_record.rs"));
}

/// Drives the cgroup-stat BPF iterator on a dedicated thread.
pub struct CgroupBpfDriver {
    cgroup_root: PathBuf,
    debug: bool,
    req: Receiver<()>,
    resp: Sender<Result<CgroupBpfSnapshot>>,
}

impl CgroupBpfDriver {
    pub fn new(
        cgroup_root: PathBuf,
        debug: bool,
        req: Receiver<()>,
        resp: Sender<Result<CgroupBpfSnapshot>>,
    ) -> Self {
        Self {
            cgroup_root,
            debug,
            req,
            resp,
        }
    }

    /// Run the driver loop: reply to each snapshot request with a fresh snapshot
    /// until the requester goes away, attaching the iterator once up front.
    /// Returns `Err` only on unrecoverable setup failure (no cgroup-iter support,
    /// no CAP_BPF); that is reported once via the error channel and the collector
    /// then reads files.
    pub fn drive(&mut self) -> Result<()> {
        // The driver only runs when a flush kfunc exists (start_cgroup_bpf checks
        // before spawning), so the in-kernel flush always fires.
        bpf_loads_skeleton_attach_iter(
            &self.cgroup_root,
            self.debug,
            CgroupIterOrder::DescendantsPre,
            |link, map, skip, features, items| {
                // Tell the program what this kernel lets it read, before the
                // first traversal.
                BpfReadConfig::detect().write_to(skip, features, items);
                // Outside the loop: the cpu user/system split is clamped against
                // what the previous sample reported.
                let mut cputime = CputimeAdjuster::default();
                while self.req.recv().is_ok() {
                    let snapshot = collect_once(link, map, &mut cputime);
                    if self.resp.send(snapshot).is_err() {
                        break;
                    }
                }
            },
        )
    }
}

/// Run `f` with an attached cgroup iterator (at `cgroup_root`), its results map,
/// and the three maps that configure it. The skeleton is loaded as locals that
/// live only for `f`, avoiding the self-referential lifetime a stored
/// `libbpf_rs` skeleton would need. The rstat flush happens in-kernel during
/// `f`'s traversal, not here.
fn bpf_loads_skeleton_attach_iter<T>(
    cgroup_root: &Path,
    debug: bool,
    order: CgroupIterOrder,
    f: impl FnOnce(&Link, &MapMut<'_>, &MapMut<'_>, &MapMut<'_>, &MapMut<'_>) -> T,
) -> Result<T> {
    let mut skel_builder = CgroupBpfSkelBuilder::default();
    skel_builder.obj_builder.debug(debug);

    let mut object = MaybeUninit::uninit();
    let skel = skel_builder
        .open(&mut object)
        .context("Failed to open cgroup_bpf BPF program")?
        .load()
        .context("Failed to load cgroup_bpf BPF program")?;

    // The iterator is attached to the cgroup root and walks its descendants in
    // pre-order (the root included).
    let root = File::open(cgroup_root)
        .with_context(|| format!("Failed to open cgroup root {cgroup_root:?}"))?;
    let mut iter_opts = CgroupIterOpts::from_fd(root.as_fd());
    iter_opts.order = order;
    let link = skel
        .progs
        .cgroup_bpf_read
        .attach_iter_with_opts(IterOpts::Cgroup(iter_opts))
        .context("Failed to attach cgroup_bpf iterator")?;

    Ok(f(
        &link,
        &skel.maps.results,
        &skel.maps.skip_groups,
        &skel.maps.features,
        &skel.maps.item_index,
    ))
}

/// Drain the results map into a snapshot and clear it for the next run. Each map
/// value is one raw record, decoded directly into the below-side stat.
fn drain_map(map: &MapMut<'_>, cputime: &mut CputimeAdjuster) -> Result<CgroupBpfSnapshot> {
    let keys: Vec<Vec<u8>> = map.keys().collect();
    let mut snapshot = CgroupBpfSnapshot::with_capacity(keys.len());
    for key in &keys {
        // Look the value up; on a transient per-key error, skip decoding it but
        // still delete below so the drain stays exhaustive. A `?` here would
        // return early and leave the remaining collected keys in the map, to be
        // re-read (stale) on the next sample.
        if let Ok(Some(value)) = map.lookup(key, MapFlags::ANY) {
            // The map value is a byte-for-byte cgroup_bpf_record (bindgen'd from
            // the shared header); copy it out, skipping anything the wrong size.
            if value.len() >= std::mem::size_of::<CgroupBpfRecord>() {
                // SAFETY: cgroup_bpf_record is a plain #[repr(C)] struct of
                // integers with no padding requirements beyond 8-byte alignment,
                // and `value` holds at least that many bytes; read_unaligned
                // needs no alignment.
                let rec = unsafe { (value.as_ptr() as *const CgroupBpfRecord).read_unaligned() };
                snapshot.insert(rec.cgroup_id, decode_record(&rec, cputime));
            }
        }
        // Clear the entry so stale cgroups (deleted since) don't accumulate.
        let _ = map.delete(key);
    }
    cputime.forget_dead(&snapshot);
    Ok(snapshot)
}

/// Run one full traversal and return its snapshot. Driving the iterator to EOF
/// runs the BPF program for every cgroup, which populates (and self-flushes) the
/// results map that we then drain. The program writes nothing to the seq stream,
/// so this is one read with no size limit.
fn collect_once(
    link: &Link,
    map: &MapMut<'_>,
    cputime: &mut CputimeAdjuster,
) -> Result<CgroupBpfSnapshot> {
    let mut iter = Iter::new(link).context("Failed to create cgroup iterator instance")?;
    let mut sink = Vec::new();
    iter.read_to_end(&mut sink)
        .context("Failed to drive cgroup iterator")?;
    drain_map(map, cputime)
}
