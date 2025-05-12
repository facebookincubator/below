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

use std::cell::RefCell;
use std::cell::RefMut;
use std::io;
use std::io::Read;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// This file contains various helpers
use chrono::prelude::*;

const BELOW_RC: &str = "/.config/below/belowrc";

/// Execute an expression every n times. For example
/// `every_n!(1 + 2, println!("I'm mod 3")` will print on the 1st,
/// 4th, and so on calls.
#[macro_export]
macro_rules! every_n {
    ($n:expr_2021, $ex:expr_2021) => {{
        static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let p = COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if p % ($n) == 0 {
            $ex
        }
    }};
}
pub use every_n;

/// Convert `timestamp` from `SystemTime` to seconds since epoch
pub fn get_unix_timestamp(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs()
}

/// Convert `timestamp` from seconds since epoch to `SystemTime`
pub fn get_system_time(timestamp: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(timestamp)
}

fn convert(val: f64, base: f64, units: &[&'static str]) -> String {
    if val < 1_f64 {
        return format!("{:.1} {}", val, units[0]);
    }
    let exponent = std::cmp::min(
        (val.ln() / base.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_val = format!("{:.1}", val / base.powi(exponent))
        .parse::<f64>()
        .unwrap()
        * 1_f64;
    let unit = units[exponent as usize];
    format!("{} {}", pretty_val, unit)
}

/// Convert `val` bytes into a human friendly string
pub fn convert_bytes(val: f64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    convert(val, 1024_f64, UNITS)
}

/// Convert `val` Hz into a human friendly string
pub fn convert_freq(val: u64) -> String {
    const UNITS: &[&str] = &["Hz", "kHz", "MHz", "GHz", "THz", "PHz", "EHz", "ZHz", "YHz"];
    let val_f64 = val as f64;
    convert(val_f64, 1000_f64, UNITS)
}

/// Convert `val` microseconds into a human friendly string. Largest unit used is seconds.
pub fn convert_duration(val: u64) -> String {
    const UNITS: &[&str] = &["us", "ms", "s"];
    convert(val as f64, 1000_f64, UNITS)
}

pub fn get_prefix(collapsed: bool) -> &'static str {
    if collapsed { "└+ " } else { "└─ " }
}

/// Fold a long string to a fixed size string and keep the front and back
///
/// The effective length of the string will be `width - 3` since this fn will use
/// 3 '.'s to replace the omitted content. If the width is less than 3, it will
/// return the original string. This function will also take a stop_filter closure
/// as an indicator of where to split the string. Please note that, this function
/// will take the minimal value of (width - 3)/2 and first index that hit the stop_filter
/// as the front half cutting point.
///
/// # Arguments
///
/// * `val` -- The string that needs to be folded.
/// * `width` -- The final target string length.
/// * `start_idx` -- From which index should we start apply the stop_filter
/// * `stop_filter` -- The first half will be cut at the first index that returns false
///    after apply the filter if the index is less than (width - 3)/2
pub fn fold_string<F>(val: &str, width: usize, start_idx: usize, stop_filter: F) -> String
where
    F: FnMut(char) -> bool,
{
    let str_len = val.len();
    if start_idx >= str_len || val[start_idx..].len() <= width || width <= 3 {
        return val.into();
    }

    let first_symbo_pos = val[start_idx..].find(stop_filter).unwrap_or(str_len) + 1;
    let mid_str_len = (width - 3) >> 1;
    let front_len = std::cmp::min(first_symbo_pos, mid_str_len);
    let front_string = val[..front_len].to_string();
    let back_string = val[str_len - width + front_len + 3..].to_string();
    format!("{}...{}", front_string, back_string)
}

/// Convert system timestamp to human readable datetime.
pub fn timestamp_to_datetime(timestamp: &i64) -> String {
    let naive = NaiveDateTime::from_timestamp_opt(*timestamp, 0).unwrap();
    let datetime = naive.and_utc();
    datetime
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Convert system time to human readable datetime.
pub fn systemtime_to_datetime(system_time: SystemTime) -> String {
    timestamp_to_datetime(&(get_unix_timestamp(system_time) as i64))
}

pub fn is_cpu_significant(v: f64) -> Option<cursive::theme::BaseColor> {
    if v > 100.0 {
        Some(cursive::theme::BaseColor::Red)
    } else {
        None
    }
}

/// Get the belowrc filename.
pub fn get_belowrc_filename() -> String {
    format!(
        "{}{}",
        std::env::var("HOME").expect("Fail to obtain HOME env var"),
        BELOW_RC
    )
}

/// The dump section key for belowrc
pub fn get_belowrc_dump_section_key() -> &'static str {
    "dump"
}

/// The cmd section key for belowrc
pub fn get_belowrc_cmd_section_key() -> &'static str {
    "cmd"
}

/// The view section key for belowrc
pub fn get_belowrc_view_section_key() -> &'static str {
    "view"
}

pub fn read_kern_file_to_internal_buffer<R: Read>(
    buffer: &RefCell<Vec<u8>>,
    mut reader: R,
) -> io::Result<RefMut<'_, str>> {
    const BUFFER_CHUNK_SIZE: usize = 1 << 16;

    let mut buffer = buffer.borrow_mut();
    let mut total_read = 0;

    loop {
        let buf_len = buffer.len();
        if buf_len < total_read + BUFFER_CHUNK_SIZE {
            buffer.resize(buf_len + BUFFER_CHUNK_SIZE, 0);
        }

        match reader.read(&mut buffer[total_read..]) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;
                // n < BUFFER_CHUNK_SIZE does not indicate EOF because kernel
                // may partially fill the buffer to avoid breaking the line.
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }

    RefMut::filter_map(buffer, |vec| {
        std::str::from_utf8_mut(&mut vec[..total_read]).ok()
    })
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 data"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_every_n() {
        let mut v1 = Vec::new();
        let mut v2 = Vec::new();
        for i in 0..10 {
            every_n!(2, v1.push(i));
            every_n!(1 + 2, v2.push(i));
        }
        assert_eq!(v1, vec![0, 2, 4, 6, 8]);
        assert_eq!(v2, vec![0, 3, 6, 9]);
    }

    #[test]
    fn test_convert_bytes() {
        // TODO(T118356932): This should really be 0 B
        assert_eq!(convert_bytes(0.0), "0.0 B".to_owned());
        assert_eq!(convert_bytes(1_024.0), "1 KB".to_owned());
        assert_eq!(convert_bytes(1_023.0), "1023 B".to_owned());
        assert_eq!(convert_bytes(1_076.0), "1.1 KB".to_owned());
        assert_eq!(convert_bytes(10_239.0), "10 KB".to_owned());
        assert_eq!(convert_bytes(1024_f64.powi(2)), "1 MB".to_owned());
        // TODO(T118356932): This should really be 1 MB
        assert_eq!(convert_bytes(1024_f64.powi(2) - 1.0), "1024 KB".to_owned());
        // TODO(T118356932): This should really be 1 GB
        assert_eq!(convert_bytes(1024_f64.powi(3) - 1.0), "1024 MB".to_owned());
        assert_eq!(convert_bytes(1024_f64.powi(3)), "1 GB".to_owned());
        assert_eq!(convert_bytes(1024_f64.powi(4)), "1 TB".to_owned());
    }

    #[test]
    fn test_convert_freq() {
        // TODO(T118356932): This should really be 0 Hz
        assert_eq!(convert_freq(0), "0.0 Hz".to_owned());
        assert_eq!(convert_freq(1_000), "1 kHz".to_owned());
        assert_eq!(convert_freq(999), "999 Hz".to_owned());
        assert_eq!(convert_freq(1_050), "1.1 kHz".to_owned());
        assert_eq!(convert_freq(9_999), "10 kHz".to_owned());
        assert_eq!(convert_freq(1_000_000), "1 MHz".to_owned());
        // TODO(T118356932): This should really be 1 GHz
        assert_eq!(convert_freq(999_950_000), "1000 MHz".to_owned());
        assert_eq!(convert_freq(1_000_000_000), "1 GHz".to_owned());
        assert_eq!(convert_freq(1_000_000_000_000), "1 THz".to_owned());
    }
}
