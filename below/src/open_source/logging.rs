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

use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use slog::error;
use slog::o;
use slog::Drain;

use crate::init::InitToken;
use crate::logutil::CommandPaletteDrain;
use crate::logutil::CompoundDecorator;

fn setup_log(_init: InitToken, file: std::fs::File, _debug: bool) -> slog::Logger {
    let decorator = CompoundDecorator::new(file, std::io::stderr());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = CommandPaletteDrain::new(drain).fuse();

    slog::Logger::root(drain, o!())
}

pub fn setup(init: InitToken, path: PathBuf, debug: bool) -> slog::Logger {
    let file = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(_) => {
            let temp_log_path = tempfile::Builder::new()
                .prefix("below.log.")
                .keep(true)
                .tempfile()
                .expect("Failed to create tempfile")
                .path()
                .to_path_buf();
            eprintln!(
                "Log path unavailable. Logging to {}",
                temp_log_path.to_string_lossy()
            );
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(temp_log_path)
                .expect("Failed to open tempfile")
        }
    };

    setup_log(init, file, debug)
}
