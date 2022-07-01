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

use once_cell::sync::Lazy;
use slog::Drain;
use slog::Level;

use std::cell::RefCell;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

#[derive(PartialEq, Copy, Clone)]
pub enum TargetLog {
    All,
    File,
    Term,
}

pub static LOG_TARGET: Lazy<Arc<RwLock<TargetLog>>> =
    Lazy::new(|| Arc::new(RwLock::new(TargetLog::All)));

pub fn get_current_log_target() -> TargetLog {
    *LOG_TARGET
        .read()
        .expect("Failed to acquire read lock on the LOG_TARGET")
}

pub fn set_current_log_target(target: TargetLog) {
    let mut log_target = LOG_TARGET
        .write()
        .expect("Failed to acquire write lock on the LOG_TARGET");
    *log_target = target;
}

/// CPMsgRecord stands for command palette record which used to pass the
/// logging message from slog object to view objet by LAST_LOG_TO_DISPLAY
pub struct CPMsgRecord {
    level: Level,
    msg: String,
    consumed: bool,
}

impl CPMsgRecord {
    pub fn get_msg(&mut self) -> Option<String> {
        if self.consumed {
            None
        } else {
            self.consumed = true;
            Some(Self::construct_msg(self.level, &self.msg))
        }
    }

    /// Convenience function of construct message.
    // Since we have method in StatsView that raise warning message directly to
    // CommandPalette instead of going through the RwLock process, we need to have
    // such function to align the message format
    pub fn construct_msg(level: Level, msg: &str) -> String {
        format!("{}: {}", level.as_str(), msg)
    }

    fn set_msg(&mut self, msg: String, level: Level) {
        self.msg = msg;
        self.level = level;
        self.consumed = false;
    }

    fn new() -> Self {
        Self {
            level: Level::Trace,
            msg: String::new(),
            consumed: true,
        }
    }
}

/// LAST_LOG_TO_DISPLAY here is used to pass msg to CommandPalette.
// This is necessary because:
// a. we cannot reference view inside log drain for:
//     1. Once log constructed, we are no longer able to access the drain.
//     2. In order to construct a view, we need a constructed log.
// b. We are also not able to pass the view struct as a key value pair since
//    slog's key val pair is a trait that does not implement `Any`, we are not
//    able to downcast it.
// c. Only reference the CommandPalette inside the log is not acceptable since
//    there no implementation of IntoBoxedView<RefCell<View>>
pub static LAST_LOG_TO_DISPLAY: Lazy<Arc<Mutex<CPMsgRecord>>> =
    Lazy::new(|| Arc::new(Mutex::new(CPMsgRecord::new())));

pub fn get_last_log_to_display() -> Option<String> {
    LAST_LOG_TO_DISPLAY
        .lock()
        .expect("Fail to acquire lock for LAST_LOG_TO_DISPLAY")
        .get_msg()
}

pub struct CompoundDecorator<W: io::Write, T: io::Write> {
    file: RefCell<W>,
    term: RefCell<T>,
}

impl<W, T> CompoundDecorator<W, T>
where
    W: io::Write,
    T: io::Write,
{
    pub fn new(file_io: W, term_io: T) -> Self {
        Self {
            file: RefCell::new(file_io),
            term: RefCell::new(term_io),
        }
    }
}

impl<W, T> slog_term::Decorator for CompoundDecorator<W, T>
where
    W: io::Write,
    T: io::Write,
{
    fn with_record<F>(
        &self,
        _record: &slog::Record,
        _logger_values: &slog::OwnedKVList,
        f: F,
    ) -> io::Result<()>
    where
        F: FnOnce(&mut dyn slog_term::RecordDecorator) -> io::Result<()>,
    {
        f(&mut CompoundRecordDecorator(
            &self.file,
            &self.term,
            *LOG_TARGET
                .read()
                .expect("Failed to acquire write lock on the LOG_TARGET"),
        ))
    }
}

pub struct CompoundRecordDecorator<'a, W: 'a, T: 'a>(&'a RefCell<W>, &'a RefCell<T>, TargetLog)
where
    W: io::Write,
    T: io::Write;

impl<'a, W, T> io::Write for CompoundRecordDecorator<'a, W, T>
where
    W: io::Write,
    T: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.2 {
            TargetLog::All => {
                let term_res = self.1.borrow_mut().write(buf);
                let file_res = self.0.borrow_mut().write(buf);
                if let Err(e) = term_res {
                    return Err(e);
                }

                file_res
            }
            TargetLog::File => self.0.borrow_mut().write(buf),
            TargetLog::Term => self.1.borrow_mut().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let term_res = self.1.borrow_mut().flush();
        let file_res = self.0.borrow_mut().flush();
        if let Err(e) = term_res {
            return Err(e);
        }

        file_res
    }
}

impl<'a, W, T> Drop for CompoundRecordDecorator<'a, W, T>
where
    W: io::Write,
    T: io::Write,
{
    fn drop(&mut self) {
        let _ = self.1.borrow_mut().flush();
        let _ = self.0.borrow_mut().flush();
    }
}

impl<'a, W, T> slog_term::RecordDecorator for CompoundRecordDecorator<'a, W, T>
where
    W: io::Write,
    T: io::Write,
{
    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct CommandPaletteDrain<D> {
    drain: D,
}

impl<D> CommandPaletteDrain<D> {
    pub fn new(drain: D) -> Self {
        Self { drain }
    }
}

impl<D> Drain for CommandPaletteDrain<D>
where
    D: Drain,
{
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> std::result::Result<Self::Ok, Self::Err> {
        // We will use tag V as indicator of whether or not log to CommandPalette.
        if record.tag() == "V" {
            LAST_LOG_TO_DISPLAY
                .lock()
                .expect("Fail to acquire write lock for LAST_LOG_TO_DISPLAY")
                .set_msg(format!("{}", record.msg()), record.level());
        }
        self.drain.log(record, values).map(Some).map_err(Some)
    }
}

pub fn get_logger() -> slog::Logger {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    slog::Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!())
}
