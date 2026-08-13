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
use std::sync::mpsc::channel;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use libbpf_rs::CgroupIterOpts;
use libbpf_rs::CgroupIterOrder;
use libbpf_rs::Iter;
use libbpf_rs::IterOpts;
use libbpf_rs::Link;
use libbpf_rs::MapCore as _;
use libbpf_rs::MapFlags;
use libbpf_rs::MapMut;
use libbpf_rs::OpenObject;
use libbpf_rs::skel::OpenSkel as _;
use libbpf_rs::skel::SkelBuilder as _;

use crate::CgroupBpfSkel;
use crate::CgroupBpfSkelBuilder;

mod coverage;
pub(crate) use coverage::*;

#[cfg(test)]
mod tests;

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

/// Start the cgroup-stat BPF driver and return the handle the collector uses to
/// request a snapshot each sample. Mirrors below's `start_exitstat`.
///
/// The program is loaded and attached here, on the caller's thread, so anything
/// that can go wrong -- no CAP_BPF, no cgroup iterator, a program the verifier
/// rejects -- comes back as an error the caller can report. Once the driver
/// thread is running it only reads, so there is nothing left to report
/// asynchronously.
///
/// `Ok(None)` means this kernel exports no rstat flush kfunc. The program has to
/// flush rstat before reading or the stats could be stale, and there is no clean
/// way to make them current without one, so the driver is not started and the
/// collector reads files.
pub fn start_cgroup_bpf(debug: bool, cgroup_root: PathBuf) -> Result<Option<CgroupBpfHandle>> {
    // Probe before loading: the flush answer decides whether to start at all,
    // and one read of the kernel's BTF answers that and what to read.
    let support = probe_host();
    if !support.can_flush_rstat() {
        return Ok(None);
    }

    // The collector drives the BPF thread over channels: exactly one request and
    // one response per sample. That round-trip is a single thread wakeup so using
    // channels instead of calling the driver inline adds negligible overhead.
    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();
    let (ready_send, ready_recv) = channel();
    let mut driver = CgroupBpfDriver {
        cgroup_root,
        debug,
        support,
        ready: ready_send,
        req: req_recv,
        resp: resp_send,
    };
    std::thread::Builder::new()
        .name("cgroup_bpf_driver".to_owned())
        .spawn(move || driver.drive())
        .context("Failed to spawn the cgroup_bpf driver thread")?;

    // Wait for setup before handing back a handle, so a program that cannot load
    // is this call's error rather than a silent no-op the caller finds out about
    // sample by sample.
    match ready_recv.recv() {
        Ok(Ok(())) => Ok(Some(CgroupBpfHandle::new(req_send, resp_recv))),
        Ok(Err(e)) => Err(e),
        Err(_) => bail!("the cgroup_bpf driver thread stopped before it was ready"),
    }
}

/// Drives the cgroup-stat BPF iterator on a dedicated thread.
///
/// Sets up once, reports whether that worked through `ready`, and then only
/// reads: from there a failure belongs to one sample and travels with that
/// sample's reply, so there is nothing left to report out of band.
struct CgroupBpfDriver {
    cgroup_root: PathBuf,
    debug: bool,
    support: KernelSupport,
    ready: Sender<Result<()>>,
    req: Receiver<()>,
    resp: Sender<Result<CgroupBpfSnapshot>>,
}

impl CgroupBpfDriver {
    fn drive(&mut self) {
        // The skeleton borrows this, so it lives as long as the thread does.
        let mut object = MaybeUninit::uninit();
        let mut iter = match load_and_attach_iter(
            &mut object,
            &self.cgroup_root,
            self.debug,
            CgroupIterOrder::DescendantsPre,
        ) {
            Ok(iter) => iter,
            Err(e) => {
                // The starter is waiting on this and turns it into its own error.
                let _ = self.ready.send(Err(e));
                return;
            }
        };
        iter.configure(&self.support);
        if self.ready.send(Ok(())).is_err() {
            return;
        }

        // Outside the loop: the cpu user/system split is clamped against what
        // the previous sample reported.
        let mut cputime = CputimeAdjuster::default();
        while self.req.recv().is_ok() {
            let snapshot = iter.collect(&mut cputime);
            if self.resp.send(snapshot).is_err() {
                break;
            }
        }
    }
}

/// A loaded cgroup_bpf program with its iterator attached, ready to traverse.
struct AttachedCgroupIter<'obj> {
    skel: CgroupBpfSkel<'obj>,
    link: Link,
}

impl AttachedCgroupIter<'_> {
    /// Tell the program what this kernel lets it read. Done once, before the
    /// first traversal.
    fn configure(&mut self, support: &KernelSupport) {
        BpfReadConfig::from_support(support).write_to(
            &self.skel.maps.skip_groups,
            &self.skel.maps.features,
            &self.skel.maps.item_index,
        );
    }

    /// One full traversal, decoded into a snapshot.
    fn collect(&self, cputime: &mut CputimeAdjuster) -> Result<CgroupBpfSnapshot> {
        collect_once(&self.link, &self.skel.maps.results, cputime)
    }
}

/// Load the program and attach the iterator at `cgroup_root`, walking its
/// descendants in `order`. Everything that can fail -- no CAP_BPF, no cgroup
/// iterator, a program the verifier rejects -- fails here, and `start_cgroup_bpf`
/// waits for the answer, so the caller learns about it rather than discovering
/// later that nothing works. The rstat flush happens in-kernel during a
/// traversal, not here.
///
/// `object` is borrowed rather than owned because the skeleton borrows from it:
/// keeping it a local of the driver thread is what avoids either leaking it or
/// wrapping the pair in a self-referential type.
fn load_and_attach_iter<'obj>(
    object: &'obj mut MaybeUninit<OpenObject>,
    cgroup_root: &Path,
    debug: bool,
    order: CgroupIterOrder,
) -> Result<AttachedCgroupIter<'obj>> {
    let mut skel_builder = CgroupBpfSkelBuilder::default();
    skel_builder.obj_builder.debug(debug);

    let skel = skel_builder
        .open(object)
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

    Ok(AttachedCgroupIter { skel, link })
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
