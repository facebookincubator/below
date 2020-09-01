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

use model::SingleNetModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct IfaceData {
    #[bttr(dfill_struct = "Iface")]
    #[bttr(title = "interface", tag = "IfaceField::Interface")]
    #[blink("SingleNetModel$get_interface")]
    pub interface: String,
    #[bttr(
        title = "RX Bytes/s",
        tag = "IfaceField::RBps",
        class = "IfaceField::Rate"
    )]
    #[blink("SingleNetModel$get_rx_bytes_per_sec")]
    pub rx_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "TX Bytes/s",
        tag = "IfaceField::TBps",
        class = "IfaceField::Rate"
    )]
    #[blink("SingleNetModel$get_tx_bytes_per_sec")]
    pub tx_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "I/O Bytes/s",
        tag = "IfaceField::IOBps",
        class = "IfaceField::Rate"
    )]
    #[blink("SingleNetModel$get_throughput_per_sec")]
    pub throughput_per_sec: Option<f64>,
    #[bttr(
        title = "RX pkts/s",
        tag = "IfaceField::RPktps",
        class = "IfaceField::Rate"
    )]
    #[blink("SingleNetModel$get_rx_packets_per_sec")]
    pub rx_packets_per_sec: Option<u64>,
    #[bttr(
        title = "TX pkts/s",
        tag = "IfaceField::TPktps",
        class = "IfaceField::Rate"
    )]
    #[blink("SingleNetModel$get_tx_packets_per_sec")]
    pub tx_packets_per_sec: Option<u64>,
    #[bttr(title = "Collisions", tag = "IfaceField::Collisions")]
    #[blink("SingleNetModel$get_collisions")]
    pub collisions: Option<u64>,
    #[bttr(title = "Multicast", tag = "IfaceField::Multicast")]
    #[blink("SingleNetModel$get_multicast")]
    pub multicast: Option<u64>,
    #[bttr(
        title = "RX Bytes",
        tag = "IfaceField::RxBytes",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_bytes")]
    pub rx_bytes: Option<u64>,
    #[bttr(
        title = "RX Compressed",
        tag = "IfaceField::RxCompressed",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_compressed")]
    pub rx_compressed: Option<u64>,
    #[bttr(
        title = "RX CRC Errors",
        tag = "IfaceField::RxCrcErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_crc_errors")]
    pub rx_crc_errors: Option<u64>,
    #[bttr(
        title = "RX Dropped",
        tag = "IfaceField::RxDropped",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_dropped")]
    pub rx_dropped: Option<u64>,
    #[bttr(
        title = "RX Errors",
        tag = "IfaceField::RxErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_errors")]
    pub rx_errors: Option<u64>,
    #[bttr(
        title = "RX Fifo Errors",
        tag = "IfaceField::RxFifoErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_fifo_errors")]
    pub rx_fifo_errors: Option<u64>,
    #[bttr(
        title = "RX Frame Errors",
        tag = "IfaceField::RxFrameErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_frame_errors")]
    pub rx_frame_errors: Option<u64>,
    #[bttr(
        title = "RX Length Errors",
        tag = "IfaceField::RxLengthErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_length_errors")]
    pub rx_length_errors: Option<u64>,
    #[bttr(
        title = "RX Missed Errors",
        tag = "IfaceField::RxMissedErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_missed_errors")]
    pub rx_missed_errors: Option<u64>,
    #[bttr(
        title = "RX Nohandler",
        tag = "IfaceField::RxNohandler",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_nohandler")]
    pub rx_nohandler: Option<u64>,
    #[bttr(
        title = "RX Over Errors",
        tag = "IfaceField::RxOverErr",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_over_errors")]
    pub rx_over_errors: Option<u64>,
    #[bttr(
        title = "RX Packets",
        tag = "IfaceField::RxPckt",
        class = "IfaceField::Rx"
    )]
    #[blink("SingleNetModel$get_rx_packets")]
    pub rx_packets: Option<u64>,
    #[bttr(
        title = "TX Aborted Errors",
        tag = "IfaceField::TxAbortedErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_aborted_errors")]
    pub tx_aborted_errors: Option<u64>,
    #[bttr(
        title = "TX Bytes",
        tag = "IfaceField::TxBytes",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_bytes")]
    pub tx_bytes: Option<u64>,
    #[bttr(
        title = "TX Carrier Errors",
        tag = "IfaceField::TxCarrierErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_carrier_errors")]
    pub tx_carrier_errors: Option<u64>,
    #[bttr(
        title = "TX Compressed",
        tag = "IfaceField::TxCompressed",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_compressed")]
    pub tx_compressed: Option<u64>,
    #[bttr(
        title = "TX Dropped",
        tag = "IfaceField::TxDropped",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_dropped")]
    pub tx_dropped: Option<u64>,
    #[bttr(
        title = "TX Errors",
        tag = "IfaceField::TxErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_errors")]
    pub tx_errors: Option<u64>,
    #[bttr(
        title = "TX Fifo Errors",
        tag = "IfaceField::TxFifoErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_fifo_errors")]
    pub tx_fifo_errors: Option<u64>,
    #[bttr(
        title = "TX Heartbeat Errors",
        tag = "IfaceField::TxHeartBeatErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_heartbeat_errors")]
    pub tx_heartbeat_errors: Option<u64>,
    #[bttr(
        title = "TX Packets",
        tag = "IfaceField::TxPckt",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_packets")]
    pub tx_packets: Option<u64>,
    #[bttr(
        title = "TX Window Errors",
        tag = "IfaceField::TxWindowErr",
        class = "IfaceField::Tx"
    )]
    #[blink("SingleNetModel$get_tx_window_errors")]
    pub tx_window_errors: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "IfaceField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "IfaceField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&IfaceData, &SingleNetModel) -> String>;
type FieldFtype = Box<dyn Fn(&IfaceData, &SingleNetModel) -> String>;

pub struct Iface {
    data: IfaceData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    select: Option<IfaceField>,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Iface {
    type Model = SingleNetModel;
    type FieldsType = IfaceField;
    type DataType = IfaceData;
}

make_dget!(
    Iface,
    IfaceField::Datetime,
    IfaceField::Collisions,
    IfaceField::Multicast,
    IfaceField::Interface,
    IfaceField::Rate,
    IfaceField::Rx,
    IfaceField::Tx,
    IfaceField::Timestamp,
);

impl Dprint for Iface {}

impl Dump for Iface {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        select: Option<IfaceField>,
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
        let ifaces: Vec<&SingleNetModel> = model
            .network
            .interfaces
            .iter()
            .filter_map(|(_, spm)| match (self.select, self.opts.filter.as_ref()) {
                (Some(tag), Some(re)) => {
                    if self.filter_by(spm, &tag, &re) {
                        Some(spm)
                    } else {
                        None
                    }
                }
                _ => Some(spm),
            })
            .collect();

        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        ifaces
            .iter()
            .map(|spm| {
                let ret = match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => self.do_print_raw(&spm, output, *round),
                    Some(OutputFormat::Csv) => self.do_print_csv(&spm, output, *round),
                    Some(OutputFormat::KeyVal) => self.do_print_kv(&spm, output),
                    Some(OutputFormat::Json) => {
                        let par = self.do_print_json(&spm);
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
