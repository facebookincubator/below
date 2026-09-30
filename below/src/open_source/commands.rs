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

use std::time::SystemTime;

use anyhow::Result;
use clap::Args;
use clap::Parser;
use tempfile::TempDir;

// This is a shim so we can add FB-internal commands without affecting the
// open source build
#[derive(Debug, Parser)]
pub enum Command {}

#[derive(Debug, Args)]
pub struct SnapshotUpload {}

pub fn snapshot_uploader(
    _init: crate::init::InitToken,
    _upload: &SnapshotUpload,
    _time_begin: SystemTime,
    _time_end: SystemTime,
) -> Result<Option<crate::SnapshotUploader>> {
    Ok(None)
}

#[derive(Debug, Args)]
pub struct SnapshotSource {}

pub fn find_snapshot(
    _init: crate::init::InitToken,
    _source: &SnapshotSource,
    host: Option<String>,
    snapshot: Option<String>,
    _time_range: impl FnOnce() -> Result<(SystemTime, SystemTime)>,
) -> Result<(Option<String>, Option<String>)> {
    Ok((host, snapshot))
}

pub fn fetch_snapshot(
    _init: crate::init::InitToken,
    snapshot: Option<String>,
) -> Result<(Option<String>, Option<TempDir>)> {
    Ok((snapshot, None))
}

pub fn run_command(
    _init: crate::init::InitToken,
    _debug: bool,
    _below_config: &crate::BelowConfig,
    _cmd: &Command,
) -> i32 {
    0
}
