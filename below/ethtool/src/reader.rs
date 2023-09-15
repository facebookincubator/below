use std::os::fd::{AsRawFd, OwnedFd};
use std::str;

use nix::errno::Errno;
use nix::libc::ioctl;
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockType};

use crate::errors::EthtoolError;
use crate::{
    ethtool_stringset_ETH_SS_STATS as ETH_SS_STATS, ETHTOOL_GSSET_INFO, ETHTOOL_GSTATS,
    ETHTOOL_GSTRINGS, ETH_GSTRING_LEN,
};

const ETH_GSTRINGS_LEN: usize = ETH_GSTRING_LEN as usize;
const ETH_GSTATS_LEN: usize = 8;

/// Maximum size of an interface name
const IFNAME_MAX_SIZE: usize = 16;

/// MAX_GSTRINGS maximum number of stats entries that ethtool can retrieve
const MAX_GSTRINGS: usize = 8192;

#[derive(Debug)]
#[repr(C)]
struct StringSetInfo {
    cmd: u32,
    reserved: u32,
    mask: u32,
    data: usize,
}

#[derive(Debug)]
#[repr(C)]
struct GStrings {
    pub cmd: u32,
    pub string_set: u32,
    pub len: u32,
    pub data: [u8; MAX_GSTRINGS * ETH_GSTRINGS_LEN],
}

#[derive(Debug)]
#[repr(C)]
struct GStats {
    pub cmd: u32,
    pub len: u32,
    pub data: [u8; MAX_GSTRINGS * ETH_GSTATS_LEN],
}

#[derive(Debug)]
#[repr(C)]
struct IfReq {
    if_name: [u8; IFNAME_MAX_SIZE],
    if_data: usize,
}

fn if_name_bytes(if_name: &str) -> [u8; IFNAME_MAX_SIZE] {
    let mut bytes = [0u8; IFNAME_MAX_SIZE];
    bytes[..if_name.len()].copy_from_slice(if_name.as_bytes());
    bytes
}

fn _ioctl(fd: &OwnedFd, if_name: [u8; IFNAME_MAX_SIZE], data: usize) -> Result<(), Errno> {
    let mut request = IfReq {
        if_name,
        if_data: data,
    };

    let exit_code = unsafe { ioctl(fd.as_raw_fd(), nix::libc::SIOCETHTOOL, &mut request) };

    if exit_code != 0 {
        return Err(Errno::from_i32(exit_code));
    }
    Ok(())
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTRINGS command.
/// In case of error during parsing any stat name,
/// the function returns a `ParseError`.
fn parse_names(data: &[u8], length: usize) -> Result<Vec<String>, EthtoolError> {
    let names = data
        .chunks(ETH_GSTRINGS_LEN)
        .map(|chunk| {
            // Find the position of the null terminator for specific stat name
            let null_pos = chunk.iter().position(|b| *b == 0).unwrap_or(length);
            // Convert the stat name to a string
            str::from_utf8(&chunk[..null_pos])
                .map(|s| s.to_string())
                .map_err(|err| EthtoolError::ParseError(err.to_string()))
        })
        .collect::<Result<Vec<String>, EthtoolError>>()?;

    Ok(names)
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTATS command.
/// In case of error during parsing any feature,
/// the function returns a `ParseError`.
fn parse_values(data: &[u8], length: usize) -> Result<Vec<u64>, EthtoolError> {
    let mut values = Vec::with_capacity(length);
    let mut value_bytes = [0u8; 8];
    for i in 0..length {
        let offset = 8 * i;
        match data.get(offset..offset + 8) {
            Some(slice) => {
                value_bytes.copy_from_slice(slice);
                values.push(u64::from_ne_bytes(value_bytes));
            }
            None => {
                return Err(EthtoolError::ParseError(format!(
                    "parse value failed at offset={}",
                    offset
                )))
            }
        }
    }

    Ok(values)
}

/// A trait for reading stats using ethtool.
///
/// This trait allows mocking the ethtool calls for unit testing.
pub trait EthtoolReadable {
    fn new(if_name: &str) -> Result<Self, EthtoolError>
    where
        Self: Sized;
    fn stats(&self) -> Result<Vec<(String, u64)>, EthtoolError>;
}

pub struct Ethtool {
    sock_fd: OwnedFd,
    if_name: [u8; IFNAME_MAX_SIZE],
}

impl Ethtool {
    /// Get the number of stats using ETHTOOL_GSSET_INFO command
    fn gsset_info(&self) -> Result<usize, EthtoolError> {
        let mut sset_info = StringSetInfo {
            cmd: ETHTOOL_GSSET_INFO,
            reserved: 1,
            mask: 1 << ETH_SS_STATS,
            data: 0,
        };

        match _ioctl(
            &self.sock_fd,
            self.if_name,
            &mut sset_info as *mut StringSetInfo as usize,
        ) {
            Ok(_) => Ok(sset_info.data),
            Err(errno) => Err(EthtoolError::GSSetInfoReadError(errno)),
        }
    }

    /// Get the feature names using ETHTOOL_GSTRINGS command
    fn gstrings(&self, length: usize) -> Result<Vec<String>, EthtoolError> {
        let mut gstrings = GStrings {
            cmd: ETHTOOL_GSTRINGS,
            string_set: ETH_SS_STATS,
            len: length as u32,
            data: [0u8; MAX_GSTRINGS * ETH_GSTRINGS_LEN],
        };

        match _ioctl(
            &self.sock_fd,
            self.if_name,
            &mut gstrings as *mut GStrings as usize,
        ) {
            Ok(_) => return parse_names(&gstrings.data[..length * ETH_GSTRINGS_LEN], length),
            Err(errno) => Err(EthtoolError::GStringsReadError(errno)),
        }
    }

    /// Get the statistics for the features using ETHTOOL_GSTATS command
    fn gstats(&self, features: &[String]) -> Result<Vec<u64>, EthtoolError> {
        let length = features.len();
        let mut gstats = GStats {
            cmd: ETHTOOL_GSTATS,
            len: features.len() as u32,
            data: [0u8; MAX_GSTRINGS * ETH_GSTATS_LEN],
        };

        match _ioctl(
            &self.sock_fd,
            self.if_name,
            &mut gstats as *mut GStats as usize,
        ) {
            Ok(_) => return parse_values(&gstats.data[..length * ETH_GSTATS_LEN], length),
            Err(errno) => Err(EthtoolError::GStatsReadError(errno)),
        }
    }
}

impl EthtoolReadable for Ethtool {
    fn new(if_name: &str) -> Result<Self, EthtoolError> {
        match socket(
            AddressFamily::Inet,
            SockType::Datagram,
            SockFlag::empty(),
            None,
        ) {
            Ok(fd) => Ok(Ethtool {
                sock_fd: fd,
                if_name: if_name_bytes(if_name),
            }),
            Err(errno) => Err(EthtoolError::SocketError(errno)),
        }
    }

    /// Get statistics using ethtool
    /// Equivalent to `ethtool -S <ifname>` command
    fn stats(&self) -> Result<Vec<(String, u64)>, EthtoolError> {
        let length = self.gsset_info()?;

        let features = self.gstrings(length)?;
        let values = self.gstats(&features)?;

        let final_stats = features.into_iter().zip(values).collect();
        Ok(final_stats)
    }
}
