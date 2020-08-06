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

use common::model::SingleNetModel;
use common::util::convert_bytes;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct IfaceData {
    #[bttr(title = "interface", width = 20, tag = "IfaceField::Interface&")]
    #[blink("SingleNetModel$get_interface")]
    pub interface: String,
    #[bttr(
        title = "RX Bytes/s",
        width = 20,
        tag = "IfaceField::RBps&",
        decorator = "convert_bytes($)"
    )]
    #[blink("SingleNetModel$get_rx_bytes_per_sec")]
    pub rx_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "TX Bytes/s",
        width = 20,
        tag = "IfaceField::TBps&",
        decorator = "convert_bytes($)"
    )]
    #[blink("SingleNetModel$get_tx_bytes_per_sec")]
    pub tx_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "I/O Bytes/s",
        width = 20,
        tag = "IfaceField::IOBps&",
        decorator = "convert_bytes($)"
    )]
    #[blink("SingleNetModel$get_throughput_per_sec")]
    pub throughput_per_sec: Option<f64>,
    #[bttr(title = "RX pkts/s", width = 20, tag = "IfaceField::RPktps&")]
    #[blink("SingleNetModel$get_rx_packets_per_sec")]
    pub rx_packets_per_sec: Option<u64>,
    #[bttr(title = "TX pkts/s", width = 20, tag = "IfaceField::TPktps&")]
    #[blink("SingleNetModel$get_tx_packets_per_sec")]
    pub tx_packets_per_sec: Option<u64>,
    #[bttr(title = "Collisions", width = 20, tag = "IfaceField::Collisions&")]
    #[blink("SingleNetModel$get_collisions")]
    pub collisions: Option<u64>,
    #[bttr(title = "Multicast", width = 20, tag = "IfaceField::Multicast&")]
    #[blink("SingleNetModel$get_multicast")]
    pub multicast: Option<u64>,
    #[bttr(title = "RX Bytes", width = 20, tag = "IfaceField::RxBytes&")]
    #[blink("SingleNetModel$get_rx_bytes")]
    pub rx_bytes: Option<u64>,
    #[bttr(title = "RX Compressed", width = 20, tag = "IfaceField::RxCompressed&")]
    #[blink("SingleNetModel$get_rx_compressed")]
    pub rx_compressed: Option<u64>,
    #[bttr(title = "RX CRC Errors", width = 20, tag = "IfaceField::RxCrcErr&")]
    #[blink("SingleNetModel$get_rx_crc_errors")]
    pub rx_crc_errors: Option<u64>,
    #[bttr(title = "RX Dropped", width = 20, tag = "IfaceField::RxDropped&")]
    #[blink("SingleNetModel$get_rx_dropped")]
    pub rx_dropped: Option<u64>,
    #[bttr(title = "RX Errors", width = 20, tag = "IfaceField::RxErr&")]
    #[blink("SingleNetModel$get_rx_errors")]
    pub rx_errors: Option<u64>,
    #[bttr(title = "RX Fifo Errors", width = 20, tag = "IfaceField::RxFifoErr&")]
    #[blink("SingleNetModel$get_rx_fifo_errors")]
    pub rx_fifo_errors: Option<u64>,
    #[bttr(title = "RX Frame Errors", width = 20, tag = "IfaceField::RxFrameErr&")]
    #[blink("SingleNetModel$get_rx_frame_errors")]
    pub rx_frame_errors: Option<u64>,
    #[bttr(
        title = "RX Length Errors",
        width = 20,
        tag = "IfaceField::RxLengthErr&"
    )]
    #[blink("SingleNetModel$get_rx_length_errors")]
    pub rx_length_errors: Option<u64>,
    #[bttr(
        title = "RX Missed Errors",
        width = 20,
        tag = "IfaceField::RxMissedErr&"
    )]
    #[blink("SingleNetModel$get_rx_missed_errors")]
    pub rx_missed_errors: Option<u64>,
    #[bttr(title = "RX Nohandler", width = 20, tag = "IfaceField::RxNohandler&")]
    #[blink("SingleNetModel$get_rx_nohandler")]
    pub rx_nohandler: Option<u64>,
    #[bttr(title = "RX Over Errors", width = 20, tag = "IfaceField::RxOverErr&")]
    #[blink("SingleNetModel$get_rx_over_errors")]
    pub rx_over_errors: Option<u64>,
    #[bttr(title = "RX Packets", width = 20, tag = "IfaceField::RxPckt&")]
    #[blink("SingleNetModel$get_rx_packets")]
    pub rx_packets: Option<u64>,
    #[bttr(
        title = "TX Aborted Errors",
        width = 20,
        tag = "IfaceField::TxAbortedErr&"
    )]
    #[blink("SingleNetModel$get_tx_aborted_errors")]
    pub tx_aborted_errors: Option<u64>,
    #[bttr(title = "TX Bytes", width = 20, tag = "IfaceField::TxBytes&")]
    #[blink("SingleNetModel$get_tx_bytes")]
    pub tx_bytes: Option<u64>,
    #[bttr(
        title = "TX Carrier Errors",
        width = 20,
        tag = "IfaceField::TxCarrierErr&"
    )]
    #[blink("SingleNetModel$get_tx_carrier_errors")]
    pub tx_carrier_errors: Option<u64>,
    #[bttr(title = "TX Compressed", width = 20, tag = "IfaceField::TxCompressed&")]
    #[blink("SingleNetModel$get_tx_compressed")]
    pub tx_compressed: Option<u64>,
    #[bttr(title = "TX Dropped", width = 20, tag = "IfaceField::TxDropped&")]
    #[blink("SingleNetModel$get_tx_dropped")]
    pub tx_dropped: Option<u64>,
    #[bttr(title = "TX Errors", width = 20, tag = "IfaceField::TxErr&")]
    #[blink("SingleNetModel$get_tx_errors")]
    pub tx_errors: Option<u64>,
    #[bttr(title = "TX Fifo Errors", width = 20, tag = "IfaceField::TxFifoErr&")]
    #[blink("SingleNetModel$get_tx_fifo_errors")]
    pub tx_fifo_errors: Option<u64>,
    #[bttr(
        title = "TX Heartbeat Errors",
        width = 20,
        tag = "IfaceField::TxHeartBeatErr&"
    )]
    #[blink("SingleNetModel$get_tx_heartbeat_errors")]
    pub tx_heartbeat_errors: Option<u64>,
    #[bttr(title = "TX Packets", width = 20, tag = "IfaceField::TxPckt&")]
    #[blink("SingleNetModel$get_tx_packets")]
    pub tx_packets: Option<u64>,
    #[bttr(
        title = "TX Window Errors",
        width = 20,
        tag = "IfaceField::TxWindowErr&"
    )]
    #[blink("SingleNetModel$get_tx_window_errors")]
    pub tx_window_errors: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime($)",
        tag = "IfaceField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "IfaceField::Timestamp")]
    timestamp: i64,
    #[bttr(
        class = "IfaceField$rx_bytes_per_sec&,tx_bytes_per_sec&,throughput_per_sec&:rx_packets_per_sec&,tx_packets_per_sec&"
    )]
    pub rate: AwaysNone,
    #[bttr(
        class = "IfaceField$rx_bytes&,rx_dropped&,rx_errors&:rx_compressed&,rx_crc_errors&,rx_fifo_errors&,rx_frame_errors&,rx_length_errors&,rx_missed_errors&,rx_nohandler&,rx_over_errors&,rx_packets&"
    )]
    pub rx: AwaysNone,
    #[bttr(
        class = "IfaceField$tx_bytes&,tx_dropped&,tx_errors&:tx_aborted_errors&,tx_carrier_errors&,tx_compressed&,tx_fifo_errors&,tx_heartbeat_errors&,tx_packets&,tx_window_errors&"
    )]
    pub tx: AwaysNone,
}

type TitleFtype = Box<dyn Fn(&IfaceData, &SingleNetModel) -> &'static str>;
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
