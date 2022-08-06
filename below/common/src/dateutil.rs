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

/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! # dateutil
//!
//! See [`HgTime`] and [`HgTime::parse`] for main features.

use std::ops::Add;
use std::ops::Range;
use std::ops::Sub;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::time::SystemTime;

use chrono::prelude::*;
use chrono::Duration;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::TimeZone;
use regex::Regex;

/// A simple time structure that matches hg's time representation.
///
/// Internally it's unixtime (in GMT), and offset (GMT -1 = +3600).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgTime {
    pub unixtime: u64,
    pub offset: i32,
}

const DEFAULT_FORMATS: [&str; 44] = [
    // mercurial/util.py defaultdateformats
    "%Y-%m-%dT%H:%M:%S", // the 'real' ISO8601
    "%Y-%m-%dT%H:%M",    //   without seconds
    "%Y-%m-%dT%H%M%S",   // another awful but legal variant without :
    "%Y-%m-%dT%H%M",     //   without seconds
    "%Y-%m-%d %H:%M:%S", // our common legal variant
    "%Y-%m-%d %H:%M",    //   without seconds
    "%Y-%m-%d %H%M%S",   // without :
    "%Y-%m-%d %H%M",     //   without seconds
    "%Y-%m-%d %I:%M:%S%p",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%d %I:%M%p",
    "%a %b %d %H:%M:%S %Y",
    "%a %b %d %I:%M:%S%p %Y",
    "%a, %d %b %Y %H:%M:%S", //  GNU coreutils "/bin/date --rfc-2822"
    "%b %d %H:%M:%S %Y",
    "%b %d %I:%M:%S%p %Y",
    "%b %d %H:%M:%S",
    "%b %d %I:%M:%S%p",
    "%b %d %H:%M",
    "%b %d %I:%M%p",
    "%m-%d",
    "%m/%d",
    "%Y-%m-%d",
    "%m/%d/%y",
    "%m/%d/%Y",
    "%b",
    "%b %d",
    "%b %Y",
    "%b %d %Y",
    "%I:%M%p",
    "%H:%M",
    "%H:%M:%S",
    "%I:%M:%S%p",
    "%Y",
    "%Y-%m",
    "%m/%d/%Y %I:%M:%S %P",
    "%m/%d/%Y %I:%M:%S%p",
    "%m/%d/%Y %H:%M:%S",
    "%m/%d/%Y %I:%M%p",
    "%m/%d/%Y %H:%M",
    "%m/%d %I:%M:%S%p",
    "%m/%d %H:%M:%S",
    "%m/%d %I:%M%p",
    "%m/%d %H:%M",
];

const TIME_OF_DAY_FORMATS: [&str; 6] = [
    "%I:%M%p",
    "%I:%M%P",
    "%I:%M:%S%p",
    "%I:%M:%S%P",
    "%H:%M",
    "%H:%M:%S",
];

const INVALID_OFFSET: i32 = i32::max_value();
static DEFAUL_OFFSET: AtomicI32 = AtomicI32::new(INVALID_OFFSET);

impl HgTime {
    pub fn now() -> Self {
        let now: HgTime = Local::now().into();
        now.use_default_offset()
    }

    /// Parse a date string.
    ///
    /// Return `None` if it cannot be parsed.
    ///
    /// This function matches `mercurial.util.parsedate`, and can parse
    /// some additional forms like `2 days ago`.
    /// It can also handle future forms like "ten hours from now" and "+10h"
    pub fn parse(date: &str) -> Option<Self> {
        match date {
            "now" => Some(Self::now()),
            "today" => Some(Self::from(Local::today().and_hms(0, 0, 0)).use_default_offset()),
            "yesterday" => Some(
                Self::from(Local::today().and_hms(0, 0, 0) - Duration::days(1))
                    .use_default_offset(),
            ),
            "tomorrow" => Some(
                Self::from(Local::today().and_hms(0, 0, 0) + Duration::days(1))
                    .use_default_offset(),
            ),
            "day after tomorrow" | "the day after tomorrow" | "overmorrow" => Some(
                Self::from(Local::today().and_hms(0, 0, 0) + Duration::days(2))
                    .use_default_offset(),
            ),
            // Match all string ends with [dhms] or ago but not ends with pm and am. Case insensitive.
            // We have to use two regex here is because regex crate doesn't support negative lookbehind
            // expression.
            date if match (
                Regex::new(r"(?i)\+?.*([dhms]|ago|from now)$"),
                Regex::new(r"(?i)(pm|am)$"),
            ) {
                (Ok(relative_re), Ok(ampm_re)) => {
                    relative_re.is_match(&date) && !ampm_re.is_match(&date)
                }
                _ => false,
            } =>
            {
                let date = date.to_ascii_lowercase();
                let plus = date.starts_with("+") && !date.ends_with("ago");
                let from_now = date.ends_with("from now");
                // Trim past/future markers down to just date
                let mut duration_str = if plus { &date[1..] } else { &date };
                duration_str = if date.ends_with("ago") {
                    &duration_str[..duration_str.len() - 4]
                } else {
                    duration_str
                };
                duration_str = if from_now {
                    &duration_str[..duration_str.len() - 9]
                } else {
                    duration_str
                };

                if plus || from_now {
                    duration_str
                        .parse::<humantime::Duration>()
                        .ok()
                        .map(|duration| Self::now() + duration.as_secs())
                } else {
                    duration_str
                        .parse::<humantime::Duration>()
                        .ok()
                        .map(|duration| Self::now() - duration.as_secs())
                }
            }
            date if match Regex::new(r"^\d{10}$") {
                Ok(systime_re) => systime_re.is_match(&date),
                _ => false,
            } =>
            {
                match date.parse::<u64>() {
                    Ok(d) => Some(
                        Self {
                            unixtime: d,
                            offset: 0,
                        }
                        .use_default_offset(),
                    ),
                    _ => None,
                }
            }
            _ => Self::parse_absolute(date, default_date_lower),
        }
    }

    /// Parse a date string as a range.
    ///
    /// For example, `Apr 2000` covers range `Apr 1, 2000` to `Apr 30, 2000`.
    /// Also support more explicit ranges:
    /// - START to END
    /// - > START
    /// - < END
    #[allow(dead_code)]
    pub fn parse_range(date: &str) -> Option<Range<Self>> {
        match date {
            "now" => {
                let now = Self::now();
                Some(now..now + 1)
            }
            "today" => {
                let date = Local::today();
                let start = Self::from(date.and_hms(0, 0, 0)).use_default_offset();
                let end = Self::from(date.and_hms(23, 59, 59)).use_default_offset() + 1;
                Some(start..end)
            }
            "yesterday" => {
                let date = Local::today() - Duration::days(1);
                let start = Self::from(date.and_hms(0, 0, 0)).use_default_offset();
                let end = Self::from(date.and_hms(23, 59, 59)).use_default_offset() + 1;
                Some(start..end)
            }
            "tomorrow" => {
                let date = Local::today() + Duration::days(1);
                let start = Self::from(date.and_hms(0, 0, 0)).use_default_offset();
                let end = Self::from(date.and_hms(23, 59, 59)).use_default_offset() + 1;
                Some(start..end)
            }
            "day after tomorrow" | "the day after tomorrow" | "overmorrow" => {
                let date = Local::today() + Duration::days(2);
                let start = Self::from(date.and_hms(0, 0, 0)).use_default_offset();
                let end = Self::from(date.and_hms(23, 59, 59)).use_default_offset() + 1;
                Some(start..end)
            }
            date if date.starts_with(">") => {
                Self::parse(&date[1..]).map(|start| start..Self::max_value())
            }
            date if date.starts_with("since ") => {
                Self::parse(&date[6..]).map(|start| start..Self::max_value())
            }
            date if date.starts_with("<") => {
                Self::parse(&date[1..]).map(|end| Self::min_value()..end)
            }
            date if date.starts_with("-") => {
                // This does not really make much sense. But is supported by hg
                // (see 'hg help dates').
                Self::parse_range(&format!("since {} days ago", &date[1..]))
            }
            date if date.starts_with("before ") => {
                Self::parse(&date[7..]).map(|end| Self::min_value()..end)
            }
            date if date.contains(" to ") => {
                let phrases: Vec<_> = date.split(" to ").collect();
                if phrases.len() == 2 {
                    if let (Some(start), Some(end)) =
                        (Self::parse(&phrases[0]), Self::parse(&phrases[1]))
                    {
                        Some(start..end)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => {
                let start = Self::parse_absolute(date, default_date_lower);
                let end = Self::parse_absolute(date, default_date_upper::<N31>)
                    .or_else(|| Self::parse_absolute(date, default_date_upper::<N30>))
                    .or_else(|| Self::parse_absolute(date, default_date_upper::<N29>))
                    .or_else(|| Self::parse_absolute(date, default_date_upper::<N28>));
                if let (Some(start), Some(end)) = (start, end) {
                    Some(start..end + 1)
                } else {
                    None
                }
            }
        }
    }

    /// Parse date in an absolute form.
    ///
    /// Return None if it cannot be parsed.
    ///
    /// `default_date` takes a format char, for example, `H`, and returns a
    /// default value of it.
    fn parse_absolute(date: &str, default_date: fn(char) -> &'static str) -> Option<Self> {
        let date = date.trim();

        // Hg internal format. "unixtime offset"
        let parts: Vec<_> = date.split(" ").collect();
        if parts.len() == 2 {
            if let Ok(unixtime) = parts[0].parse() {
                if let Ok(offset) = parts[1].parse() {
                    if is_valid_offset(offset) {
                        return Some(Self { unixtime, offset });
                    }
                }
            }
        }

        // Normalize UTC timezone name to +0000. The parser does not know
        // timezone names.
        let date = if date.ends_with("GMT") || date.ends_with("UTC") {
            format!("{} +0000", &date[..date.len() - 3])
        } else {
            date.to_string()
        };
        let mut now = None; // cached, lazily calculated "now"

        // Try all formats!
        for naive_format in DEFAULT_FORMATS.iter() {
            // Fill out default fields.  See mercurial.util.strdate.
            // This makes it possible to parse partial dates like "month/day",
            // or "hour:minute", since the missing fields will be filled.
            let mut default_format = String::new();
            let mut date_with_defaults = date.clone();
            let mut use_now = false;
            for part in ["S", "M", "HI", "d", "mb", "Yy"] {
                if part
                    .chars()
                    .any(|ch| naive_format.contains(&format!("%{}", ch)))
                {
                    // For example, if the user specified "d" (day), but
                    // not other things, we should use 0 for "H:M:S", and
                    // "now" for "Y-m" (year, month).
                    use_now = true;
                } else {
                    let format_char = part.chars().nth(0).unwrap();
                    default_format += &format!(" @%{}", format_char);
                    if use_now {
                        // For example, if the user only specified "month/day",
                        // then we should use the current "year", instead of
                        // year 0.
                        let now = now.get_or_insert_with(|| Local::now());
                        date_with_defaults +=
                            &format!(" @{}", now.format(&format!("%{}", format_char)));
                    } else {
                        // For example, if the user only specified
                        // "hour:minute", then we should use "second 0", instead
                        // of the current second.
                        date_with_defaults += " @";
                        date_with_defaults += default_date(format_char);
                    }
                }
            }

            // Try parse with timezone.
            // See https://docs.rs/chrono/0.4.9/chrono/format/strftime/index.html#specifiers
            let format = format!("{}%#z{}", naive_format, default_format);
            if let Ok(parsed) = DateTime::parse_from_str(&date_with_defaults, &format) {
                return Some(parsed.into());
            }

            // Without timezone.
            let format = format!("{}{}", naive_format, default_format);
            if let Ok(parsed) = NaiveDateTime::parse_from_str(&date_with_defaults, &format) {
                return Some(parsed.into());
            }
        }

        None
    }

    pub fn parse_time_of_day(date: &str) -> Option<NaiveTime> {
        for naive_format in TIME_OF_DAY_FORMATS.iter() {
            let format = naive_format.to_string();
            if let Ok(parsed) = NaiveTime::parse_from_str(date, &format) {
                return Some(parsed);
            }
        }

        None
    }

    fn extract_local_date_from_system_time(system_time: SystemTime) -> Option<NaiveDate> {
        if let Ok(system_time_as_duration) = system_time.duration_since(SystemTime::UNIX_EPOCH) {
            return Some(
                Local
                    .timestamp(system_time_as_duration.as_secs() as i64, 0)
                    .date()
                    .naive_local(),
            );
        }

        None
    }

    pub fn time_of_day_relative_to_system_time(
        system_time: SystemTime,
        time_of_day: NaiveTime,
    ) -> Option<SystemTime> {
        if let Some(date_from_system_time) = Self::extract_local_date_from_system_time(system_time)
        {
            let naive_date_with_time_of_day =
                NaiveDateTime::new(date_from_system_time, time_of_day);
            if let Some(local_time) = Local
                .from_local_datetime(&naive_date_with_time_of_day)
                .single()
            {
                let local_system_time = SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(local_time.timestamp() as u64);
                return Some(local_system_time);
            }
        }

        None
    }

    /// Change "offset" to DEFAUL_OFFSET. Useful for tests so they won't be
    /// affected by local timezone.
    fn use_default_offset(mut self) -> Self {
        let offset = DEFAUL_OFFSET.load(Ordering::SeqCst);
        if is_valid_offset(offset) {
            self.offset = offset
        }
        self
    }

    pub fn min_value() -> Self {
        Self {
            unixtime: 0,
            offset: 0,
        }
    }

    pub fn max_value() -> Self {
        Self {
            unixtime: u64::max_value() >> 2,
            offset: 0,
        }
    }
}

impl Add<u64> for HgTime {
    type Output = Self;

    fn add(self, seconds: u64) -> Self {
        Self {
            unixtime: self.unixtime + seconds,
            offset: self.offset,
        }
    }
}

impl Sub<u64> for HgTime {
    type Output = Self;

    fn sub(self, seconds: u64) -> Self {
        Self {
            // XXX: This might silently change negative time to 0.
            unixtime: self.unixtime.max(seconds) - seconds,
            offset: self.offset,
        }
    }
}

impl PartialOrd for HgTime {
    fn partial_cmp(&self, other: &HgTime) -> Option<std::cmp::Ordering> {
        self.unixtime.partial_cmp(&other.unixtime)
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for HgTime {
    fn from(time: DateTime<Tz>) -> Self {
        assert!(time.timestamp() >= 0);
        Self {
            unixtime: time.timestamp() as u64,
            offset: time.offset().fix().utc_minus_local(),
        }
    }
}

impl From<NaiveDateTime> for HgTime {
    fn from(time: NaiveDateTime) -> Self {
        let timestamp = time.timestamp();
        // Use local offset. (Is there a better way to do this?)
        let offset = Self::now().offset;
        // XXX: This might silently change negative time to 0.
        let unixtime = (timestamp + offset as i64).max(0) as u64;
        Self { unixtime, offset }
    }
}

/// Change default offset (timezone).
#[allow(dead_code)]
pub fn set_default_offset(offset: i32) {
    DEFAUL_OFFSET.store(offset, Ordering::SeqCst);
}

fn is_valid_offset(offset: i32) -> bool {
    // UTC-12 to UTC+14.
    offset >= -50400 && offset <= 43200
}

/// Lower bound for default values in dates.
fn default_date_lower(format_char: char) -> &'static str {
    match format_char {
        'H' | 'M' | 'S' => "00",
        'm' | 'd' => "1",
        _ => unreachable!(),
    }
}

trait ToStaticStr {
    fn to_static_str() -> &'static str;
}

struct N31;
struct N30;
struct N29;
struct N28;

impl ToStaticStr for N31 {
    fn to_static_str() -> &'static str {
        "31"
    }
}

impl ToStaticStr for N30 {
    fn to_static_str() -> &'static str {
        "30"
    }
}

impl ToStaticStr for N29 {
    fn to_static_str() -> &'static str {
        "29"
    }
}

impl ToStaticStr for N28 {
    fn to_static_str() -> &'static str {
        "28"
    }
}

/// Upper bound. Assume a month has `N::to_static_str()` days.
fn default_date_upper<N: ToStaticStr>(format_char: char) -> &'static str {
    match format_char {
        'H' => "23",
        'M' | 'S' => "59",
        'm' => "12",
        'd' => N::to_static_str(),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        // Test cases are mostly from test-parse-date.t.
        // Some variants were added.
        set_default_offset(7200);

        // t: parse date
        // d: parse date, compare with now, within expected range
        // The right side of assert_eq! is a string so it's autofix-able.

        assert_eq!(t("2006-02-01 13:00:30"), "1138806030 7200");
        assert_eq!(t("2006-02-01 13:00:30-0500"), "1138816830 18000");
        assert_eq!(t("2006-02-01 13:00:30 +05:00"), "1138780830 -18000");
        assert_eq!(t("2006-02-01 13:00:30Z"), "1138798830 0");
        assert_eq!(t("2006-02-01 13:00:30 GMT"), "1138798830 0");
        assert_eq!(t("2006-4-5 13:30"), "1144251000 7200");
        assert_eq!(t("1150000000 14400"), "1150000000 14400");
        assert_eq!(t("100000 1400000"), "fail");
        assert_eq!(t("1000000000 -16200"), "1000000000 -16200");
        assert_eq!(t("2006-02-01 1:00:30PM +0000"), "1138798830 0");

        assert_eq!(d("1:00:30PM +0000", Duration::days(1)), "0");
        assert_eq!(d("02/01", Duration::weeks(52)), "0");
        assert_eq!(d("today", Duration::days(1)), "0");
        assert_eq!(d("yesterday", Duration::days(2)), "0");
        assert_eq!(d("tomorrow", Duration::days(1)), "0");
        assert_eq!(d("day after tomorrow", Duration::days(2)), "0");
        assert_eq!(d("overmorrow", Duration::days(2)), "0");

        // ISO8601
        assert_eq!(t("2016-07-27T12:10:21"), "1469628621 7200");
        assert_eq!(t("2016-07-27T12:10:21Z"), "1469621421 0");
        assert_eq!(t("2016-07-27T12:10:21+00:00"), "1469621421 0");
        assert_eq!(t("2016-07-27T121021Z"), "1469621421 0");
        assert_eq!(t("2016-07-27 12:10:21"), "1469628621 7200");
        assert_eq!(t("2016-07-27 12:10:21Z"), "1469621421 0");
        assert_eq!(t("2016-07-27 12:10:21+00:00"), "1469621421 0");
        assert_eq!(t("2016-07-27 121021Z"), "1469621421 0");

        // Months
        assert_eq!(t("Jan 2018"), "1514772000 7200");
        assert_eq!(t("Feb 2018"), "1517450400 7200");
        assert_eq!(t("Mar 2018"), "1519869600 7200");
        assert_eq!(t("Apr 2018"), "1522548000 7200");
        assert_eq!(t("May 2018"), "1525140000 7200");
        assert_eq!(t("Jun 2018"), "1527818400 7200");
        assert_eq!(t("Jul 2018"), "1530410400 7200");
        assert_eq!(t("Sep 2018"), "1535767200 7200");
        assert_eq!(t("Oct 2018"), "1538359200 7200");
        assert_eq!(t("Nov 2018"), "1541037600 7200");
        assert_eq!(t("Dec 2018"), "1543629600 7200");
        assert_eq!(t("Foo 2018"), "fail");

        // Extra tests not in test-parse-date.t
        assert_eq!(d("Jan", Duration::weeks(52)), "0");
        assert_eq!(d("Jan 1", Duration::weeks(52)), "0"); // 1 is not considered as "year 1"
        assert_eq!(d("4-26", Duration::weeks(52)), "0");
        assert_eq!(d("4/26", Duration::weeks(52)), "0");
        assert_eq!(t("4/26/2000"), "956714400 7200");
        assert_eq!(t("Apr 26 2000"), "956714400 7200");
        assert_eq!(t("2020"), "1577844000 7200"); // 2020 is considered as a "year"
        assert_eq!(t("2020 GMT"), "1577836800 0");
        assert_eq!(t("2020-12"), "1606788000 7200");
        assert_eq!(t("2020-13"), "fail");

        assert_eq!(t("Fri, 20 Sep 2019 12:15:13 -0700"), "1569006913 25200"); // date --rfc-2822
        assert_eq!(t("Fri, 20 Sep 2019 12:15:13"), "1568988913 7200");

        assert_eq!(t("09/20/2019 12:15:13"), "1568988913 7200");
        assert_eq!(t("09/20/2019 12:15"), "1568988900 7200");
        assert_eq!(t("09/20/2019 12:15:13PM"), "1568988913 7200");
        assert_eq!(t("09/20/2019 12:15PM"), "1568988900 7200");
        assert_eq!(t("09/20 12:15:13"), t("Sep 20 12:15:13"));
        assert_eq!(t("09/20 12:15"), t("Sep 20 12:15:00"));
        assert_eq!(t("09/20 12:15:13PM"), t("Sep 20 12:15:13"));
        assert_eq!(t("09/20 12:15PM"), t("Sep 20 12:15:00"));
    }

    #[test]
    fn test_parse_ago() {
        set_default_offset(7200);
        // I believe the comparisons are rounded up to the next largest duration size
        // to absorb differences in execution time between test runs, otherwise this
        // doesn't really make sense.
        assert_eq!(d("10m ago", Duration::hours(1)), "0");
        assert_eq!(d("10 min ago", Duration::hours(1)), "0");
        assert_eq!(d("10 minutes ago", Duration::hours(1)), "0");
        assert_eq!(d("10 hours ago", Duration::days(1)), "0");
        assert_eq!(d("10 h ago", Duration::days(1)), "0");
        assert_eq!(t("9999999 years ago"), "0 7200");
    }

    #[test]
    fn test_parse_from_now() {
        set_default_offset(7200);
        // We'll keep the same duration comparisons for this set of tests for
        // consistency.  Maybe we should instead round to the next minute to
        // decrease the accepted timeframe and increase test accuracy?
        assert_eq!(d("10m from now", Duration::hours(1)), "0");
        assert_eq!(d("10 min from now", Duration::hours(1)), "0");
        assert_eq!(d("10 minutes from now", Duration::hours(1)), "0");
        assert_eq!(d("10 hours from now", Duration::days(1)), "0");
        assert_eq!(d("10 h from now", Duration::days(1)), "0");
    }

    #[test]
    fn test_parse_ago_short() {
        set_default_offset(7200);
        assert!(diff_from_now("10m", Duration::minutes(10)) < 2);
        assert!(diff_from_now("2d", Duration::days(2)) < 2);
        assert!(diff_from_now("10s", Duration::seconds(10)) < 2);
        assert!(diff_from_now("10h", Duration::hours(10)) < 2);
        assert!(diff_from_now("10h10m5s", Duration::seconds(36_605)) < 2);
        assert!(diff_from_now("10h5s10m", Duration::seconds(36_605)) < 2);
        assert!(diff_from_now("10H5s10M", Duration::seconds(36_605)) < 2);
        // wrong format
        assert_eq!(diff_from_now("10AM", Duration::minutes(10)), -1);
        assert_eq!(diff_from_now("10hm", Duration::minutes(10)), -1);
    }

    #[test]
    fn test_parse_from_now_short() {
        set_default_offset(7200);
        assert!(future_diff_from_now("+10m", Duration::minutes(10)) < 2);
        assert!(future_diff_from_now("+2d", Duration::days(2)) < 2);
        assert!(future_diff_from_now("+10s", Duration::seconds(10)) < 2);
        assert!(future_diff_from_now("+10h", Duration::hours(10)) < 2);
        assert!(future_diff_from_now("+10h10m5s", Duration::seconds(36_605)) < 2);
        assert!(future_diff_from_now("+10h5s10m", Duration::seconds(36_605)) < 2);
        assert!(future_diff_from_now("+10H5s10M", Duration::seconds(36_605)) < 2);
        // wrong format
        assert_eq!(future_diff_from_now("+10AM", Duration::minutes(10)), -1);
        assert_eq!(future_diff_from_now("+10hm", Duration::minutes(10)), -1);
    }

    #[test]
    fn test_parse_range() {
        set_default_offset(7200);

        assert_eq!(c("today", "tomorrow"), "does not contain");
        assert_eq!(c("tomorrow", "overmorrow"), "does not contain");
        assert_eq!(c("the day after tomorrow", "overmorrow"), "contains");
        assert_eq!(c("overmorrow", "1 days from now"), "does not contain");
        assert_eq!(c("overmorrow", "2 days from now"), "contains");
        assert_eq!(c("overmorrow", "3 days from now"), "does not contain");
        assert_eq!(c("since 1 month ago", "now"), "contains");
        assert_eq!(c("since 1 month ago", "2 months ago"), "does not contain");
        assert_eq!(c("> 1 month ago", "2 months ago"), "does not contain");
        assert_eq!(c("< 1 month ago", "2 months ago"), "contains");
        assert_eq!(c("< 1 month ago", "now"), "does not contain");

        assert_eq!(c("-3", "now"), "contains");
        assert_eq!(c("-3", "2 days ago"), "contains");
        assert_eq!(c("-3", "4 days ago"), "does not contain");

        assert_eq!(c("2018", "2017-12-31 23:59:59"), "does not contain");
        assert_eq!(c("2018", "2018-1-1"), "contains");
        assert_eq!(c("2018", "2018-12-31 23:59:59"), "contains");
        assert_eq!(c("2018", "2019-1-1"), "does not contain");

        assert_eq!(c("2018-5-1 to 2018-6-2", "2018-4-30"), "does not contain");
        assert_eq!(c("2018-5-1 to 2018-6-2", "2018-5-30"), "contains");
        assert_eq!(c("2018-5-1 to 2018-6-2", "2018-6-30"), "does not contain");
    }

    /// String representation of parse result.
    fn t(date: &str) -> String {
        match HgTime::parse(date) {
            Some(time) => format!("{} {}", time.unixtime, time.offset),
            None => "fail".to_string(),
        }
    }

    /// String representation of (parse result - now) / seconds.
    fn d(date: &str, duration: Duration) -> String {
        match HgTime::parse(date) {
            Some(time) => {
                let value = (time.unixtime as i64 - HgTime::now().unixtime as i64).abs()
                    / duration.num_seconds();
                format!("{}", value)
            }
            None => "fail".to_string(),
        }
    }

    /// String "contains" (if range contains date) or "does not contain"
    /// or "fail" (if either range or date fails to parse).
    fn c(range: &str, date: &str) -> &'static str {
        if let (Some(range), Some(date)) = (HgTime::parse_range(range), HgTime::parse(date)) {
            if range.contains(&date) {
                "contains"
            } else {
                "does not contain"
            }
        } else {
            "fail"
        }
    }

    /// Comparing the date before now.
    /// Return diff |now - date - expected|
    /// Return negative value for parse error.
    fn diff_from_now(date: &str, expected: Duration) -> i64 {
        match HgTime::parse(date) {
            Some(time) => {
                let now = HgTime::now().unixtime as i64;
                let before = time.unixtime as i64;
                let expected_diff = expected.num_seconds();
                (now - before - expected_diff).abs()
            }
            None => -1,
        }
    }

    /// Comparing the date from now.
    /// Return diff |future - now - expected|
    /// Return negative value for parse error.
    fn future_diff_from_now(date: &str, expected: Duration) -> i64 {
        match HgTime::parse(date) {
            Some(time) => {
                let now = HgTime::now().unixtime as i64;
                let future = time.unixtime as i64;
                let expected_diff = expected.num_seconds();
                (future - now - expected_diff).abs()
            }
            None => -1,
        }
    }
}
