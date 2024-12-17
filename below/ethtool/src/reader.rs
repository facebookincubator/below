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

use std::alloc;
use std::ffi::CStr;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::fd::OwnedFd;
use std::ptr;
use std::str;

use nix::errno::Errno;
use nix::libc;
use nix::sys::socket::socket;
use nix::sys::socket::AddressFamily;
use nix::sys::socket::SockFlag;
use nix::sys::socket::SockType;

use crate::errors::EthtoolError;
use crate::ethtool_sys;
const ETH_GSTATS_LEN: usize = 8;

fn if_name_bytes(if_name: &str) -> [libc::c_char; libc::IF_NAMESIZE] {
    let mut it = if_name.as_bytes().iter().copied();
    [0; libc::IF_NAMESIZE].map(|_| it.next().unwrap_or(0) as libc::c_char)
}

fn ioctl(
    fd: &OwnedFd,
    if_name: [libc::c_char; libc::IF_NAMESIZE],
    data: *mut libc::c_char,
) -> Result<(), Errno> {
    let ifr = libc::ifreq {
        ifr_name: if_name,
        ifr_ifru: libc::__c_anonymous_ifr_ifru { ifru_data: data },
    };

    // The SIOCETHTOOL conversion is necessary because POSIX (and thus libcs like musl) defines the
    // `ioctl` request argument as `c_int`, while glibc and nix::libc use `c_ulong`. In C, this
    // discrepancy is hidden behind typeless #defines, but in Rust, it becomes a type mismatch.
    //
    // By converting `libc::SIOCETHTOOL` (likely defined as `c_ulong`) to the libc's native type,
    // we ensure portability across different libc implementations. On glibc this does nothing, but
    // this conversion prevents type errors on libcs that use `c_int` for the request argument,
    // such as musl.
    #[allow(clippy::useless_conversion)]
    let exit_code = unsafe { libc::ioctl(fd.as_raw_fd(), libc::SIOCETHTOOL as _, &ifr) };
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
        .chunks(ethtool_sys::ETH_GSTRING_LEN as usize)
        .map(|chunk| {
            // Find the position of the null terminator for specific stat name
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
    let values = data.iter().take(length).copied().collect::<Vec<u64>>();

    Ok(values)
}

struct StringSetInfo {
    layout: alloc::Layout,
    ptr: *mut ethtool_sys::ethtool_sset_info,
}

impl StringSetInfo {
    /// Allocates memory for ethtool_sset_info struct and initializes it.
    fn new() -> Result<Self, EthtoolError> {
        // Calculate the layout with proper alignment
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_sys::ethtool_sset_info>(),
            mem::align_of::<ethtool_sys::ethtool_sset_info>(),
        )
        .map_err(EthtoolError::CStructInitError)?;

        // Allocate memory for the struct
        let sset_info_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_sys::ethtool_sset_info;

        // Initialize the fields of the struct
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*sset_info_ptr).cmd);
            let reserved_ptr = ptr::addr_of_mut!((*sset_info_ptr).reserved);
            let sset_mask_ptr = ptr::addr_of_mut!((*sset_info_ptr).sset_mask);
            let data_ptr = ptr::addr_of_mut!((*sset_info_ptr).data);

            cmd_ptr.write(ethtool_sys::ETHTOOL_GSSET_INFO);
            reserved_ptr.write(1u32);
            sset_mask_ptr.write((1 << ethtool_sys::ethtool_stringset_ETH_SS_STATS) as u64);

            data_ptr.write_bytes(0u8, mem::size_of::<u32>());
        }

        Ok(StringSetInfo {
            layout,
            ptr: sset_info_ptr,
        })
    }

    fn data(&self) -> Result<usize, EthtoolError> {
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
    ptr: *mut ethtool_sys::ethtool_gstrings,
}

impl GStrings {
    fn new(length: usize) -> Result<Self, EthtoolError> {
        let data_length = length * ethtool_sys::ETH_GSTRING_LEN as usize;

        // Calculate the layout with proper alignment based on the struct itself
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_sys::ethtool_gstrings>() + data_length * mem::size_of::<u8>(),
            mem::align_of::<ethtool_sys::ethtool_gstrings>(),
        )
        .map_err(EthtoolError::CStructInitError)?;

        // Allocate memory for the struct
        let gstrings_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_sys::ethtool_gstrings;
        if gstrings_ptr.is_null() {
            return Err(EthtoolError::AllocationFailure());
        }

        // Initialize the fields of the struct using raw pointers
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*gstrings_ptr).cmd);
            let ss_ptr = ptr::addr_of_mut!((*gstrings_ptr).string_set);
            let data_ptr = ptr::addr_of_mut!((*gstrings_ptr).data);

            cmd_ptr.write(ethtool_sys::ETHTOOL_GSTRINGS);
            ss_ptr.write(ethtool_sys::ethtool_stringset_ETH_SS_STATS);

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
                self.length * ethtool_sys::ETH_GSTRING_LEN as usize,
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
    ptr: *mut ethtool_sys::ethtool_stats,
}

impl GStats {
    /// Allocates memory for ethtool_stats struct and initializes it.
    fn new(length: usize) -> Result<Self, EthtoolError> {
        let data_length = length * ETH_GSTATS_LEN;

        // Calculate the layout with proper alignment
        let layout = alloc::Layout::from_size_align(
            mem::size_of::<ethtool_sys::ethtool_stats>() + data_length * mem::size_of::<u64>(),
            mem::align_of::<ethtool_sys::ethtool_stats>(),
        )
        .map_err(EthtoolError::CStructInitError)?;

        // Allocate memory for the struct
        let stats_ptr = unsafe { alloc::alloc(layout) } as *mut ethtool_sys::ethtool_stats;
        if stats_ptr.is_null() {
            return Err(EthtoolError::AllocationFailure());
        }

        // Initialize the fields of the struct
        unsafe {
            let cmd_ptr = ptr::addr_of_mut!((*stats_ptr).cmd);
            let data_ptr = ptr::addr_of_mut!((*stats_ptr).data);

            cmd_ptr.write(ethtool_sys::ETHTOOL_GSTATS);

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
    if_name: [libc::c_char; libc::IF_NAMESIZE],
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
            Ok(sock_fd) => Ok(Ethtool {
                sock_fd,
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
