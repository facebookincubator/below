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

use core::time::Duration;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use libbpf_rs::MapCore;
use libbpf_rs::MapFlags;
use libbpf_rs::RingBufferBuilder;
use libbpf_rs::skel::OpenSkel as _;
use libbpf_rs::skel::Skel as _;
use libbpf_rs::skel::SkelBuilder as _;
use plain::Plain;
use slog::warn;

use crate::ExitstatSkelBuilder;

static PAGE_SIZE: LazyLock<u64> = LazyLock::new(page_size);

#[repr(C)]
#[derive(Default)]
pub struct Metadata {
    pub tid: i32,
    pub ppid: i32,
    pub pgrp: i32,
    pub sid: i32,
    pub cpu: i32,
    pub comm: [u8; 16],
}

// See bpf prog for comments on what each field is
#[repr(C)]
#[derive(Default)]
pub struct ExitStats {
    pub min_flt: u64,
    pub maj_flt: u64,
    pub utime_us: u64,
    pub stime_us: u64,
    pub etime_us: u64,
    pub nr_threads: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    active_rss_pages: u64,
}

#[repr(C)]
#[derive(Default)]
pub struct Event {
    pub meta: Metadata,
    pub stats: ExitStats,
}

unsafe impl Plain for Event {}

fn page_size() -> u64 {
    match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        -1 => panic!("Failed to query page size"),
        x => x as u64,
    }
}

fn lost_event_delta(previous: &mut u64, current: u64) -> u64 {
    let delta = current.wrapping_sub(*previous);
    *previous = current;
    delta
}

pub struct ExitstatDriver {
    logger: slog::Logger,
    debug: bool,
    buffer: Arc<Mutex<procfs::PidMap>>,
}

impl ExitstatDriver {
    pub fn new(logger: slog::Logger, debug: bool) -> Self {
        Self {
            logger,
            debug,
            buffer: Arc::new(Mutex::new(procfs::PidMap::default())),
        }
    }

    pub fn get_buffer(&self) -> Arc<Mutex<procfs::PidMap>> {
        self.buffer.clone()
    }

    fn handle_event(handle: &Arc<Mutex<procfs::PidMap>>, data: &[u8]) {
        let mut event = Event::default();
        plain::copy_from_bytes(&mut event, data).expect("Data buffer was too short");

        // The ffi::CStr constructors don't like interior nuls
        let mut comm_no_interior_nul = Vec::with_capacity(16);
        for b in &event.meta.comm {
            if *b != 0 {
                comm_no_interior_nul.push(*b);
            } else {
                break;
            }
        }
        comm_no_interior_nul.push(0);

        let pidinfo = procfs::PidInfo {
            stat: procfs::PidStat {
                pid: Some(event.meta.tid), // event.meta.pid is actually tgid
                comm: CStr::from_bytes_with_nul(&comm_no_interior_nul).map_or_else(
                    |_| None,
                    |v| v.to_str().map_or_else(|_| None, |v| Some(v.to_string())),
                ),
                state: Some(procfs::PidState::Dead),
                ppid: Some(event.meta.ppid),
                pgrp: Some(event.meta.pgrp),
                session: Some(event.meta.sid),
                minflt: Some(event.stats.min_flt),
                majflt: Some(event.stats.maj_flt),
                user_usecs: Some(event.stats.utime_us),
                system_usecs: Some(event.stats.stime_us),
                num_threads: Some(event.stats.nr_threads),
                running_secs: Some(event.stats.etime_us / 1000000),
                rss_bytes: Some(event.stats.active_rss_pages * *PAGE_SIZE),
                processor: Some(event.meta.cpu),
            },
            io: procfs::PidIo {
                rbytes: Some(event.stats.io_read_bytes),
                wbytes: Some(event.stats.io_write_bytes),
            },
            // It seems to be somewhat tricky to get a cgroup name using bpf. It might be possible
            // with the bpf_get_current_cgroup_id() helper, but that returns what looks like an
            // inode number. I'm not sure if it's easy/possible to translate an inode # to a path.
            cgroup: "?".to_string(),
            // We can't access cmdline b/c it requires taking mmap_sem and a
            // bunch of memory management helpers.
            ..Default::default()
        };

        // handle.lock() only fails if a thread holding the lock panic'd, in which
        // case we should probably panic too.
        handle
            .lock()
            .expect("lock poisoned: a thread holding the lock panicked")
            .insert(event.meta.tid, pidinfo);
    }

    fn handle_lost_events(logger: &slog::Logger, cpu: i32, count: u64) {
        warn!(logger, "Lost {} events on CPU {}", count, cpu);
    }

    fn report_lost_events(&self, counters: Result<Option<Vec<Vec<u8>>>>, previous: &mut Vec<u64>) {
        let values = match counters {
            Ok(Some(values)) => values,
            Ok(None) => {
                warn!(self.logger, "Missing lost event counters");
                return;
            }
            Err(error) => {
                warn!(self.logger, "Failed to read lost event counters: {}", error);
                return;
            }
        };
        previous.resize(values.len(), 0);
        for (cpu, (value, previous)) in values.iter().zip(previous).enumerate() {
            let Ok(bytes) = value.as_slice().try_into() else {
                warn!(self.logger, "Invalid lost event counter on CPU {}", cpu);
                continue;
            };
            let count = lost_event_delta(previous, u64::from_ne_bytes(bytes));
            if count != 0 {
                Self::handle_lost_events(&self.logger, cpu as i32, count);
            }
        }
    }

    /// Loops forever unless an error is hit
    pub fn drive(&mut self) -> Result<()> {
        let mut skel_builder = ExitstatSkelBuilder::default();
        skel_builder.obj_builder.debug(self.debug);

        let mut object = MaybeUninit::uninit();
        let mut skel = skel_builder
            .open(&mut object)
            .context("Failed to open BPF program")?
            .load()
            .context("Failed to load BPF program")?;
        skel.attach().context("Failed to attach BPF program?")?;

        let buffer = self.get_buffer();
        let mut ring_builder = RingBufferBuilder::new();
        ring_builder.add(&skel.maps.events, move |data| {
            Self::handle_event(&buffer, data);
            0
        })?;
        let ring = ring_builder.build()?;
        let mut previous_lost = Vec::new();
        let mut last_lost_check = Instant::now();

        loop {
            ring.poll(Duration::from_millis(100))
                .context("Error polling ring buffer")?;
            if last_lost_check.elapsed() < Duration::from_secs(1) {
                continue;
            }
            last_lost_check = Instant::now();
            self.report_lost_events(
                skel.maps
                    .lost_events
                    .lookup_percpu(&0u32.to_ne_bytes(), MapFlags::ANY)
                    .context("Lost event counter lookup"),
                &mut previous_lost,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_event_deltas() {
        let mut previous = [0, 0];
        assert_eq!(lost_event_delta(&mut previous[0], 0), 0);
        assert_eq!(lost_event_delta(&mut previous[0], 3), 3);
        assert_eq!(lost_event_delta(&mut previous[0], 3), 0);
        assert_eq!(lost_event_delta(&mut previous[0], 5), 2);
        assert_eq!(lost_event_delta(&mut previous[1], 1), 1);
        assert_eq!(previous, [5, 1]);
        previous[0] = u64::MAX;
        assert_eq!(lost_event_delta(&mut previous[0], 1), 2);
        assert_eq!(previous, [1, 1]);
    }

    #[test]
    fn lost_reporting_failures_are_best_effort() {
        let logger = slog::Logger::root(slog::Discard, slog::o!());
        let driver = ExitstatDriver::new(logger, false);
        let mut previous = vec![3, 4];
        driver.report_lost_events(Err(anyhow::anyhow!("Interrupted lookup")), &mut previous);
        assert_eq!(previous, [3, 4]);
        driver.report_lost_events(Ok(None), &mut previous);
        assert_eq!(previous, [3, 4]);
        driver.report_lost_events(
            Ok(Some(vec![vec![0; 7], 5u64.to_ne_bytes().to_vec()])),
            &mut previous,
        );
        assert_eq!(previous, [3, 5]);
        driver.report_lost_events(
            Ok(Some(vec![
                8u64.to_ne_bytes().to_vec(),
                6u64.to_ne_bytes().to_vec(),
            ])),
            &mut previous,
        );
        assert_eq!(previous, [8, 6]);
        let mut record = vec![0; std::mem::size_of::<Event>()];
        record[..4].copy_from_slice(&42i32.to_ne_bytes());
        let buffer = driver.get_buffer();
        ExitstatDriver::handle_event(&buffer, &record);
        assert!(buffer.lock().expect("lock poisoned").contains_key(&42));
    }
}
