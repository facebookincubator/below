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

use anyhow::{anyhow, bail, Result};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::dateutil;

/// Convert from date to `SystemTime`
pub fn system_time_from_date(date: &str) -> Result<SystemTime> {
    Ok(UNIX_EPOCH
    + Duration::from_secs(
        dateutil::HgTime::parse(&date)
            .ok_or_else(|| {
                anyhow!(
                    "Unrecognized timestamp format\n\
                Input: {}.\n\
                Examples:\n\t\
                Keywords: now, today, yesterday\n\t\
                Relative: \"{{humantime}} ago\", e.g. 2 days 3 hr 15m 10sec ago\n\t\
                Relative short: Mixed {{time_digit}}{{time_unit_char}}. E.g. 10m, 3d2H, 5h30s, 10m5h\n\t\
                Absolute: \"Jan 01 23:59\", \"01/01/1970 11:59PM\", \"1970-01-01 23:59:59\"\n\t\
                Unix Epoch: 1589808367",
                    &date
                )
            })?
            .unixtime,
    ))
}

/// Convert from date and an optional days adjuster to `SystemTime`. Days
/// adjuster is of form y[y...] to `SystemTime`. Each "y" will deduct 1 day
/// from the resulting time.
pub fn system_time_from_date_and_adjuster(
    date: &str,
    days_adjuster: Option<&str>,
) -> Result<SystemTime> {
    let mut time = system_time_from_date(date)?;
    if let Some(days) = days_adjuster {
        if days.is_empty() || days.find(|c: char| c != 'y').is_some() {
            bail!("Unrecognized days adjuster format: {}", days);
        }
        let time_to_deduct = Duration::from_secs(days.chars().count() as u64 * 86400);
        time -= time_to_deduct;
    }
    Ok(time)
}

/// Convert from date range and an optional days adjuster to a start and end
/// `SystemTime`. Days adjuster is of form y[y...]. Each "y" will deduct 1 day
/// from the resulting time.
pub fn system_time_range_from_date_and_adjuster(
    start_date: &str,
    end_date: Option<&str>,
    days_adjuster: Option<&str>,
) -> Result<(SystemTime, SystemTime)> {
    let start = system_time_from_date_and_adjuster(start_date, days_adjuster)?;
    let end = match end_date {
        Some(t) => system_time_from_date_and_adjuster(t, days_adjuster)?,
        None => SystemTime::now(),
    };
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_time_from_date_fail() {
        match system_time_from_date("invalid") {
            Ok(_) => panic!("Expected to fail but didn't"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_system_time_from_date_and_adjuster() {
        assert_eq!(
            system_time_from_date_and_adjuster("2006-02-01 13:00:30", None).unwrap(),
            t("2006-02-01 13:00:30")
        );
        assert_eq!(
            system_time_from_date_and_adjuster("2006-02-01 13:00:30", Some("y")).unwrap(),
            t("2006-01-31 13:00:30")
        );
        assert_eq!(
            system_time_from_date_and_adjuster("2006-02-01 13:00:30", Some("yy")).unwrap(),
            t("2006-01-30 13:00:30")
        );
        assert_eq!(
            system_time_from_date_and_adjuster("2006-02-01 13:00:30", Some("yyy")).unwrap(),
            t("2006-01-29 13:00:30")
        );
    }

    #[test]
    fn test_system_time_from_date_and_adjuster_fail() {
        match system_time_from_date_and_adjuster("2006-02-01 13:00:30", Some("invalid")) {
            Ok(_) => panic!("Expected fo fail as adjuster is invalid"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_system_time_range_from_date_and_adjuster() {
        assert_eq!(
            system_time_range_from_date_and_adjuster(
                "2006-02-01 13:00:30",
                Some("2006-02-01 15:00:30"),
                Some("y")
            )
            .unwrap(),
            (t("2006-01-31 13:00:30"), t("2006-01-31 15:00:30"))
        )
    }

    /// Convert date to `SystemTime`
    fn t(h: &str) -> SystemTime {
        system_time_from_date(h).unwrap()
    }
}
