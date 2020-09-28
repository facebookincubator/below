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
pub struct NetworkData {
    #[bttr(dfill_struct = "Network", raw = "self.raw")]
    #[bttr(
        title = "IpForwPkts/s",
        tag = "NetworkField::ForwPkts",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_forwarding_pkts_per_sec")]
    pub forwarding_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInPkts/s",
        tag = "NetworkField::InRecvPkts",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_in_receives_pkts_per_sec")]
    pub in_receives_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpForwDatagrams/s",
        tag = "NetworkField::ForwDgrm",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_forw_datagrams_per_sec")]
    pub forw_datagrams_per_sec: Option<u64>,
    #[bttr(
        title = "IpInDiscardPkts/s",
        tag = "NetworkField::InDiscards",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_in_discards_pkts_per_sec")]
    pub in_discards_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInDeliversPkts/s",
        tag = "NetworkField::InDelivers",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_in_delivers_pkts_per_sec")]
    pub in_delivers_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutReqs/s",
        tag = "NetworkField::OutRequests",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_out_requests_per_sec")]
    pub out_requests_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutDiscardPkts/s",
        tag = "NetworkField::OutDiscards",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_out_discards_pkts_per_sec")]
    pub out_discards_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutNoRoutesPkts/s",
        tag = "NetworkField::OutNoRoutes",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip.get_out_no_routes_pkts_per_sec")]
    pub out_no_routes_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInMcastPkts/s",
        tag = "NetworkField::InMcast",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip_ext.get_in_mcast_pkts_per_sec")]
    pub in_mcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutMcastPkts/s",
        tag = "NetworkField::OutMcast",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip_ext.get_out_mcast_pkts_per_sec")]
    pub out_mcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInBcastPkts/s",
        tag = "NetworkField::InBcast",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip_ext.get_in_bcast_pkts_per_sec")]
    pub in_bcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutBcastPkts/s",
        tag = "NetworkField::OutBcast",
        class = "NetworkField::Ip"
    )]
    #[blink("NetworkModel$ip_ext.get_out_bcast_pkts_per_sec")]
    pub out_bcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "Ip6InPkts/s",
        tag = "NetworkField::InRecvPkts6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_receives_pkts_per_sec")]
    pub in_receives_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InHdrErrs",
        tag = "NetworkField::InHdrErr6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_hdr_errors")]
    pub in_hdr_errors6: Option<u64>,
    #[bttr(
        title = "Ip6InNoRoutesPkts/s",
        tag = "NetworkField::InNoRoutes6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_no_routes_pkts_per_sec")]
    pub in_no_routes_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InAddrErrs",
        tag = "NetworkField::InAddrErr6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_addr_errors")]
    pub in_addr_errors6: Option<u64>,
    #[bttr(
        title = "Ip6InDiscardsPkts/s",
        tag = "NetworkField::InDiscards6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_discards_pkts_per_sec")]
    pub in_discards_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InDeliversPkts/s",
        tag = "NetworkField::InDelivers6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_delivers_pkts_per_sec")]
    pub in_delivers_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6ForwDatagrams/s",
        tag = "NetworkField::ForwDgrm6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_out_forw_datagrams_per_sec")]
    pub out_forw_datagrams_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutReqs/s",
        tag = "NetworkField::OutRequests6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_out_requests_per_sec")]
    pub out_requests_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutNoRoutesPkts/s",
        tag = "NetworkField::OutNoRoutes6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_out_no_routes_pkts_per_sec")]
    pub out_no_routes_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InMcastPkts/s",
        tag = "NetworkField::InMcast6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_mcast_pkts_per_sec")]
    pub in_mcast_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutMcastPkts/s",
        tag = "NetworkField::OutMcast6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_out_mcast_pkts_per_sec")]
    pub out_mcast_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InBcastOctets/s",
        tag = "NetworkField::InBcast6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_in_bcast_octets_per_sec")]
    pub in_bcast_octets_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutBcastOctets/s",
        tag = "NetworkField::OutBcast6",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$ip6.get_out_bcast_octets_per_sec")]
    pub out_bcast_octets_per_sec6: Option<u64>,
    #[bttr(
        title = "IcmpInMsg/s",
        tag = "NetworkField::InMsg",
        class = "NetworkField::Ip6"
    )]
    #[blink("NetworkModel$icmp.get_in_msgs_per_sec")]
    pub in_msgs_per_sec: Option<u64>,
    #[bttr(
        title = "IcmpInErrs",
        tag = "NetworkField::InErrs",
        class = "NetworkField::Icmp"
    )]
    #[blink("NetworkModel$icmp.get_in_errors")]
    pub in_errors: Option<u64>,
    #[bttr(
        title = "IcmpInDestUnreachs",
        tag = "NetworkField::InDestUnreachs",
        class = "NetworkField::Icmp"
    )]
    #[blink("NetworkModel$icmp.get_in_dest_unreachs")]
    pub in_dest_unreachs: Option<u64>,
    #[bttr(
        title = "IcmpOutMsg/s",
        tag = "NetworkField::OutMsg",
        class = "NetworkField::Icmp"
    )]
    #[blink("NetworkModel$icmp.get_out_msgs_per_sec")]
    pub out_msgs_per_sec: Option<u64>,
    #[bttr(
        title = "IcmpOutErrs",
        tag = "NetworkField::OutErrs",
        class = "NetworkField::Icmp"
    )]
    #[blink("NetworkModel$icmp.get_out_errors")]
    pub out_errors: Option<u64>,
    #[bttr(
        title = "IcmpOutDestUnreachs",
        tag = "NetworkField::OutDestUnreachs",
        class = "NetworkField::Icmp"
    )]
    #[blink("NetworkModel$icmp.get_out_dest_unreachs")]
    pub out_dest_unreachs: Option<u64>,
    #[bttr(
        title = "Icmp6InMsg/s",
        tag = "NetworkField::InMsg6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_in_msgs_per_sec")]
    pub in_msgs_per_sec6: Option<u64>,
    #[bttr(
        title = "Icmp6InErrs",
        tag = "NetworkField::InErrs6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_in_errors")]
    pub in_errors6: Option<u64>,
    #[bttr(
        title = "Icmp6InDestUnreachs",
        tag = "NetworkField::InDestUnreachs6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_in_dest_unreachs")]
    pub in_dest_unreachs6: Option<u64>,
    #[bttr(
        title = "Icmp6OutMsg/s",
        tag = "NetworkField::OutMsg6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_out_msgs_per_sec")]
    pub out_msgs_per_sec6: Option<u64>,
    #[bttr(
        title = "Icmp6OutErrs",
        tag = "NetworkField::OutErrs6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_out_errors")]
    pub out_errors6: Option<u64>,
    #[bttr(
        title = "Icmp6OutDestUnreachs",
        tag = "NetworkField::OutDestUnreachs6",
        class = "NetworkField::Icmp6"
    )]
    #[blink("NetworkModel$icmp6.get_out_dest_unreachs")]
    pub out_dest_unreachs6: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime(&$)",
        tag = "NetworkField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "NetworkField::Timestamp")]
    timestamp: i64,
    raw: bool,
}

type TitleFtype = Box<dyn Fn(&NetworkData, &NetworkModel) -> String>;
type FieldFtype = Box<dyn Fn(&NetworkData, &NetworkModel) -> String>;

pub struct Network {
    data: NetworkData,
    opts: GeneralOpt,
    advance: Advance,
    time_end: SystemTime,
    pub title_fns: Vec<TitleFtype>,
    pub field_fns: Vec<FieldFtype>,
}

impl DumpType for Network {
    type Model = NetworkModel;
    type FieldsType = NetworkField;
    type DataType = NetworkData;
}

make_dget!(
    Network,
    NetworkField::Datetime,
    NetworkField::Ip,
    NetworkField::Ip6,
    NetworkField::Icmp,
    NetworkField::Icmp6,
    NetworkField::Timestamp,
);

impl Dprint for Network {}

impl Dump for Network {
    fn new(
        opts: GeneralOpt,
        advance: Advance,
        time_end: SystemTime,
        _: Option<NetworkField>,
    ) -> Self {
        let mut network = Self {
            data: Default::default(),
            opts,
            advance,
            time_end,
            title_fns: vec![],
            field_fns: vec![],
        };
        network.data.raw = network.opts.raw;
        network
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
