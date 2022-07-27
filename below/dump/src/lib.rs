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

use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use anyhow::bail;
use anyhow::Error;
use anyhow::Result;
use serde_json::json;
use serde_json::Value;
use toml::value::Value as TValue;

use common::cliutil;
use common::util::get_belowrc_dump_section_key;
use common::util::get_belowrc_filename;
use common::util::timestamp_to_datetime;
use model::EnumIter;
use model::Field;
use model::FieldId;
use model::Queriable;

use store::advance::new_advance_local;
use store::advance::new_advance_remote;
use store::Advance;
use store::Direction;

pub mod btrfs;
pub mod cgroup;
pub mod command;
pub mod disk;
pub mod iface;
pub mod network;
pub mod print;
pub mod process;
pub mod system;
pub mod tmain;
pub mod transport;

#[cfg(test)]
mod test;

use command::expand_fields;
pub use command::DumpCommand;
use command::GeneralOpt;
use command::OutputFormat;
use print::HasRenderConfigForDump;
use tmain::dump_timeseries;
use tmain::Dumper;
use tmain::IterExecResult;

/// Fields available to all commands. Each enum represents some semantics and
/// knows how to extract relevant data from a CommonFieldContext.
#[derive(
    Clone,
    Debug,
    PartialEq,
    below_derive::EnumIter,
    below_derive::EnumFromStr,
    below_derive::EnumToString
)]
pub enum CommonField {
    Timestamp,
    Datetime,
}

/// Context for initializing CommonFields.
pub struct CommonFieldContext {
    pub timestamp: i64,
}

impl CommonField {
    pub fn get_field(&self, ctx: &CommonFieldContext) -> Option<Field> {
        match self {
            Self::Timestamp => Field::from(ctx.timestamp),
            Self::Datetime => Field::from(timestamp_to_datetime(&ctx.timestamp)),
        }
        .into()
    }
}

/// Generic field for dumping different types of models. It's either a
/// CommonField or a FieldId that extracts a Field from a given model. It
/// represents a unified interface for dumpable items.
#[derive(Clone, Debug, PartialEq)]
pub enum DumpField<F: FieldId> {
    Common(CommonField),
    FieldId(F),
}

pub type CgroupField = DumpField<model::SingleCgroupModelFieldId>;
pub type ProcessField = DumpField<model::SingleProcessModelFieldId>;
pub type SystemField = DumpField<model::SystemModelFieldId>;
pub type DiskField = DumpField<model::SingleDiskModelFieldId>;
pub type BtrfsField = DumpField<model::BtrfsModelFieldId>;
pub type NetworkField = DumpField<model::NetworkModelFieldId>;
pub type IfaceField = DumpField<model::SingleNetModelFieldId>;
// Essentially the same as NetworkField
pub type TransportField = DumpField<model::NetworkModelFieldId>;

fn get_advance(
    logger: slog::Logger,
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    opts: &command::GeneralOpt,
) -> Result<(SystemTime, SystemTime, Advance)> {
    let (time_begin, time_end) = cliutil::system_time_range_from_date_and_adjuster(
        opts.begin.as_str(),
        opts.end.as_deref(),
        opts.duration.as_deref(),
        opts.yesterdays.as_deref(),
    )?;

    let mut advance = if let Some(host) = host {
        new_advance_remote(logger.clone(), host, port, time_begin)?
    } else {
        new_advance_local(logger.clone(), dir, time_begin)
    };

    advance.initialize();

    Ok((time_begin, time_end, advance))
}

/// Try to read $HOME/.config/below/belowrc file and generate a list of keys which will
/// be used as fields. Any errors happen in this function will directly trigger a panic.
pub fn parse_pattern<T: FromStr>(
    filename: String,
    pattern_key: String,
    section_key: &str,
) -> Option<Vec<T>> {
    let dump_map = match std::fs::read_to_string(filename) {
        Ok(belowrc_str) => match belowrc_str.parse::<TValue>() {
            Ok(belowrc_val) => belowrc_val
                .get(get_belowrc_dump_section_key())
                .unwrap_or_else(|| {
                    panic!(
                        "Failed to get section key: [{}.{}]",
                        get_belowrc_dump_section_key(),
                        section_key
                    )
                })
                .to_owned(),
            Err(e) => panic!("Failed to parse belowrc file: {:#}", e),
        },
        Err(e) => panic!("Failed to parse belowrc file: {:#}", e),
    };

    Some(
        dump_map
            .get(section_key)
            .unwrap_or_else(|| {
                panic!(
                    "Failed to get section key: [{}.{}]",
                    get_belowrc_dump_section_key(),
                    section_key
                )
            })
            .get(&pattern_key)
            .unwrap_or_else(|| panic!("Failed to get pattern key: {}", pattern_key))
            .as_array()
            .unwrap_or_else(|| panic!("Failed to parse pattern {} value to array.", pattern_key))
            .iter()
            .map(|field| {
                T::from_str(
                    field.as_str().unwrap_or_else(|| {
                        panic!("Failed to parse field key {} into string", field)
                    }),
                )
                .or_else(|_| Err(format!("Failed to parse field key: {}", field)))
                .unwrap()
            })
            .collect(),
    )
}

pub fn run(
    logger: slog::Logger,
    errs: Receiver<Error>,
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    cmd: DumpCommand,
) -> Result<()> {
    let filename = get_belowrc_filename();

    match cmd {
        DumpCommand::System {
            fields,
            opts,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "system")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_SYSTEM_FIELDS,
                },
                detail,
            );
            let system = system::System::new(&opts, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &system,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Disk {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "disk")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_DISK_FIELDS,
                },
                detail,
            );
            let disk = disk::Disk::new(&opts, select, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &disk,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Btrfs {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "btrfs")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_BTRFS_FIELDS,
                },
                detail,
            );
            let btrfs = btrfs::Btrfs::new(&opts, select, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &btrfs,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Process {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "process")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_PROCESS_FIELDS,
                },
                detail,
            );
            let process = process::Process::new(&opts, select, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &process,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Cgroup {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "cgroup")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_CGROUP_FIELDS,
                },
                detail,
            );
            let cgroup = cgroup::Cgroup::new(&opts, select, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &cgroup,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Iface {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "iface")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_IFACE_FIELDS,
                },
                detail,
            );
            let iface = iface::Iface::new(&opts, select, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &iface,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Network {
            fields,
            opts,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "network")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_NETWORK_FIELDS,
                },
                detail,
            );
            let network = network::Network::new(&opts, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &network,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
        DumpCommand::Transport {
            fields,
            opts,
            pattern,
        } => {
            let (time_begin, time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
            let default = opts.everything || opts.default;
            let detail = opts.everything || opts.detail;
            let fields = if let Some(pattern_key) = pattern {
                parse_pattern(filename, pattern_key, "transport")
            } else {
                fields
            };
            let fields = expand_fields(
                match fields.as_ref() {
                    Some(fields) if !default => fields,
                    _ => command::DEFAULT_TRANSPORT_FIELDS,
                },
                detail,
            );
            let transport = transport::Transport::new(&opts, fields);
            let mut output: Box<dyn Write> = match opts.output.as_ref() {
                Some(file_path) => Box::new(File::create(file_path)?),
                None => Box::new(io::stdout()),
            };
            dump_timeseries(
                advance,
                time_begin,
                time_end,
                &transport,
                output.as_mut(),
                opts.output_format,
                opts.br,
                errs,
            )
        }
    }
}
