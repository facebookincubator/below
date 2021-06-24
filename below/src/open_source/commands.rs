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

use structopt::StructOpt;

// This is a shim so we can add FB-internal commands without affecting the
// open source build
#[derive(Debug, StructOpt)]
pub enum Command {}

pub fn run_command(
    _init: crate::init::InitToken,
    _debug: bool,
    _below_config: crate::BelowConfig,
    _cmd: &Command,
) -> i32 {
    0
}
