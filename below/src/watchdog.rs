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

use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(not(target_has_atomic = "64"))]
compile_error!("below's watchdog requires lock-free 64-bit atomics");

const KMSG_PATH: &str = "/dev/kmsg";
const RECORD_MAX: usize = 800;
const RAW_STACK_MAX: usize = 4096;
const STACK_PAYLOAD_MAX: usize = 192;
const IMMEDIATE_TIMER_DELAY_NS: u64 = 1;

struct FixedBuf<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuf<N> {
    fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    fn len(&self) -> usize {
        self.len
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).expect("FixedBuf only accepts UTF-8 strings")
    }

    fn try_push(&mut self, value: char) -> fmt::Result {
        let mut encoded = [0; 4];
        self.write_str(value.encode_utf8(&mut encoded))
    }
}

impl<const N: usize> fmt::Write for FixedBuf<N> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let Some(end) = self.len.checked_add(value.len()).filter(|end| *end <= N) else {
            return Err(fmt::Error);
        };
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl<const N: usize> fmt::Display for FixedBuf<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

struct WatchdogState {
    timer_fd: OwnedFd,
    control_fd: OwnedFd,
    beat_ns: AtomicU64,
    stop_requested: AtomicBool,
    #[cfg(test)]
    completed_expirations: AtomicU64,
}

fn create_timer_fd() -> io::Result<OwnedFd> {
    // SAFETY: timerfd_create has no pointer arguments and returns a new fd.
    let fd = unsafe {
        libc::timerfd_create(
            libc::CLOCK_MONOTONIC,
            libc::TFD_CLOEXEC | libc::TFD_NONBLOCK,
        )
    };
    owned_fd(fd)
}

fn create_event_fd() -> io::Result<OwnedFd> {
    // SAFETY: eventfd has no pointer arguments and returns a new fd.
    owned_fd(unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) })
}

fn owned_fd(fd: libc::c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` was just returned by a successful fd-creating syscall.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn ns_timespec(ns: u64) -> libc::timespec {
    libc::timespec {
        tv_sec: (ns / 1_000_000_000) as libc::time_t,
        tv_nsec: (ns % 1_000_000_000) as libc::c_long,
    }
}

fn read_counter(fd: libc::c_int) -> io::Result<Option<u64>> {
    let mut value = 0_u64;
    loop {
        // SAFETY: `value` is writable for exactly the supplied size.
        let bytes = unsafe {
            libc::read(
                fd,
                (&mut value as *mut u64).cast(),
                mem::size_of_val(&value),
            )
        };
        if bytes == mem::size_of_val(&value) as isize {
            return Ok(Some(value));
        }
        if bytes < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(error);
        }
        return Err(io::Error::from_raw_os_error(libc::EMSGSIZE));
    }
}

fn notify_worker(fd: libc::c_int) {
    let value = 1_u64;
    loop {
        // SAFETY: `value` is readable for exactly the supplied size.
        let bytes =
            unsafe { libc::write(fd, (&value as *const u64).cast(), mem::size_of_val(&value)) };
        if bytes == mem::size_of_val(&value) as isize {
            return;
        }
        if bytes < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
        }
        return;
    }
}

fn arm_timer(fd: libc::c_int, initial_ns: u64, interval: Duration) -> io::Result<()> {
    let timer = libc::itimerspec {
        it_interval: ns_timespec(duration_ns(interval)),
        it_value: ns_timespec(initial_ns),
    };
    loop {
        // SAFETY: `timer` is valid for the call and `fd` remains open.
        if unsafe { libc::timerfd_settime(fd, 0, &timer, std::ptr::null_mut()) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

fn arm_heartbeat_timer(shared: &WatchdogState, timeout: Duration) -> io::Result<()> {
    let heartbeat_ns = shared.beat_ns.load(Ordering::Acquire);
    let initial_ns = if heartbeat_ns == 0 {
        0
    } else {
        heartbeat_timer_delay_ns(heartbeat_ns, monotonic_ns()?, timeout)
    };
    arm_timer(shared.timer_fd.as_raw_fd(), initial_ns, timeout)
}

fn heartbeat_timer_delay_ns(heartbeat_ns: u64, observed_ns: u64, timeout: Duration) -> u64 {
    duration_ns(timeout)
        .saturating_sub(observed_ns.saturating_sub(heartbeat_ns))
        .max(IMMEDIATE_TIMER_DELAY_NS)
}

fn monotonic_ns() -> io::Result<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: CLOCK_MONOTONIC is valid and `time` is writable for the call.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((time.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(time.tv_nsec as u64))
}

pub(crate) struct Watchdog {
    shared: Arc<WatchdogState>,
    worker: Option<JoinHandle<()>>,
}

impl Watchdog {
    pub(crate) fn start(timeout: Duration, store_writer_tid: libc::pid_t) -> io::Result<Self> {
        Self::start_with_options(StartOptions {
            timeout,
            kmsg_path: Path::new(KMSG_PATH),
            record_stack_path: None,
            store_writer_stack_path: None,
            store_writer_tid,
        })
    }

    fn start_with_options(options: StartOptions<'_>) -> io::Result<Self> {
        if options.timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "watchdog timeout must be positive",
            ));
        }

        let kmsg = open_kmsg(options.kmsg_path)?;
        let record_tid = current_tid();
        if options.store_writer_tid <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "store-writer TID has not been published",
            ));
        }
        let shared = Arc::new(WatchdogState {
            timer_fd: create_timer_fd()?,
            control_fd: create_event_fd()?,
            beat_ns: AtomicU64::new(monotonic_ns()?),
            stop_requested: AtomicBool::new(false),
            #[cfg(test)]
            completed_expirations: AtomicU64::new(0),
        });
        arm_heartbeat_timer(&shared, options.timeout)?;
        let targets = WorkerTargets {
            record_tid,
            store_writer_tid: options.store_writer_tid,
            record_stack_path: options.record_stack_path.map(Path::to_path_buf),
            store_writer_stack_path: options.store_writer_stack_path.map(Path::to_path_buf),
        };
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("below-watchdog".to_owned())
            .spawn(move || watchdog_thread_main(worker_shared, targets, kmsg, options.timeout))?;

        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }

    pub(crate) fn beat(&self) {
        if let Ok(heartbeat_ns) = monotonic_ns() {
            self.shared.beat_ns.store(heartbeat_ns, Ordering::Release);
            notify_worker(self.shared.control_fd.as_raw_fd());
        }
    }
}

struct StartOptions<'a> {
    timeout: Duration,
    kmsg_path: &'a Path,
    record_stack_path: Option<&'a Path>,
    store_writer_stack_path: Option<&'a Path>,
    store_writer_tid: libc::pid_t,
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.shared.stop_requested.store(true, Ordering::Release);
        notify_worker(self.shared.control_fd.as_raw_fd());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct WorkerTargets {
    record_tid: libc::pid_t,
    store_writer_tid: libc::pid_t,
    record_stack_path: Option<PathBuf>,
    store_writer_stack_path: Option<PathBuf>,
}

fn watchdog_thread_main(
    shared: Arc<WatchdogState>,
    targets: WorkerTargets,
    mut kmsg: File,
    timeout: Duration,
) {
    let mut fds = [
        libc::pollfd {
            fd: shared.timer_fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: shared.control_fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    loop {
        fds.iter_mut().for_each(|fd| fd.revents = 0);
        // SAFETY: `fds` is valid and writable for both entries during the call.
        let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            emit_error(&mut kmsg, "poll", error.raw_os_error().unwrap_or(libc::EIO));
            return;
        }

        if fds[1].revents & libc::POLLIN != 0 {
            match read_counter(shared.control_fd.as_raw_fd()) {
                Ok(Some(_)) if shared.stop_requested.load(Ordering::Acquire) => return,
                Ok(Some(_)) => {
                    if let Err(error) = arm_heartbeat_timer(&shared, timeout) {
                        emit_error(
                            &mut kmsg,
                            "timerfd_settime",
                            error.raw_os_error().unwrap_or(libc::EIO),
                        );
                        return;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    emit_error(
                        &mut kmsg,
                        "control_read",
                        error.raw_os_error().unwrap_or(libc::EIO),
                    );
                    return;
                }
            }
        }

        if fds
            .iter()
            .any(|fd| fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0)
        {
            emit_poll_error(&mut kmsg, fds[0].revents, fds[1].revents);
            return;
        }
        if fds[0].revents & libc::POLLIN == 0 {
            continue;
        }
        match read_counter(shared.timer_fd.as_raw_fd()) {
            Ok(Some(_)) => {}
            Ok(None) => continue,
            Err(error) => {
                emit_error(
                    &mut kmsg,
                    "timer_read",
                    error.raw_os_error().unwrap_or(libc::EIO),
                );
                return;
            }
        }

        let heartbeat_ns = shared.beat_ns.load(Ordering::Acquire);
        let observed_ns = match monotonic_ns() {
            Ok(observed_ns) => observed_ns,
            Err(error) => {
                emit_error(
                    &mut kmsg,
                    "clock_gettime",
                    error.raw_os_error().unwrap_or(libc::EIO),
                );
                return;
            }
        };
        let heartbeat_age_ns = observed_ns.saturating_sub(heartbeat_ns);
        if heartbeat_ns == 0 || heartbeat_age_ns < duration_ns(timeout) {
            if let Err(error) = arm_heartbeat_timer(&shared, timeout) {
                emit_error(
                    &mut kmsg,
                    "timerfd_settime",
                    error.raw_os_error().unwrap_or(libc::EIO),
                );
                return;
            }
            continue;
        }

        let record = capture_stack(targets.record_tid, targets.record_stack_path.as_deref());
        let store_writer = capture_stack(
            targets.store_writer_tid,
            targets.store_writer_stack_path.as_deref(),
        );
        if shared.beat_ns.load(Ordering::Acquire) == heartbeat_ns {
            let _ = emit_report(
                &mut kmsg,
                heartbeat_ns,
                observed_ns,
                timeout,
                &record,
                &store_writer,
            );
        }
        #[cfg(test)]
        shared.completed_expirations.fetch_add(1, Ordering::Release);
    }
}

struct StackCapture {
    tid: libc::pid_t,
    stack: FixedBuf<STACK_PAYLOAD_MAX>,
    truncated: bool,
    status: StackStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StackStatus {
    Ok,
    OpenFailed,
    ReadFailed,
    Empty,
}

impl StackStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::OpenFailed => "open_failed",
            Self::ReadFailed => "read_failed",
            Self::Empty => "empty",
        }
    }
}

fn capture_stack(watched_tid: libc::pid_t, override_path: Option<&Path>) -> StackCapture {
    if watched_tid <= 0 {
        return stack_error(watched_tid, StackStatus::OpenFailed);
    }
    let mut default_path = FixedBuf::<64>::new();
    let path = match override_path {
        Some(path) => path,
        None => {
            if write!(&mut default_path, "/proc/self/task/{watched_tid}/stack").is_err() {
                return stack_error(watched_tid, StackStatus::OpenFailed);
            }
            Path::new(default_path.as_str())
        }
    };

    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return stack_error(watched_tid, StackStatus::OpenFailed),
    };
    read_stack(watched_tid, file)
}

fn read_stack(watched_tid: libc::pid_t, mut reader: impl Read) -> StackCapture {
    let mut bytes = [0; RAW_STACK_MAX + 1];
    let mut len = 0;
    loop {
        match reader.read(&mut bytes[len..]) {
            Ok(0) if len == 0 => {
                return stack_error(watched_tid, StackStatus::Empty);
            }
            Ok(0) => {
                let (stack, truncated) = normalize_stack(&bytes[..len], false);
                let status = if stack.len() == 0 {
                    StackStatus::Empty
                } else {
                    StackStatus::Ok
                };
                return StackCapture {
                    tid: watched_tid,
                    stack,
                    truncated,
                    status,
                };
            }
            Ok(read) => {
                len += read;
                if len == bytes.len() {
                    let (stack, truncated) = normalize_stack(&bytes[..RAW_STACK_MAX], true);
                    let status = if stack.len() == 0 {
                        StackStatus::Empty
                    } else {
                        StackStatus::Ok
                    };
                    return StackCapture {
                        tid: watched_tid,
                        stack,
                        truncated,
                        status,
                    };
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return stack_error(watched_tid, StackStatus::ReadFailed),
        }
    }
}

fn stack_error(tid: libc::pid_t, status: StackStatus) -> StackCapture {
    StackCapture {
        tid,
        stack: FixedBuf::new(),
        truncated: false,
        status,
    }
}

fn emit_report<W: io::Write>(
    kmsg: &mut W,
    heartbeat_ns: u64,
    observed_ns: u64,
    timeout: Duration,
    record_stack: &StackCapture,
    store_writer_stack: &StackCapture,
) -> bool {
    let mut record = FixedBuf::<RECORD_MAX>::new();
    let heartbeat_age_ms = observed_ns.saturating_sub(heartbeat_ns) / 1_000_000;
    if writeln!(
        &mut record,
        "<4>below watchdog: kind=stall heartbeat_age_ms={} timeout_ms={} pid={} heartbeat_mono_ms={} record_tid={} record_stack_truncated={} record_stack_status={} record_stack={} store_writer_tid={} store_writer_stack_truncated={} store_writer_stack_status={} store_writer_stack={}",
        heartbeat_age_ms,
        duration_ns(timeout) / 1_000_000,
        std::process::id(),
        heartbeat_ns / 1_000_000,
        record_stack.tid,
        u8::from(record_stack.truncated),
        record_stack.status.as_str(),
        record_stack.stack,
        store_writer_stack.tid,
        u8::from(store_writer_stack.truncated),
        store_writer_stack.status.as_str(),
        store_writer_stack.stack,
    )
    .is_err()
    {
        return false;
    }

    write_record(kmsg, record.as_bytes())
}

fn emit_error<W: io::Write>(kmsg: &mut W, operation: &str, error: libc::c_int) {
    let mut record = FixedBuf::<256>::new();
    if writeln!(
        &mut record,
        "<3>below watchdog: kind=error operation={operation} error={error}"
    )
    .is_ok()
    {
        let _ = write_record(kmsg, record.as_bytes());
    }
}

fn emit_poll_error<W: io::Write>(
    kmsg: &mut W,
    timer_revents: libc::c_short,
    control_revents: libc::c_short,
) {
    let mut record = FixedBuf::<256>::new();
    if writeln!(
        &mut record,
        "<3>below watchdog: kind=error operation=poll timer_revents={timer_revents} control_revents={control_revents}"
    )
    .is_ok()
    {
        let _ = write_record(kmsg, record.as_bytes());
    }
}

fn write_record<W: io::Write>(writer: &mut W, record: &[u8]) -> bool {
    loop {
        match writer.write(record) {
            Ok(written) => return written == record.len(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
    }
}

fn normalize_stack(bytes: &[u8], input_truncated: bool) -> (FixedBuf<STACK_PAYLOAD_MAX>, bool) {
    let mut output = FixedBuf::<STACK_PAYLOAD_MAX>::new();
    let mut truncated = input_truncated;
    let mut first_frame = true;

    for raw_line in bytes.split(|byte| *byte == b'\n' || *byte == b'\r') {
        let mut line = trim_ascii(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"[<0>] ") {
            line = &line[6..];
        }
        if !first_frame && output.try_push(';').is_err() {
            truncated = true;
            break;
        }
        first_frame = false;

        let mut index = 0;
        while index < line.len() {
            if let Some(end) = kernel_offset_end(line, index) {
                index = end;
                continue;
            }
            let byte = line[index];
            let value = if byte.is_ascii_whitespace() || !byte.is_ascii_graphic() {
                '_'
            } else {
                byte as char
            };
            if output.try_push(value).is_err() {
                truncated = true;
                return (output, truncated);
            }
            index += 1;
        }
    }
    (output, truncated)
}

fn kernel_offset_end(line: &[u8], start: usize) -> Option<usize> {
    if line.get(start..start + 3)? != b"+0x" {
        return None;
    }
    let mut index = start + 3;
    let first_start = index;
    while line.get(index).is_some_and(u8::is_ascii_hexdigit) {
        index += 1;
    }
    if index == first_start || line.get(index..index + 3)? != b"/0x" {
        return None;
    }
    index += 3;
    let second_start = index;
    while line.get(index).is_some_and(u8::is_ascii_hexdigit) {
        index += 1;
    }
    (index > second_start).then_some(index)
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn open_kmsg(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .append(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    if path == Path::new(KMSG_PATH) && !file.metadata()?.file_type().is_char_device() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a character device", path.display()),
        ));
    }
    Ok(file)
}

fn duration_ns(duration: Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

pub(crate) fn current_tid() -> libc::pid_t {
    // SAFETY: SYS_gettid has no arguments and returns the caller's thread ID.
    unsafe { libc::syscall(libc::SYS_gettid) as libc::pid_t }
}

#[cfg(test)]
mod tests {
    use std::array;
    use std::collections::VecDeque;
    use std::fs;
    use std::io::Cursor;
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;
    use std::sync::mpsc::RecvTimeoutError;
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use std::time::Instant;

    use clap::Parser;
    use tempfile::NamedTempFile;
    use tempfile::TempDir;

    use super::*;
    use crate::Command;
    use crate::Opt;

    fn start_test_watchdog(
        timeout: Duration,
        sink: &Path,
        record_stack: &Path,
        store_writer_stack: &Path,
    ) -> io::Result<Watchdog> {
        Watchdog::start_with_options(StartOptions {
            timeout,
            kmsg_path: sink,
            record_stack_path: Some(record_stack),
            store_writer_stack_path: Some(store_writer_stack),
            store_writer_tid: current_tid(),
        })
    }

    fn wait_for(predicate: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if predicate() {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        predicate()
    }

    fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
        line.split_ascii_whitespace().find_map(|part| {
            let (candidate, value) = part.split_once('=')?;
            (candidate == key).then_some(value)
        })
    }

    fn assert_schema(line: &str) {
        let mut parts = line.split_ascii_whitespace();
        assert_eq!(parts.next(), Some("<4>below"));
        assert_eq!(parts.next(), Some("watchdog:"));
        let keys = parts
            .map(|part| part.split_once('=').expect("key=value field").0)
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            [
                "kind",
                "heartbeat_age_ms",
                "timeout_ms",
                "pid",
                "heartbeat_mono_ms",
                "record_tid",
                "record_stack_truncated",
                "record_stack_status",
                "record_stack",
                "store_writer_tid",
                "store_writer_stack_truncated",
                "store_writer_stack_status",
                "store_writer_stack",
            ]
        );
    }

    fn captured_stack(tid: libc::pid_t, value: &[u8]) -> StackCapture {
        let (stack, truncated) = normalize_stack(value, false);
        StackCapture {
            tid,
            stack,
            truncated,
            status: StackStatus::Ok,
        }
    }

    fn open_fifo_writer(path: &Path) -> File {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)
            {
                Ok(writer) => return writer,
                Err(error)
                    if error.raw_os_error() == Some(libc::ENXIO) && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("open fifo writer: {error}"),
            }
        }
    }

    fn make_fifo(dir: &TempDir) -> PathBuf {
        let fifo = dir.path().join("stack");
        let path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("fifo path");
        // SAFETY: `path` is NUL-terminated and valid for the duration of the call.
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        fifo
    }

    #[derive(Clone, Copy)]
    enum WriteOutcome {
        Full,
        Short,
        Failure,
        Interrupted,
    }

    struct ScriptedWriter {
        outcomes: VecDeque<WriteOutcome>,
        attempts: usize,
        written: Vec<String>,
    }

    impl ScriptedWriter {
        fn new(outcomes: impl IntoIterator<Item = WriteOutcome>) -> Self {
            Self {
                outcomes: outcomes.into_iter().collect(),
                attempts: 0,
                written: Vec::new(),
            }
        }
    }

    impl io::Write for ScriptedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.attempts += 1;
            match self.outcomes.pop_front().unwrap_or(WriteOutcome::Full) {
                WriteOutcome::Full => {
                    self.written.push(String::from_utf8_lossy(buf).into_owned());
                    Ok(buf.len())
                }
                WriteOutcome::Short => Ok(buf.len() - 1),
                WriteOutcome::Failure => Err(io::Error::from_raw_os_error(libc::EIO)),
                WriteOutcome::Interrupted => Err(io::Error::from(io::ErrorKind::Interrupted)),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn calculates_heartbeat_timer_delay() {
        let timeout = Duration::from_millis(100);
        let heartbeat_ns = 1_000_000_000;

        assert_eq!(
            heartbeat_timer_delay_ns(heartbeat_ns, heartbeat_ns, timeout),
            duration_ns(timeout)
        );
        assert_eq!(
            heartbeat_timer_delay_ns(heartbeat_ns, heartbeat_ns + 25_000_000, timeout),
            duration_ns(Duration::from_millis(75))
        );
        assert_eq!(
            heartbeat_timer_delay_ns(heartbeat_ns, heartbeat_ns + 100_000_000, timeout),
            IMMEDIATE_TIMER_DELAY_NS
        );
        assert_eq!(
            heartbeat_timer_delay_ns(heartbeat_ns, heartbeat_ns + 200_000_000, timeout),
            IMMEDIATE_TIMER_DELAY_NS
        );
    }

    #[test]
    fn normalizes_kernel_stack() {
        let (stack, truncated) = normalize_stack(
            b"[<0>] do_freezer_trap+0x1a/0x90\n[<0>] module_frame+0xABC/0xDEF [module name]\n[<1>] other+0x2/0x3 unparsed+0x/0x2 frame\n",
            false,
        );
        assert_eq!(
            stack.as_str(),
            "do_freezer_trap;module_frame_[module_name];[<1>]_other_unparsed+0x/0x2_frame"
        );
        assert!(!truncated);
    }

    #[test]
    fn distinguishes_exact_raw_limit_from_truncation() {
        let mut exact = vec![b' '; RAW_STACK_MAX];
        exact[RAW_STACK_MAX - 6..].copy_from_slice(b"frame\n");
        let capture = read_stack(11, Cursor::new(exact));
        assert_eq!(capture.status, StackStatus::Ok);
        assert_eq!(capture.stack.as_str(), "frame");
        assert!(!capture.truncated);

        let mut over = vec![b' '; RAW_STACK_MAX + 1];
        over[RAW_STACK_MAX - 6..RAW_STACK_MAX].copy_from_slice(b"frame\n");
        let capture = read_stack(11, Cursor::new(over));
        assert_eq!(capture.stack.as_str(), "frame");
        assert!(capture.truncated);
    }

    #[test]
    fn emits_one_bounded_combined_row() {
        let input = array::from_fn::<_, 1024, _>(|_| b'x');
        let record = captured_stack(i32::MAX, &input);
        let store_writer = captured_stack(i32::MAX, &input);
        let mut writer = ScriptedWriter::new([]);
        assert!(emit_report(
            &mut writer,
            1,
            u64::MAX,
            Duration::from_secs(86_400),
            &record,
            &store_writer,
        ));

        let row = &writer.written[0];
        assert_schema(row);
        assert!(row.len() < RECORD_MAX);
        assert_eq!(field(row, "heartbeat_age_ms"), Some("18446744073709"));
        assert_eq!(field(row, "timeout_ms"), Some("86400000"));
        assert_eq!(field(row, "heartbeat_mono_ms"), Some("0"));
        assert_eq!(field(row, "record_stack_status"), Some("ok"));
        assert_eq!(field(row, "store_writer_stack_status"), Some("ok"));
        assert_eq!(field(row, "record_tid"), Some("2147483647"));
        assert_eq!(field(row, "store_writer_tid"), Some("2147483647"));
        assert_eq!(field(row, "record_stack_truncated"), Some("1"));
        assert_eq!(field(row, "store_writer_stack_truncated"), Some("1"));
    }

    #[test]
    fn reports_independent_stack_errors() {
        let empty = NamedTempFile::new().expect("empty stack");
        let missing = empty.path().with_extension("missing");
        let directory = TempDir::new().expect("directory stack");
        let record = capture_stack(11, Some(empty.path()));
        let store_writer = capture_stack(12, Some(&missing));
        let read_failed = capture_stack(13, Some(directory.path()));
        let mut writer = ScriptedWriter::new([]);
        assert!(emit_report(
            &mut writer,
            2_000_000_000,
            3_000_000_000,
            Duration::from_secs(1),
            &record,
            &store_writer,
        ));
        assert!(emit_report(
            &mut writer,
            2_000_000_000,
            3_000_000_000,
            Duration::from_secs(1),
            &read_failed,
            &store_writer,
        ));

        let row = &writer.written[0];
        assert_eq!(field(row, "record_stack_status"), Some("empty"));
        assert_eq!(field(row, "record_stack"), Some(""));
        assert_eq!(field(row, "store_writer_stack_status"), Some("open_failed"));
        assert_eq!(field(row, "store_writer_stack"), Some(""));
        let row = &writer.written[1];
        assert_eq!(field(row, "record_stack_status"), Some("read_failed"));
        assert_eq!(field(row, "record_stack"), Some(""));
    }

    #[test]
    fn write_failures_are_left_for_the_next_expiration() {
        let record = captured_stack(11, b"record_frame\n");
        let store_writer = captured_stack(12, b"store_writer_frame\n");
        let mut failed = ScriptedWriter::new([
            WriteOutcome::Interrupted,
            WriteOutcome::Failure,
            WriteOutcome::Full,
        ]);
        assert!(!emit_report(
            &mut failed,
            2_000_000_000,
            3_000_000_000,
            Duration::from_secs(1),
            &record,
            &store_writer,
        ));
        assert_eq!(failed.attempts, 2);
        assert!(emit_report(
            &mut failed,
            2_000_000_000,
            4_000_000_000,
            Duration::from_secs(1),
            &record,
            &store_writer,
        ));
        assert_eq!(failed.attempts, 3);

        let mut short = ScriptedWriter::new([WriteOutcome::Short, WriteOutcome::Full]);
        assert!(!emit_report(
            &mut short,
            2_000_000_000,
            3_000_000_000,
            Duration::from_secs(1),
            &record,
            &store_writer,
        ));
        assert_eq!(short.attempts, 1);
        assert!(emit_report(
            &mut short,
            2_000_000_000,
            4_000_000_000,
            Duration::from_secs(1),
            &record,
            &store_writer,
        ));
        assert_eq!(short.attempts, 2);
    }

    #[test]
    fn serializes_runtime_errors() {
        let mut writer = ScriptedWriter::new([]);
        emit_error(&mut writer, "timerfd_settime", libc::EBADF);
        emit_poll_error(&mut writer, libc::POLLERR, libc::POLLHUP);

        assert_eq!(
            writer.written,
            [
                format!(
                    "<3>below watchdog: kind=error operation=timerfd_settime error={}\n",
                    libc::EBADF
                ),
                format!(
                    "<3>below watchdog: kind=error operation=poll timer_revents={} control_revents={}\n",
                    libc::POLLERR,
                    libc::POLLHUP
                ),
            ]
        );
    }

    #[test]
    fn repeats_fresh_samples_and_rearm_is_silent() {
        let sink = NamedTempFile::new().expect("sink");
        let record_stack = NamedTempFile::new().expect("record stack");
        let store_writer_stack = NamedTempFile::new().expect("store writer stack");
        fs::write(record_stack.path(), "record_one\n").expect("record stack");
        fs::write(store_writer_stack.path(), "store_one\n").expect("store writer stack");
        let watchdog = start_test_watchdog(
            Duration::from_millis(100),
            sink.path(),
            record_stack.path(),
            store_writer_stack.path(),
        )
        .expect("start watchdog");

        assert!(wait_for(|| {
            fs::read_to_string(sink.path()).is_ok_and(|value| value.lines().count() >= 1)
        }));
        fs::write(record_stack.path(), "record_two\n").expect("replace record stack");
        fs::write(store_writer_stack.path(), "store_two\n").expect("replace writer stack");
        assert!(wait_for(|| {
            fs::read_to_string(sink.path()).is_ok_and(|value| {
                value.lines().any(|line| {
                    line.contains("record_stack=record_two")
                        && line.contains("store_writer_stack=store_two")
                })
            })
        }));

        for _ in 0..20 {
            watchdog.beat();
            thread::sleep(Duration::from_millis(10));
        }
        let before = fs::read_to_string(sink.path()).expect("reports");
        for _ in 0..20 {
            watchdog.beat();
            thread::sleep(Duration::from_millis(10));
        }
        let after = fs::read_to_string(sink.path()).expect("reports");
        assert_eq!(after.lines().count(), before.lines().count());

        let first_stall = after.lines().collect::<Vec<_>>();
        assert!(first_stall.len() >= 2);
        first_stall.iter().for_each(|line| {
            assert_schema(line);
            assert_eq!(
                field(line, "heartbeat_mono_ms"),
                field(first_stall[0], "heartbeat_mono_ms")
            );
            assert_eq!(field(line, "timeout_ms"), Some("100"));
        });
        assert!(first_stall[0].contains("record_stack=record_one"));
        assert!(first_stall[0].contains("store_writer_stack=store_one"));
        let last = first_stall.last().expect("last report");
        assert!(last.contains("record_stack=record_two"));
        assert!(last.contains("store_writer_stack=store_two"));
        assert!(
            field(last, "heartbeat_age_ms")
                .expect("last heartbeat age")
                .parse::<u64>()
                .expect("numeric last heartbeat age")
                > field(first_stall[0], "heartbeat_age_ms")
                    .expect("first heartbeat age")
                    .parse::<u64>()
                    .expect("numeric first heartbeat age")
        );

        let first_heartbeat = field(first_stall[0], "heartbeat_mono_ms")
            .expect("first heartbeat")
            .to_owned();
        assert!(wait_for(|| {
            fs::read_to_string(sink.path()).is_ok_and(|value| {
                value.lines().any(|line| {
                    field(line, "heartbeat_mono_ms")
                        .is_some_and(|heartbeat| heartbeat != first_heartbeat)
                })
            })
        }));
        let reports = fs::read_to_string(sink.path()).expect("reports");
        let next = reports.lines().last().expect("next stall report");
        assert_ne!(
            field(next, "heartbeat_mono_ms"),
            Some(first_heartbeat.as_str())
        );
    }

    #[test]
    fn heartbeat_racing_capture_discards_the_sample() {
        let sink = NamedTempFile::new().expect("sink");
        let dir = TempDir::new().expect("temp dir");
        let fifo = make_fifo(&dir);
        let store_writer_stack = NamedTempFile::new().expect("store writer stack");
        fs::write(store_writer_stack.path(), "store_writer_frame\n").expect("writer stack");
        let watchdog = start_test_watchdog(
            Duration::from_secs(1),
            sink.path(),
            &fifo,
            store_writer_stack.path(),
        )
        .expect("start watchdog");
        watchdog.beat();

        let mut writer = open_fifo_writer(&fifo);
        let completed = watchdog
            .shared
            .completed_expirations
            .load(Ordering::Acquire);
        watchdog.beat();
        writer.write_all(b"stale_frame\n").expect("release reader");
        drop(writer);
        assert!(wait_for(|| {
            watchdog
                .shared
                .completed_expirations
                .load(Ordering::Acquire)
                > completed
        }));
        assert_eq!(fs::read(sink.path()).expect("sink"), b"");
    }

    #[test]
    fn drop_joins_a_blocked_capture() {
        let sink = NamedTempFile::new().expect("sink");
        let dir = TempDir::new().expect("temp dir");
        let fifo = make_fifo(&dir);
        let store_writer_stack = NamedTempFile::new().expect("store writer stack");
        fs::write(store_writer_stack.path(), "store_writer_frame\n").expect("writer stack");
        let watchdog = start_test_watchdog(
            Duration::from_millis(20),
            sink.path(),
            &fifo,
            store_writer_stack.path(),
        )
        .expect("start watchdog");
        watchdog.beat();

        let mut writer = open_fifo_writer(&fifo);
        let (dropped_tx, dropped_rx) = channel();
        let dropper = thread::spawn(move || {
            drop(watchdog);
            dropped_tx.send(()).expect("signal drop");
        });
        assert_eq!(
            dropped_rx.recv_timeout(Duration::from_millis(100)),
            Err(RecvTimeoutError::Timeout)
        );
        writer.write_all(b"frame\n").expect("release reader");
        drop(writer);
        dropped_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("joined drop");
        dropper.join().expect("dropper");
    }

    #[test]
    fn fresh_deadline_does_not_report() {
        let sink = NamedTempFile::new().expect("sink");
        let record_stack = NamedTempFile::new().expect("record stack");
        let store_writer_stack = NamedTempFile::new().expect("store writer stack");
        let watchdog = start_test_watchdog(
            Duration::from_secs(1),
            sink.path(),
            record_stack.path(),
            store_writer_stack.path(),
        )
        .expect("start watchdog");
        watchdog.beat();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(fs::read(sink.path()).expect("sink"), b"");
    }

    #[test]
    fn validates_startup_and_cli_defaults() {
        let sink = NamedTempFile::new().expect("sink");
        assert!(
            Watchdog::start_with_options(StartOptions {
                timeout: Duration::ZERO,
                kmsg_path: sink.path(),
                record_stack_path: None,
                store_writer_stack_path: None,
                store_writer_tid: current_tid(),
            })
            .is_err()
        );
        assert!(
            Watchdog::start_with_options(StartOptions {
                timeout: Duration::from_secs(1),
                kmsg_path: sink.path(),
                record_stack_path: None,
                store_writer_stack_path: None,
                store_writer_tid: 0,
            })
            .is_err()
        );

        let opts = Opt::try_parse_from(["below", "record", "--watchdog-timeout-s", "30"])
            .expect("valid timeout");
        let Some(Command::Record {
            watchdog_timeout_s, ..
        }) = opts.cmd
        else {
            panic!("record command");
        };
        assert_eq!(watchdog_timeout_s.map(|value| value.get()), Some(30));
        assert!(Opt::try_parse_from(["below", "record", "--watchdog-timeout-s", "0"]).is_err());
        assert!(Opt::try_parse_from(["below", "record", "--watchdog-timeout-s", "86401"]).is_err());

        let opts = Opt::try_parse_from(["below", "record"]).expect("defaults");
        let Some(Command::Record {
            watchdog_timeout_s,
            writer_buffer_size,
            ..
        }) = opts.cmd
        else {
            panic!("record command");
        };
        assert_eq!(watchdog_timeout_s, None);
        assert_eq!(writer_buffer_size, 10);

        let five_seconds = Duration::from_secs(5);
        assert!(crate::validate_watchdog_timeout(five_seconds, None).is_ok());
        assert!(
            crate::validate_watchdog_timeout(five_seconds, Some(Duration::from_secs(10))).is_ok()
        );
        assert!(crate::validate_watchdog_timeout(five_seconds, Some(five_seconds)).is_err());
    }
}
