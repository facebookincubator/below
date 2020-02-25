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
use std::cell::RefCell;
use std::io;
use std::sync::{Arc, RwLock};

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
        f(&mut CompoundRecordDecorator(&self.file, &self.term))
    }
}

pub struct CompoundRecordDecorator<'a, W: 'a, T: 'a>(&'a RefCell<W>, &'a RefCell<T>)
where
    W: io::Write,
    T: io::Write;

impl<'a, W, T> io::Write for CompoundRecordDecorator<'a, W, T>
where
    W: io::Write,
    T: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let log_target = get_current_log_target();
        match log_target {
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
