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

use crate::model::NetworkModel;

use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
pub struct NetworkData {
    #[bttr(
        title = "IpForwPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::ForwPkts&"
    )]
    #[blink("NetworkModel$ip.get_forwarding_pkts_per_sec")]
    pub forwarding_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInPkts/s",
        unit = " pkts",
        width = 2,
        tag = "NetworkField::InRecvPkts&"
    )]
    #[blink("NetworkModel$ip.get_in_receives_pkts_per_sec")]
    pub in_receives_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpForwDatagrams/s",
        width = 20,
        tag = "NetworkField::ForwDgrm&"
    )]
    #[blink("NetworkModel$ip.get_forw_datagrams_per_sec")]
    pub forw_datagrams_per_sec: Option<u64>,
    #[bttr(
        title = "IpInDiscardPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InDiscards&"
    )]
    #[blink("NetworkModel$ip.get_in_discards_pkts_per_sec")]
    pub in_discards_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInDeliversPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InDelivers&"
    )]
    #[blink("NetworkModel$ip.get_in_delivers_pkts_per_sec")]
    pub in_delivers_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutReqs/s",
        unit = " reqs",
        width = 20,
        tag = "NetworkField::OutRequests&"
    )]
    #[blink("NetworkModel$ip.get_out_requests_per_sec")]
    pub out_requests_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutDiscardPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutDiscards&"
    )]
    #[blink("NetworkModel$ip.get_out_discards_pkts_per_sec")]
    pub out_discards_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutNoRoutesPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutNoRoutes&"
    )]
    #[blink("NetworkModel$ip.get_out_no_routes_pkts_per_sec")]
    pub out_no_routes_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInMcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InMcast&"
    )]
    #[blink("NetworkModel$ip_ext.get_in_mcast_pkts_per_sec")]
    pub in_mcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutMcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutMcast&"
    )]
    #[blink("NetworkModel$ip_ext.get_out_mcast_pkts_per_sec")]
    pub out_mcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpInBcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InBcast&"
    )]
    #[blink("NetworkModel$ip_ext.get_in_bcast_pkts_per_sec")]
    pub in_bcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "IpOutBcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutBcast&"
    )]
    #[blink("NetworkModel$ip_ext.get_out_bcast_pkts_per_sec")]
    pub out_bcast_pkts_per_sec: Option<u64>,
    #[bttr(
        title = "Ip6InPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InRecvPkts6&"
    )]
    #[blink("NetworkModel$ip6.get_in_receives_pkts_per_sec")]
    pub in_receives_pkts_per_sec6: Option<u64>,
    #[bttr(title = "Ip6InHdrErrs", width = 20, tag = "NetworkField::InHdrErr6&")]
    #[blink("NetworkModel$ip6.get_in_hdr_errors")]
    pub in_hdr_errors6: Option<u64>,
    #[bttr(
        title = "Ip6InNoRoutesPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InNoRoutes6&"
    )]
    #[blink("NetworkModel$ip6.get_in_no_routes_pkts_per_sec")]
    pub in_no_routes_pkts_per_sec6: Option<u64>,
    #[bttr(title = "Ip6InAddrErrs", width = 20, tag = "NetworkField::InAddrErr6&")]
    #[blink("NetworkModel$ip6.get_in_addr_errors")]
    pub in_addr_errors6: Option<u64>,
    #[bttr(
        title = "Ip6InDiscardsPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InDiscards6&"
    )]
    #[blink("NetworkModel$ip6.get_in_discards_pkts_per_sec")]
    pub in_discards_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InDeliversPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InDelivers6&"
    )]
    #[blink("NetworkModel$ip6.get_in_delivers_pkts_per_sec")]
    pub in_delivers_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6ForwDatagrams/s",
        width = 20,
        tag = "NetworkField::ForwDgrm6&"
    )]
    #[blink("NetworkModel$ip6.get_out_forw_datagrams_per_sec")]
    pub out_forw_datagrams_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutReqs/s",
        unit = " reqs",
        width = 20,
        tag = "NetworkField::OutRequests6&"
    )]
    #[blink("NetworkModel$ip6.get_out_requests_per_sec")]
    pub out_requests_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutNoRoutesPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutNoRoutes6&"
    )]
    #[blink("NetworkModel$ip6.get_out_no_routes_pkts_per_sec")]
    pub out_no_routes_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InMcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::InMcast6&"
    )]
    #[blink("NetworkModel$ip6.get_in_mcast_pkts_per_sec")]
    pub in_mcast_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutMcastPkts/s",
        unit = " pkts",
        width = 20,
        tag = "NetworkField::OutMcast6&"
    )]
    #[blink("NetworkModel$ip6.get_out_mcast_pkts_per_sec")]
    pub out_mcast_pkts_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6InBcastOctets/s",
        unit = " octets",
        width = 20,
        tag = "NetworkField::InBcast6&"
    )]
    #[blink("NetworkModel$ip6.get_in_bcast_octets_per_sec")]
    pub in_bcast_octets_per_sec6: Option<u64>,
    #[bttr(
        title = "Ip6OutBcastOctets/s",
        unit = " octets",
        width = 20,
        tag = "NetworkField::OutBcast6&"
    )]
    #[blink("NetworkModel$ip6.get_out_bcast_octets_per_sec")]
    pub out_bcast_octets_per_sec6: Option<u64>,
    #[bttr(
        title = "IcmpInMsg/s",
        unit = " msgs",
        width = 20,
        tag = "NetworkField::InMsg&"
    )]
    #[blink("NetworkModel$icmp.get_in_msgs_per_sec")]
    pub in_msgs_per_sec: Option<u64>,
    #[bttr(title = "IcmpInErrs", width = 20, tag = "NetworkField::InErrs&")]
    #[blink("NetworkModel$icmp.get_in_errors")]
    pub in_errors: Option<u64>,
    #[bttr(
        title = "IcmpInDestUnreachs",
        width = 20,
        tag = "NetworkField::InDestUnreachs&"
    )]
    #[blink("NetworkModel$icmp.get_in_dest_unreachs")]
    pub in_dest_unreachs: Option<u64>,
    #[bttr(
        title = "IcmpOutMsg/s",
        unit = " msgs",
        width = 20,
        tag = "NetworkField::OutMsg&"
    )]
    #[blink("NetworkModel$icmp.get_out_msgs_per_sec")]
    pub out_msgs_per_sec: Option<u64>,
    #[bttr(title = "IcmpOutErrs", width = 20, tag = "NetworkField::OutErrs&")]
    #[blink("NetworkModel$icmp.get_out_errors")]
    pub out_errors: Option<u64>,
    #[bttr(
        title = "IcmpOutDestUnreachs",
        width = 20,
        tag = "NetworkField::OutDestUnreachs&"
    )]
    #[blink("NetworkModel$icmp.get_out_dest_unreachs")]
    pub out_dest_unreachs: Option<u64>,
    #[bttr(
        title = "Icmp6InMsg/s",
        unit = " msgs",
        width = 20,
        tag = "NetworkField::InMsg6&"
    )]
    #[blink("NetworkModel$icmp6.get_in_msgs_per_sec")]
    pub in_msgs_per_sec6: Option<u64>,
    #[bttr(title = "Icmp6InErrs", width = 20, tag = "NetworkField::InErrs6&")]
    #[blink("NetworkModel$icmp6.get_in_errors")]
    pub in_errors6: Option<u64>,
    #[bttr(
        title = "Icmp6InDestUnreachs",
        width = 20,
        tag = "NetworkField::InDestUnreachs6&"
    )]
    #[blink("NetworkModel$icmp6.get_in_dest_unreachs")]
    pub in_dest_unreachs6: Option<u64>,
    #[bttr(
        title = "Icmp6OutMsg/s",
        unit = " msgs",
        width = 20,
        tag = "NetworkField::OutMsg6&"
    )]
    #[blink("NetworkModel$icmp6.get_out_msgs_per_sec")]
    pub out_msgs_per_sec6: Option<u64>,
    #[bttr(title = "Icmp6OutErrs", width = 20, tag = "NetworkField::OutErrs6&")]
    #[blink("NetworkModel$icmp6.get_out_errors")]
    pub out_errors6: Option<u64>,
    #[bttr(
        title = "Icmp6OutDestUnreachs",
        width = 20,
        tag = "NetworkField::OutDestUnreachs6&"
    )]
    #[blink("NetworkModel$icmp6.get_out_dest_unreachs")]
    pub out_dest_unreachs6: Option<u64>,
    #[bttr(
        title = "Datetime",
        width = 19,
        decorator = "translate_datetime($)",
        tag = "NetworkField::Datetime"
    )]
    datetime: i64,
    #[bttr(title = "Timestamp", width = 10, tag = "NetworkField::Timestamp")]
    timestamp: i64,
    #[bttr(
        class = "NetworkField$forwarding_pkts_per_sec&,in_receives_pkts_per_sec&,forw_datagrams_per_sec&,in_discards_pkts_per_sec&,in_delivers_pkts_per_sec&,out_requests_per_sec&,out_discards_pkts_per_sec&,out_no_routes_pkts_per_sec&,in_mcast_pkts_per_sec&,out_mcast_pkts_per_sec&,in_bcast_pkts_per_sec&,out_bcast_pkts_per_sec&"
    )]
    pub ip: AwaysNone,
    #[bttr(
        class = "NetworkField$in_receives_pkts_per_sec6&,in_hdr_errors6&,in_no_routes_pkts_per_sec6&,in_addr_errors6&,in_discards_pkts_per_sec6&,in_delivers_pkts_per_sec6&,out_forw_datagrams_per_sec6&,out_requests_per_sec6&,out_no_routes_pkts_per_sec6&,in_mcast_pkts_per_sec6&,out_mcast_pkts_per_sec6&,in_bcast_octets_per_sec6&,out_bcast_octets_per_sec6&"
    )]
    pub ip6: AwaysNone,
    #[bttr(
        class = "NetworkField$in_msgs_per_sec&,in_errors&,in_dest_unreachs&,out_msgs_per_sec&,out_errors&,out_dest_unreachs&"
    )]
    pub icmp: AwaysNone,
    #[bttr(
        class = "NetworkField$in_msgs_per_sec6&,in_errors6&,in_dest_unreachs6&,out_msgs_per_sec6&,out_errors6&,out_dest_unreachs6&"
    )]
    pub icmp6: AwaysNone,
}

type TitleFtype = Box<dyn Fn(&NetworkData, &NetworkModel) -> &'static str>;
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
