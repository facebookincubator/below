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
use crate::RedirectLogOnFail;

fn setup_log<T: 'static + std::io::Write + std::marker::Send>(
    _init: InitToken,
    file: T,
    _debug: bool,
    error: Option<std::io::Error>,
) -> slog::Logger {
    let decorator = CompoundDecorator::new(file, std::io::stderr());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = CommandPaletteDrain::new(drain).fuse();

    let logger = slog::Logger::root(drain, o!());

    // When we want to redirect the log, also log the open file err to stderr
    if let Some(e) = error {
        error!(
            logger,
            "Fail to open log path: {}\n.Redirecting all log to stderr.", e
        );
    }

    logger
}

pub fn setup(
    init: InitToken,
    path: PathBuf,
    debug: bool,
    redirect: RedirectLogOnFail,
) -> slog::Logger {
    let file_maybe = OpenOptions::new().create(true).append(true).open(path);

    if let Ok(file) = file_maybe.as_ref() {
        // We don't need to worry about the permission setting here since
        // as long as the FS is writable, user can run with sudo to reset
        // file permission.
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
    } else if redirect == RedirectLogOnFail::Off {
        // No redirect, let it crash.
        file_maybe.as_ref().expect("Failed to open log path");
    }

    match file_maybe {
        Ok(f) => setup_log(init, f, debug, None),
        Err(e) => setup_log(init, std::io::stderr(), debug, Some(e)),
    }
}
