use std::{
    alloc,
    ffi::CStr,
    mem,
    os::fd::{AsRawFd, OwnedFd},
    ptr, str,
};

use nix::errno::Errno;
use nix::libc;
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockType};

use crate::errors::EthtoolError;
use crate::{
    ethtool_gstrings, ethtool_sset_info, ethtool_stats,
    ethtool_stringset_ETH_SS_STATS as ETH_SS_STATS, ETHTOOL_GSSET_INFO, ETHTOOL_GSTATS,
    ETHTOOL_GSTRINGS, ETH_GSTRING_LEN,
};

const ETH_GSTRINGS_LEN: usize = ETH_GSTRING_LEN as usize;
const ETH_GSTATS_LEN: usize = 8;

fn if_name_bytes(if_name: &str) -> [i8; libc::IF_NAMESIZE] {
    let mut it = if_name.as_bytes().iter().copied();
    [0; libc::IF_NAMESIZE].map(|_| it.next().unwrap_or(0) as libc::c_char)
}

fn ioctl(
    fd: &OwnedFd,
    if_name: [i8; libc::IF_NAMESIZE],
    data: *mut libc::c_char,
) -> Result<(), Errno> {
    let ifr = libc::ifreq {
        ifr_name: if_name,
        ifr_ifru: libc::__c_anonymous_ifr_ifru { ifru_data: data },
    };

    let exit_code = unsafe { libc::ioctl(fd.as_raw_fd(), nix::libc::SIOCETHTOOL, &ifr) };
    if exit_code != 0 {
        return Err(Errno::from_i32(exit_code));
    }
    Ok(())
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTRINGS command.
/// In case of error during parsing any stat name,
/// the function returns a `ParseError`.
fn parse_names(data: &[u8]) -> Result<Vec<String>, EthtoolError> {
    let names = data
        .chunks(ETH_GSTRINGS_LEN)
        .map(|chunk| {
            // // Find the position of the null terminator for specific stat name
            // let null_pos = chunk.iter().position(|b| *b == 0).unwrap_or(length);
            // // Convert the stat name to a string
            // str::from_utf8(&chunk[..null_pos])
            //     .map(|s| s.to_string())
            //     .map_err(|err| EthtoolError::ParseError(err.to_string()))
            let c_str = CStr::from_bytes_until_nul(chunk);
            match c_str {
                Ok(c_str) => Ok(c_str.to_string_lossy().into_owned()),
                Err(err) => Err(EthtoolError::ParseError(err.to_string())),
            }
        })
        .collect::<Result<Vec<String>, EthtoolError>>()?;

    Ok(names)
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTATS command.
/// In case of error during parsing any feature,
/// the function returns a `ParseError`.
fn parse_values(data: &[u64], length: usize) -> Result<Vec<u64>, EthtoolError> {
    let values = data
        .iter()
        .take(length)
        .map(|value| *value)
        .collect::<Vec<u64>>();

    Ok(values)
}

struct StringSetInfo {
    layout: alloc::Layout,
    ptr: *mut ethtool_sset_info,
}

impl StringSetInfo {
    /// Allocates memory for ethtool_sset_info struct and initializes it.
    fn new() -> Result<Self, EthtoolError> {
        // Calculate the layout with proper alignment
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_sset_info>(),
            mem::align_of::<ethtool_sset_info>(),
        )
        .map_err(|err| EthtoolError::CStructInitError(err))?;

        // Allocate memory for the struct
        let sset_info_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_sset_info;

        // Initialize the fields of the struct
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*sset_info_ptr).cmd);
            let reserved_ptr = ptr::addr_of_mut!((*sset_info_ptr).reserved);
            let sset_mask_ptr = ptr::addr_of_mut!((*sset_info_ptr).sset_mask);
            let data_ptr = ptr::addr_of_mut!((*sset_info_ptr).data);

            cmd_ptr.write(ETHTOOL_GSSET_INFO);
            reserved_ptr.write(1u32);
            sset_mask_ptr.write((1 << ETH_SS_STATS) as u64);

            data_ptr.write_bytes(0u8, mem::size_of::<u32>());
        }

        Ok(StringSetInfo {
            layout,
            ptr: sset_info_ptr,
        })
    }

    fn data(&self) -> Result<usize, EthtoolError> {
        // (unsafe { self.ptr.as_mut().unwrap().data.as_ptr().read() }) as usize
        unsafe {
            let value = self.ptr.as_ref().ok_or(EthtoolError::CStructReadError())?;
            Ok(value.data.as_ptr().read() as usize)
        }
    }
}

impl Drop for StringSetInfo {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr as *mut u8, self.layout);
        }
    }
}

struct GStrings {
    length: usize,
    layout: alloc::Layout,
    ptr: *mut ethtool_gstrings,
}

impl GStrings {
    fn new(length: usize) -> Result<Self, EthtoolError> {
        let data_length = length * ETH_GSTRINGS_LEN;

        // Calculate the layout with proper alignment based on the struct itself
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_gstrings>() + data_length * mem::size_of::<u8>(),
            mem::align_of::<ethtool_gstrings>(),
        )
        .map_err(|err| EthtoolError::CStructInitError(err))?;

        // Allocate memory for the struct
        let gstrings_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_gstrings;

        // Initialize the fields of the struct using raw pointers
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*gstrings_ptr).cmd);
            let ss_ptr = ptr::addr_of_mut!((*gstrings_ptr).string_set);
            let len_ptr = ptr::addr_of_mut!((*gstrings_ptr).len);
            let data_ptr = ptr::addr_of_mut!((*gstrings_ptr).data);

            cmd_ptr.write(ETHTOOL_GSTRINGS);
            ss_ptr.write(ETH_SS_STATS);
            len_ptr.write(length as u32);

            // Initialize the data field with zeros
            data_ptr.write_bytes(0u8, data_length);
        }

        Ok(GStrings {
            length,
            layout,
            ptr: gstrings_ptr,
        })
    }

    fn data(&self) -> Result<&[u8], EthtoolError> {
        unsafe {
            Ok(std::slice::from_raw_parts(
                (*self.ptr).data.as_ptr(),
                self.length * ETH_GSTRINGS_LEN,
            ))
        }
    }
}

impl Drop for GStrings {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr as *mut u8, self.layout);
        }
    }
}

struct GStats {
    length: usize,
    layout: alloc::Layout,
    ptr: *mut ethtool_stats,
}

impl GStats {
    /// Allocates memory for ethtool_stats struct and initializes it.
    fn new(length: usize) -> Result<Self, EthtoolError> {
        let data_length = length * ETH_GSTATS_LEN;

        // Calculate the layout with proper alignment
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_stats>() + data_length * mem::size_of::<u64>(),
            mem::align_of::<ethtool_stats>(),
        )
        .map_err(|err| EthtoolError::CStructInitError(err))?;

        let n_stats = length as u32;

        // Allocate memory for the struct
        let stats_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_stats;

        // Initialize the fields of the struct
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*stats_ptr).cmd);
            let n_stats_ptr = ptr::addr_of_mut!((*stats_ptr).n_stats);
            let data_ptr = ptr::addr_of_mut!((*stats_ptr).data);

            cmd_ptr.write(ETHTOOL_GSTATS);
            n_stats_ptr.write(n_stats);

            // Initialize the data field with zeros
            let data_bytes = data_length * mem::size_of::<u64>();
            data_ptr.write_bytes(0u8, data_bytes);
        }

        Ok(GStats {
            length,
            layout,
            ptr: stats_ptr,
        })
    }

    fn data(&self) -> Result<&[u64], EthtoolError> {
        unsafe {
            Ok(std::slice::from_raw_parts(
                (*self.ptr).data.as_ptr(),
                self.length * ETH_GSTATS_LEN,
            ))
        }
    }
}

impl Drop for GStats {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr as *mut u8, self.layout);
        }
    }
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
    if_name: [i8; libc::IF_NAMESIZE],
}

impl Ethtool {
    /// Get the number of stats using ETHTOOL_GSSET_INFO command
    fn gsset_info(&self) -> Result<usize, EthtoolError> {
        let sset_info = StringSetInfo::new()?;
        let data = sset_info.ptr as *mut libc::c_char;

        match ioctl(&self.sock_fd, self.if_name, data) {
            Ok(_) => Ok(sset_info.data()?),
            Err(errno) => Err(EthtoolError::SocketError(errno)),
        }
    }

    /// Get the feature names using ETHTOOL_GSTRINGS command
    fn gstrings(&self, length: usize) -> Result<Vec<String>, EthtoolError> {
        let gstrings = GStrings::new(length)?;
        let data = gstrings.ptr as *mut libc::c_char;

        match ioctl(&self.sock_fd, self.if_name, data) {
            Ok(_) => parse_names(gstrings.data()?),
            Err(errno) => Err(EthtoolError::GStringsReadError(errno)),
        }
    }

    /// Get the statistics for the features using ETHTOOL_GSTATS command
    fn gstats(&self, features: &[String]) -> Result<Vec<u64>, EthtoolError> {
        let length = features.len();
        let gstats = GStats::new(length)?;
        let data = gstats.ptr as *mut libc::c_char;

        match ioctl(&self.sock_fd, self.if_name, data) {
            Ok(_) => parse_values(gstats.data()?, length),
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
