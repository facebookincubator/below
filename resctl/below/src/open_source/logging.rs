use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use slog::{o, Drain};
use slog_term;

use crate::init::InitToken;

pub enum LogTo {
    Stderr,
    LogFile(PathBuf),
}

pub fn setup(_init: InitToken, log_to: &LogTo, debug: bool) -> slog::Logger {
    match log_to {
        LogTo::Stderr => {
            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_async::Async::new(drain).build().fuse();
            slog::Logger::root(drain, o!())
        }
        LogTo::LogFile(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("Failed to open log path");

            let mut perms = file
                .metadata()
                .expect("failed to get file metadata for logfile")
                .permissions();
            if perms.mode() != 0o666 {
                // World readable/writable -- the devil's permissions
                perms.set_mode(0o666);
                file.set_permissions(perms)
                    .expect("failed to set permissions on logfile");
            }

            let decorator = slog_term::PlainDecorator::new(file);
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_async::Async::new(drain).build().fuse();

            slog::Logger::root(drain, o!())
        }
    }
}
