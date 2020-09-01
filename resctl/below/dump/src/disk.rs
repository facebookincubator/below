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

use super::*;

use model::SingleDiskModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct DiskData {
    #[bttr(dfill_struct = "Disk")]
    #[bttr(tag = "DiskField::Name")]
    #[blink("SingleDiskModel$get_name")]
    pub name: Option<String>,
    #[bttr(tag = "DiskField::TotalBytes")]
    #[blink("SingleDiskModel$get_disk_total_bytes_per_sec")]
    pub disk_total_bytes_per_sec: Option<f64>,
    #[bttr(tag = "DiskField::ReadBytes", class = "DiskField::Read")]
    #[blink("SingleDiskModel$get_read_bytes_per_sec")]
    pub read_bytes_per_sec: Option<f64>,
    #[bttr(tag = "DiskField::WriteBytes", class = "DiskField::Write")]
    #[blink("SingleDiskModel$get_write_bytes_per_sec")]
    pub write_bytes_per_sec: Option<f64>,
    #[bttr(tag = "DiskField::DiscardBytes", class = "DiskField::Discard")]
    #[blink("SingleDiskModel$get_discard_bytes_per_sec")]
    pub discard_bytes_per_sec: Option<f64>,
    #[bttr(tag = "DiskField::ReadComplated", class = "DiskField::Read")]
    #[blink("SingleDiskModel$get_read_completed")]
    pub read_completed: Option<u64>,
    #[bttr(tag = "DiskField::ReadMerged", class = "DiskField::Read")]
    #[blink("SingleDiskModel$get_read_merged")]
    pub read_merged: Option<u64>,
    #[bttr(tag = "DiskField::ReadSectors", class = "DiskField::Read")]
    #[blink("SingleDiskModel$get_read_sectors")]
    pub read_sectors: Option<u64>,
    #[bttr(tag = "DiskField::TimeSpendRead", class = "DiskField::Read")]
    #[blink("SingleDiskModel$get_time_spend_read_ms")]
    pub time_spend_read_ms: Option<u64>,
    #[bttr(tag = "DiskField::WriteCompleted", class = "DiskField::Write")]
    #[blink("SingleDiskModel$get_write_completed")]
    pub write_completed: Option<u64>,
    #[bttr(tag = "DiskField::WriteMerged", class = "DiskField::Write")]
    #[blink("SingleDiskModel$get_write_merged")]
    pub write_merged: Option<u64>,
    #[bttr(tag = "DiskField::WriteSectors", class = "DiskField::Write")]
    #[blink("SingleDiskModel$get_write_sectors")]
    pub write_sectors: Option<u64>,
    #[bttr(tag = "DiskField::TimeSpendWrite", class = "DiskField::Write")]
    #[blink("SingleDiskModel$get_time_spend_write_ms")]
    pub time_spend_write_ms: Option<u64>,
    #[bttr(tag = "DiskField::DiscardCompleted", class = "DiskField::Discard")]
    #[blink("SingleDiskModel$get_discard_completed")]
    pub discard_completed: Option<u64>,
    #[bttr(tag = "DiskField::DiscardMerged", class = "DiskField::Discard")]
    #[blink("SingleDiskModel$get_discard_merged")]
    pub discard_merged: Option<u64>,
    #[bttr(tag = "DiskField::DiscardSectors", class = "DiskField::Discard")]
    #[blink("SingleDiskModel$get_discard_sectors")]
    pub discard_sectors: Option<u64>,
    #[bttr(tag = "DiskField::TimeSpendDiscard", class = "DiskField::Discard")]
    #[blink("SingleDiskModel$get_time_spend_discard_ms")]
    pub time_spend_discard_ms: Option<u64>,
    #[bttr(tag = "DiskField::Major")]
    #[blink("SingleDiskModel$get_major")]
    pub major: Option<u64>,
    #[bttr(tag = "DiskField::Minor")]
    #[blink("SingleDiskModel$get_minor")]
    pub minor: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "DiskField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "DiskField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&DiskData, &SingleDiskModel) -> String>;
type FieldFtype = Box<dyn Fn(&DiskData, &SingleDiskModel) -> String>;

pub struct Disk {
    data: DiskData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    select: Option<DiskField>,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Disk {
    type Model = SingleDiskModel;
    type FieldsType = DiskField;
    type DataType = DiskData;
}

make_dget!(
    Disk,
    DiskField::Datetime,
    DiskField::Name,
    DiskField::TotalBytes,
    DiskField::Major,
    DiskField::Minor,
    DiskField::Read,
    DiskField::Write,
    DiskField::Discard,
    DiskField::Timestamp,
);

impl Dprint for Disk {}

impl Dump for Disk {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        select: Option<DiskField>,
    ) -> Self {
        Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
            select,
            title_fns: vec![],
            field_fns: vec![],
        }
    }

    fn advance_timestamp(&mut self, model: &model::Model) -> Result<()> {
        self.data.timestamp = match model.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(t) => t.as_secs() as i64,
            Err(e) => bail!("Fail to convert system time: {}", e),
        };
        self.data.datetime = self.data.timestamp;

        Ok(())
    }

    fn iterate_exec<T: Write>(
        &self,
        model: &model::Model,
        output: &mut T,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        let mut disks: Vec<&SingleDiskModel> = model
            .system
            .disks
            .iter()
            .filter_map(|(_, sdm)| match (self.select, self.opts.filter.as_ref()) {
                (Some(tag), Some(re)) => {
                    if self.filter_by(sdm, &tag, &re) {
                        Some(sdm)
                    } else {
                        None
                    }
                }
                _ => Some(sdm),
            })
            .collect();

        if let Some(tag) = self.select {
            if self.opts.sort {
                Self::sort_by(&mut disks, &tag, false);
            }

            if self.opts.rsort {
                Self::sort_by(&mut disks, &tag, true);
            }

            if (self.opts.sort || self.opts.rsort) && self.opts.top != 0 {
                disks.truncate(self.opts.top as usize);
            }
        }
        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        disks
            .iter()
            .map(|sdm| {
                let ret = match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => self.do_print_raw(&sdm, output, *round),
                    Some(OutputFormat::Csv) => self.do_print_csv(&sdm, output, *round),
                    Some(OutputFormat::KeyVal) => self.do_print_kv(&sdm, output),
                    Some(OutputFormat::Json) => {
                        let par = self.do_print_json(&sdm);
                        json_output.as_array_mut().unwrap().push(par);
                        Ok(())
                    }
                };
                *round += 1;
                ret
            })
            .collect::<Result<Vec<_>>>()?;

        match (json, comma_flag) {
            (true, true) => write!(output, ",{}", json_output)?,
            (true, false) => write!(output, "{}", json_output)?,
            _ => write!(output, "\n")?,
        };

        Ok(IterExecResult::Success)
    }
}
