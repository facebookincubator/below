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

/// Collects data samples and maintains the latest data
pub struct Collector {
    last: Option<(Sample, Instant)>,
}

impl Collector {
    pub fn new() -> Collector {
        Collector { last: None }
    }

    /// Collect a new `Sample`, returning an updated Model
    pub fn update_model(&mut self) -> Result<Model> {
        let now = Instant::now();
        let sample = collect_sample(true)?;
        let last = self.last.replace((sample, now));
        let model = Model::new(
            SystemTime::now(),
            &self.last.as_ref().unwrap().0,
            last.as_ref().map(|(s, i)| (s, now.duration_since(*i))),
        );
        Ok(model)
    }
}

pub fn opt_add<S: Sized + std::ops::Add<T, Output = S>, T: Sized>(
    a: Option<S>,
    b: Option<T>,
) -> Option<S> {
    a.and_then(|x| b.map(|y| x + y))
}

pub fn opt_multiply<S: Sized + std::ops::Mul<T, Output = S>, T: Sized>(
    a: Option<S>,
    b: Option<T>,
) -> Option<S> {
    a.and_then(|x| b.map(|y| x * y))
}

pub fn get_hostname() -> Result<String> {
    if let Ok(h) = hostname::get() {
        if let Ok(s) = h.into_string() {
            return Ok(s);
        }
    }
    Err(anyhow!("Could not get hostname"))
}

pub fn collect_sample(collect_io_stat: bool) -> Result<Sample> {
    let reader = procfs::ProcReader::new();
    Ok(Sample {
        cgroup: collect_cgroup_sample(&cgroupfs::CgroupReader::root()?, collect_io_stat)?,
        processes: reader.read_all_pids()?,
        system: SystemSample {
            stat: reader.read_stat()?,
            meminfo: reader.read_meminfo()?,
            vmstat: reader.read_vmstat()?,
            hostname: get_hostname()?,
        },
    })
}

/// cgroupfs can give us a NotFound error if the cgroup doesn't have
/// the relevant stat file (e.g. if it is the root cgroup). We
/// translate that into `None` so that other errors are propagated,
/// but omitted data is allowed.
///
/// This method just does that translation for us.
fn wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    if let Err(cgroupfs::Error::IoError(_, ref e)) = v {
        if e.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        if e.kind() == std::io::ErrorKind::Other {
            if let Some(errno) = e.raw_os_error() {
                if errno == /* ENODEV */ 19 {
                    // If the cgroup is removed after a control file is opened,
                    // ENODEV is returned. Ignore it.
                    return Ok(None);
                }
            }
        }
    }
    v.map(Some)
}

/// As above, but in addition, io.stat can have broken formatting due
/// to a kernel bug which will not output more than one page. In such
/// cases we should not fail all data collection, but just omit the io
/// data.
fn io_stat_wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    match wrap(v) {
        Err(cgroupfs::Error::InvalidFileFormat(_)) => Ok(None),
        Err(cgroupfs::Error::UnexpectedLine(_, _)) => Ok(None),
        wrapped => wrapped,
    }
}

fn collect_cgroup_sample(
    reader: &cgroupfs::CgroupReader,
    collect_io_stat: bool,
) -> Result<CgroupSample> {
    let io_stat = if collect_io_stat {
        io_stat_wrap(reader.read_io_stat())?
    } else {
        None
    };
    Ok(CgroupSample {
        cpu_stat: wrap(reader.read_cpu_stat())?,
        io_stat,
        memory_current: wrap(reader.read_memory_current().map(|v| v as i64))?,
        memory_stat: wrap(reader.read_memory_stat())?,
        pressure: wrap(reader.read_pressure())?,
        // We transpose at the end here to convert the
        // Option<Result<BTreeMap... into Result<Option<BTreeMap and
        // then bail any errors with `?` - leaving us with the
        // Option<BTreeMap...
        //
        // The only case this can be None is if the cgroup no longer
        // exists - this is consistent with the above members
        children: wrap(reader.child_cgroup_iter())
            .context("Failed to get iterator over cgroup children")?
            .map(|child_iter| {
                child_iter
                    .map(|child| {
                        collect_cgroup_sample(&child, collect_io_stat).map(|child_sample| {
                            (
                                child
                                    .name()
                                    .file_name()
                                    .expect("Unexpected .. in cgroup path")
                                    .to_string_lossy()
                                    .to_string(),
                                child_sample,
                            )
                        })
                    })
                    .collect::<Result<BTreeMap<String, CgroupSample>>>()
            })
            .transpose()?,
        memory_swap_current: wrap(reader.read_memory_swap_current().map(|v| v as i64))?,
    })
}

macro_rules! usec_pct {
    ($a_opt:expr, $b_opt:expr, $delta:expr) => {{
        let mut ret = None;
        if let (Some(a), Some(b)) = ($a_opt, $b_opt) {
            if a <= b {
                ret = Some((b - a) as f64 * 100.0 / $delta.as_micros() as f64);
            }
        }
        ret
    }};
}

macro_rules! count_per_sec {
    ($a_opt:expr, $b_opt:expr, $delta:expr) => {{
        let mut ret = None;
        if let (Some(a), Some(b)) = ($a_opt, $b_opt) {
            if a <= b {
                ret = Some((b - a) as f64 / $delta.as_secs_f64());
            }
        }
        ret
    }};
}

#[allow(unused)]
macro_rules! get_option_rate {
    ($key:ident, $sample:ident, $last:ident) => {
        $last
            .map(|(l, d)| {
                count_per_sec!(l.$key.map(|s| s as u64), $sample.$key.map(|s| s as u64), d)
            })
            .unwrap_or_default()
            .map(|s| s as u64);
    };
}
