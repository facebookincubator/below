mod errors;
mod types;

#[cfg(test)]
mod test;

use std::borrow::{Borrow, BorrowMut};
use std::collections::BTreeMap;

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_DUMP, NLM_F_REQUEST,
};
use netlink_packet_route::tc::TcMessage;
use netlink_packet_route::RouteNetlinkMessage;
use netlink_sys::constants::NETLINK_ROUTE;
use netlink_sys::{Socket, SocketAddr};
use nix::net::if_;

use errors::TcError;
pub use types::{FqCodelQDisc, FqCodelQdStats, FqCodelXStats, QDisc, TcStat, XStats};

pub type TcStats = Vec<TcStat>;
type Result<T> = std::result::Result<T, TcError>;

/// Get list of all `tc` qdiscs.
pub fn tc_stats() -> Result<TcStats> {
    let ifaces = get_interfaces()?;
    read_tc_stats(ifaces, &get_netlink_qdiscs)
}

fn read_tc_stats(
    interfaces: BTreeMap<u32, String>,
    netlink_retriever: &dyn Fn() -> Result<Vec<TcMessage>>,
) -> Result<TcStats> {
    let messages = netlink_retriever()?;
    let tc_stats = messages
        .into_iter()
        .filter_map(|msg| {
            interfaces
                .get(&(msg.header.index as u32))
                .cloned()
                .map(|if_name| TcStat::new(if_name, &msg))
        })
        .collect();

    Ok(tc_stats)
}

/// Open a netlink socket to retrieve `tc` qdiscs.
/// The underlying library sends a message of type `RTM_GETQDISC` to the kernel.
/// The kernel responds with a message of type `RTM_NEWQDISC` for each qdisc.
fn get_netlink_qdiscs() -> Result<Vec<TcMessage>> {
    // open a socket
    let socket = Socket::new(NETLINK_ROUTE).map_err(|e| TcError::Netlink(e.to_string()))?;
    socket
        .connect(&SocketAddr::new(0, 0))
        .map_err(|e| TcError::Netlink(e.to_string()))?;

    // create a netlink request
    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_DUMP;
    let msg = RouteNetlinkMessage::GetQueueDiscipline(TcMessage::default());
    let mut packet = NetlinkMessage::new(nl_hdr, NetlinkPayload::from(msg));
    packet.finalize();
    let mut buf = vec![0; packet.header.length as usize];
    packet.serialize(buf[..].borrow_mut());

    // send the request
    socket
        .send(buf[..].borrow(), 0)
        .map_err(|e| TcError::Netlink(e.to_string()))?;

    // receive the response
    let mut recv_buf = vec![0; 4096];
    let mut offset = 0;
    let mut response = Vec::new();
    'out: while let Ok(size) = socket.recv(&mut recv_buf[..].borrow_mut(), 0) {
        loop {
            let bytes = recv_buf[offset..].borrow();
            let rx_packet = <NetlinkMessage<RouteNetlinkMessage>>::deserialize(bytes)
                .map_err(|e| TcError::Netlink(e.to_string()))?;
            response.push(rx_packet.clone());
            let payload = rx_packet.payload;
            if let NetlinkPayload::Error(err) = payload {
                return Err(TcError::Netlink(err.to_string()));
            }
            if let NetlinkPayload::Done(_) = payload {
                break 'out;
            }

            offset += rx_packet.header.length as usize;
            if offset == size || rx_packet.header.length == 0 {
                offset = 0;
                break;
            }
        }
    }

    let mut tc_msgs = Vec::new();
    for msg in response {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewQueueDiscipline(tc)) =
            msg.payload
        {
            tc_msgs.push(tc);
        }
    }

    return Ok(tc_msgs);
}

/// Get a map of interface index to interface name.
fn get_interfaces() -> Result<BTreeMap<u32, String>> {
    let ifaces = if_::if_nameindex().map_err(|e| TcError::ReadInterfaces(e.to_string()))?;
    let if_map = ifaces
        .iter()
        .map(|iface| {
            let index = iface.index();
            let name = if let Ok(name) = iface.name().to_str() {
                name.to_string()
            } else {
                String::new()
            };
            (index, name)
        })
        .collect::<BTreeMap<u32, String>>();

    Ok(if_map)
}
