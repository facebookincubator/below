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

use model::NetworkModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct TransportData {
    #[bttr(dfill_struct = "Transport")]
    #[bttr(
        title = "TcpActiveOpens/s",
        tag = "TransportField::ActiveOpens",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_active_opens_per_sec")]
    pub active_opens_per_sec: Option<u64>,
    #[bttr(
        title = "TcpPassiveOpens/s",
        tag = "TransportField::PassiveOpens",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_passive_opens_per_sec")]
    pub passive_opens_per_sec: Option<u64>,
    #[bttr(
        title = "TcpAttemptFails/s",
        tag = "TransportField::AttemptFailed",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_attempt_fails_per_sec")]
    pub attempt_fails_per_sec: Option<u64>,
    #[bttr(
        title = "TcpEstabResets/s",
        tag = "TransportField::EstabReset",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_estab_resets_per_sec")]
    pub estab_resets_per_sec: Option<u64>,
    #[bttr(
        title = "CurEstabConn",
        tag = "TransportField::CurrEstab",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_curr_estab_conn")]
    pub curr_estab_conn: Option<u64>,
    #[bttr(
        title = "TcpInSegs/s",
        tag = "TransportField::InSegs",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_in_segs_per_sec")]
    pub in_segs_per_sec: Option<u64>,
    #[bttr(
        title = "TcpOutSegs/s",
        tag = "TransportField::OutSegs",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_out_segs_per_sec")]
    pub out_segs_per_sec: Option<u64>,
    #[bttr(
        title = "TcpRetransSegs/s",
        tag = "TransportField::RetransSegsPS",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_retrans_segs_per_sec")]
    pub retrans_segs_per_sec: Option<u64>,
    #[bttr(
        title = "TcpRetransSegs",
        tag = "TransportField::RetransSegs",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_retrans_segs")]
    pub retrans_segs: Option<u64>,
    #[bttr(
        title = "TcpInErrors",
        tag = "TransportField::TcpInErrs",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_in_errs")]
    pub in_errs: Option<u64>,
    #[bttr(
        title = "TcpOutRsts/s",
        tag = "TransportField::OutRsts",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_out_rsts_per_sec")]
    pub out_rsts_per_sec: Option<u64>,
    #[bttr(
        title = "TcpInCsumErrors",
        tag = "TransportField::InCsumErrs",
        class = "TransportField::Tcp"
    )]
    #[blink("NetworkModel$tcp.get_in_csum_errors")]
    pub in_csum_errors: Option<u64>,
    #[bttr(
        title = "UdpInPkts/s",
        tag = "TransportField::InDgrms",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_in_datagrams_pkts_per_sec")]
    pub in_datagrams_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "UdpNoPorts",
        tag = "TransportField::NoPorts",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_no_ports")]
    pub no_ports: Option<u64>,
    #[bttr(
        title = "UdpInErrs",
        tag = "TransportField::UdpInErrs",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_in_errors")]
    pub in_errors: Option<u64>,
    #[bttr(
        title = "UdpOutPkts/s",
        tag = "TransportField::OutDgrms",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_out_datagrams_pkts_per_sec")]
    pub out_datagrams_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "UdpRcvbufErrs",
        tag = "TransportField::RecvBufErrs",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_rcvbuf_errors")]
    pub rcvbuf_errors: Option<u64>,
    #[bttr(
        title = "UdpSndBufErrs",
        tag = "TransportField::SndBufErrs",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_sndbuf_errors")]
    pub sndbuf_errors: Option<u64>,
    #[bttr(
        title = "UdpIgnoredMulti",
        tag = "TransportField::IgnoredMulti",
        class = "TransportField::Udp"
    )]
    #[blink("NetworkModel$udp.get_ignored_multi")]
    pub ignored_multi: Option<u64>,
    #[bttr(
        title = "Udp6InPkts/s",
        tag = "TransportField::InDgrms6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_in_datagrams_pkts_per_sec")]
    pub in_datagrams_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Udp6NoPorts",
        tag = "TransportField::NoPorts6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_no_ports")]
    pub no_ports6: Option<u64>,
    #[bttr(
        title = "Udp6InErrs",
        tag = "TransportField::UdpInErrs6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_in_errors")]
    pub in_errors6: Option<u64>,
    #[bttr(
        title = "Udp6OutPkts/s",
        tag = "TransportField::OutDgrms6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_out_datagrams_pkts_per_sec")]
    pub out_datagrams_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Udp6RcvbufErrs",
        tag = "TransportField::RecvBufErrs6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_rcvbuf_errors")]
    pub rcvbuf_errors6: Option<u64>,
    #[bttr(
        title = "Udp6SndBufErrs",
        tag = "TransportField::SndBufErrs6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_sndbuf_errors")]
    pub sndbuf_errors6: Option<u64>,
    #[bttr(
        title = "Udp6InCsumErrs",
        tag = "TransportField::InCsumErrs6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_in_csum_errors")]
    pub in_csum_errors6: Option<u64>,
    #[bttr(
        title = "Udp6IgnoredMulti",
        tag = "TransportField::IgnoredMulti6",
        class = "TransportField::Udp6"
    )]
    #[blink("NetworkModel$udp6.get_ignored_multi")]
    pub ignored_multi6: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "TransportField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "TransportField::Timestamp")]
    timestamp: i64,
}

type TitleFtype = Box<dyn Fn(&TransportData, &NetworkModel) -> String>;
type FieldFtype = Box<dyn Fn(&TransportData, &NetworkModel) -> String>;

pub struct Transport {
    data: TransportData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Transport {
    type Model = NetworkModel;
    type FieldsType = TransportField;
    type DataType = TransportData;
}

make_dget!(
    Transport,
    TransportField::Datetime,
    TransportField::Tcp,
    TransportField::Udp,
    TransportField::Udp6,
    TransportField::Timestamp,
);

impl Dprint for Transport {}

impl Dump for Transport {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        _: Option<TransportField>,
    ) -> Self {
        Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
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
        let network_model = &model.network;

        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let mut json_output = json!([]);

        match self.opts.output_format {
            Some(OutputFormat::Raw) | None => self.do_print_raw(network_model, output, *round),
            Some(OutputFormat::Csv) => self.do_print_csv(network_model, output, *round),
            Some(OutputFormat::KeyVal) => self.do_print_kv(network_model, output),
            Some(OutputFormat::Json) => {
                let par = self.do_print_json(network_model);
                json_output.as_array_mut().unwrap().push(par);
                Ok(())
            }
        }?;
        *round += 1;

        match (json, comma_flag) {
            (true, true) => write!(output, ",{}", json_output)?,
            (true, false) => write!(output, "{}", json_output)?,
            _ => write!(output, "\n")?,
        };

        Ok(IterExecResult::Success)
    }
}
