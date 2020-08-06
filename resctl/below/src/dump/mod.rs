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

use crate::advance::Advance;

use crate::dateutil;
use crate::model;
use crate::store::Direction;
use crate::util::translate_datetime;

use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use cursive::utils::markup::StyledString;
use regex::Regex;
use serde_json::{json, Value};

#[macro_use]
pub mod get;
pub mod cgroup;
pub mod command;
pub mod disk;
mod fill;
pub mod iface;
pub mod network;
pub mod print;
pub mod process;
pub mod system;
pub mod tmain;
pub mod transport;

pub use command::DumpCommand;
use command::{
    CgroupField, DiskField, GeneralOpt, IfaceField, NetworkField, OutputFormat, ProcField,
    SysField, TransportField,
};
use fill::Dfill;
use get::Dget;
use print::Dprint;
use tmain::{Dump, IterExecResult};

// I put concrete type for other `blink` fields is because the proc
// macro will use that to infer the underlying type of a field.
//
// For `bttr::class` type of field, however, we can think it's a bridge
// to display multiple fields. For example, `cpu` will actually display
// `usage_pct`, `user_pct`, and `system_pct`. For such kind of behavior,
// the type is not important and the value will always be `None`.
type AwaysNone = Option<()>;

// The DumpType trait is the key of how we make our dump generic.
// Basically, the DumpType trait will be required by all dump related
// traits to provide a guideline on what's the concrete type looks like.
// For how traits work altogether, please take a look at tmain.rs.
// # Types:
// Model ==> The real model typle, like CgroupModel or SingleProcessModel.
// FieldsType ==> The enum tag type we defined in command.rs, like Sys.
// DataType ==> Our struct that implement the BelowDecor per dump module.
pub trait DumpType {
    type Model: Default;
    type FieldsType: Eq + Hash;
    type DataType;
}

fn get_advance(
    logger: slog::Logger,
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    begin: &str,
    end: &Option<String>,
) -> Result<(SystemTime, Advance)> {
    let time_begin = UNIX_EPOCH
        + Duration::from_secs(
            dateutil::HgTime::parse(&begin)
                .ok_or_else(|| anyhow!("Unrecognized begin format"))?
                .unixtime,
        );

    let time_end = if end.is_none() {
        SystemTime::now()
    } else {
        UNIX_EPOCH
            + Duration::from_secs(
                dateutil::HgTime::parse(end.as_ref().unwrap())
                    .ok_or_else(|| anyhow!("Unrecognized end format"))?
                    .unixtime,
            )
    };

    let mut advance = if let Some(host) = host {
        Advance::new_with_remote(logger, host, port, time_begin)?
    } else {
        Advance::new(logger.clone(), dir, time_begin)
    };

    advance.initialize();

    Ok((time_end, advance))
}

pub fn run(
    logger: slog::Logger,
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    cmd: DumpCommand,
) -> Result<()> {
    match cmd {
        DumpCommand::System { fields, opts } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut sys = system::System::new(opts, advance, time_end, None);
            sys.init(fields);
            sys.exec()
        }
        DumpCommand::Disk {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut disk = disk::Disk::new(opts, advance, time_end, select);
            disk.init(fields);
            disk.exec()
        }
        DumpCommand::Process {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut process = process::Process::new(opts, advance, time_end, select);
            process.init(fields);
            process.exec()
        }
        DumpCommand::Cgroup {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut cgroup = cgroup::Cgroup::new(opts, advance, time_end, select);
            cgroup.init(fields);
            cgroup.exec()
        }
        DumpCommand::Iface {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut iface = iface::Iface::new(opts, advance, time_end, select);
            iface.init(fields);
            iface.exec()
        }
        DumpCommand::Network {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut network = network::Network::new(opts, advance, time_end, select);
            network.init(fields);
            network.exec()
        }
        DumpCommand::Transport {
            fields,
            opts,
            select,
        } => {
            let (time_end, advance) = get_advance(logger, dir, host, port, &opts.begin, &opts.end)?;
            let mut transport = transport::Transport::new(opts, advance, time_end, select);
            transport.init(fields);
            transport.exec()
        }
    }
}
