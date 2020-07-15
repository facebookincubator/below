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

use slog::{o, Drain};
use slog_term;

use crate::init::InitToken;
use crate::logutil::{CommandPaletteDrain, CompoundDecorator};

pub fn setup(_init: InitToken, path: PathBuf, debug: bool) -> slog::Logger {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("Failed to open log path");

    let mut perms = file
        .metadata()
        .expect("failed to get file metadata for logfile")
        .permissions();
    if perms.mode() & 0o777 != 0o666 {
        // World readable/writable -- the devil's permissions
        perms.set_mode(0o666);
        file.set_permissions(perms)
            .expect("failed to set permissions on logfile");
    }

    let decorator = CompoundDecorator::new(file, std::io::stderr());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = CommandPaletteDrain::new(drain).fuse();

    slog::Logger::root(drain, o!())
}
