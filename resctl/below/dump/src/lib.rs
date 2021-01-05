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
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use toml::value::Value as TValue;

use common::dateutil;
use common::util::translate_datetime;
use model::{Field, FieldId, Queriable};
#[macro_use]
extern crate render;

use store::advance::Advance;
use store::Direction;

#[macro_use]
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

pub use command::DumpCommand;
use command::{expand_fields, GeneralOpt, OutputFormat};
use print::HasRenderConfigForDump;
use tmain::{dump_timeseries, Dumper, IterExecResult};

const BELOW_DUMP_RC: &str = "/.config/below/dumprc";

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
            Self::Datetime => Field::from(translate_datetime(&ctx.timestamp)),
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

pub type CgroupField = DumpField<model::CgroupModelFieldId>;
pub type ProcessField = DumpField<model::SingleProcessModelFieldId>;
pub type SystemField = DumpField<model::SystemModelFieldId>;
pub type DiskField = DumpField<model::SingleDiskModelFieldId>;
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
) -> Result<(SystemTime, Advance)> {
    let mut time_begin = UNIX_EPOCH
        + Duration::from_secs(
            dateutil::HgTime::parse(&opts.begin)
                .ok_or_else(|| anyhow!("Unrecognized begin format"))?
                .unixtime,
        );

    let mut time_end = if opts.end.is_none() {
        SystemTime::now()
    } else {
        UNIX_EPOCH
            + Duration::from_secs(
                dateutil::HgTime::parse(opts.end.as_ref().unwrap())
                    .ok_or_else(|| anyhow!("Unrecognized end format"))?
                    .unixtime,
            )
    };

    if let Some(days) = opts.yesterdays.as_ref() {
        if days.is_empty() || days.find(|c: char| c != 'y').is_some() {
            bail!("Unrecognized days adjuster format: {}", days);
        }
        let time_to_deduct = Duration::from_secs(days.chars().count() as u64 * 86400);
        time_begin -= time_to_deduct;
        time_end -= time_to_deduct;
    }

    let mut advance = if let Some(host) = host {
        Advance::new_with_remote(logger, host, port, time_begin)?
    } else {
        Advance::new(logger.clone(), dir, time_begin)
    };

    advance.initialize();

    Ok((time_end, advance))
}

/// Try to read $HOME/.config/below/dumprc file and generate a list of keys which will
/// be used as fields. Any errors happen in this function will directly trigger a panic.
pub fn parse_pattern<T: FromStr>(
    filename: String,
    pattern_key: String,
    section_key: &str,
) -> Option<Vec<T>> {
    let dumprc_map = match std::fs::read_to_string(filename) {
        Ok(dumprc_str) => match dumprc_str.parse::<TValue>() {
            Ok(dumprc) => dumprc
                .as_table()
                .expect("Failed to parse dumprc: File may be empty.")
                .to_owned(),
            Err(e) => panic!("Failed to parse dumprc file: {}", e),
        },
        Err(e) => panic!("Failed to read dumprc file: {}", e),
    };

    Some(
        dumprc_map
            .get(section_key)
            .unwrap_or_else(|| panic!("Failed to get section key: [{}]", section_key))
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
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    cmd: DumpCommand,
) -> Result<()> {
    let filename = format!(
        "{}{}",
        std::env::var("HOME").expect("Fail to obtain HOME env var"),
        BELOW_DUMP_RC
    );

    match cmd {
        DumpCommand::System {
            fields,
            opts,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &system,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Disk {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &disk,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Process {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &process,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Cgroup {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &cgroup,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Iface {
            fields,
            opts,
            select,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &iface,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Network {
            fields,
            opts,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &network,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
        DumpCommand::Transport {
            fields,
            opts,
            pattern,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts)?;
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
                time_end,
                &transport,
                output.as_mut(),
                opts.output_format,
                opts.br,
            )
        }
    }
}
