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

#![deny(clippy::all)]

use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use bitflags::bitflags;
use common::fileutil::get_dir_size;
use common::open_source_shim;
use common::util::get_unix_timestamp;
use model::Model;
use serde::Deserialize;
use serde::Serialize;
use slog::info;
use slog::warn;
use static_assertions::const_assert_eq;

use crate::compression::Compressor;
use crate::cursor::KeyedCursor;
use crate::cursor::StoreCursor;

pub mod advance;
pub mod compression;
pub mod cursor;
#[cfg(test)]
mod test;

pub type Advance = advance::Advance<DataFrame, Model>;

open_source_shim!();

/// This data store works as follows:
///
/// Each data item (e.g. DataFrame) is simply appended to a data file.
///
/// An IndexEntry is appended to a corresponding index file. Each
/// IndexEntry contains the timestamp (e.g. key) of the data item, its
/// offset into the data file, the length of the data entry, and a CRC
/// of the data entry as well as a CRC of itself. It also contains
/// flags that can indicate if the corresponding data is compressed
/// and how it is compressed.
///
/// The CRCs in the index entry give us an atomicity guarantee - if
/// they are not present and correct, we treat it as if the entry
/// never existed.
///
/// In dictionary compression mode, the index file may be padded with
/// zeros (i.e. empty index entries). Thus empty index entries are
/// not considered to be corrupt, but we ignore such entries as they
/// do not point to any data.
///
/// Data and Index files are append-only and never modified (only ever
/// removed).
///
/// Data and Index files are sharded by SHARD_TIME - e.g. any one file
/// only contains data or index entries whose timestamps are congruent
/// modulo SHARD_TIME. This allows data and index files to be cleaned
/// up by just unlinking the files.

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DataFrame {
    pub sample: model::Sample,
}

const SHARD_TIME: u64 = 24 * 60 * 60;

// Number of bits used by other bit flags in IndexEntry before the
// chunk compress flags.
const CHUNK_COMPRESS_SHIFT: u32 = 2;

const MAX_CHUNK_COMPRESS_SIZE_PO2: u32 = 0x0F;
pub const MAX_CHUNK_COMPRESS_SIZE: u32 = 1 << MAX_CHUNK_COMPRESS_SIZE_PO2;
const_assert_eq!(MAX_CHUNK_COMPRESS_SIZE, 32768);

bitflags! {
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    struct IndexEntryFlags: u32 {
        /// If set, data item is compressed with zstd.
        const COMPRESSED = 0x1;
        /// If set, data item is serialized as CBOR. If unset,
        /// serialization is set to the default (also CBOR in the
        /// case of open source build).
        const CBOR = 0x2;
        /// If `COMPRESSED` is set `CHUNK_COMPRESS_SIZE_PO2` is
        /// non-zero, then zstd dictionary compression is used.
        /// Data is return in "chunks" of size
        /// `2^CHUNK_COMPRESS_SIZE_PO2` entries. The first entry of
        /// each chunk, in its uncompressed form, is used as the zstd
        /// dictionary for the rest of the chunk.
        ///
        /// Chunks are aligned to chunk size. The index is padded
        /// with empty index entries as necessary. For example, if
        /// below is started with a chunk size of 4, and the index
        /// has 5 entries, then the index will be zero-padded to the
        /// length of 8 entries before the first dict key frame is
        /// written. Padding occurs on a restart in recording, but
        /// can also occur if possible data corruption has been
        /// detected.
        const CHUNK_COMPRESS_SIZE_PO2 = MAX_CHUNK_COMPRESS_SIZE_PO2 << CHUNK_COMPRESS_SHIFT;
    }
}

impl IndexEntryFlags {
    fn get_chunk_compress_size_po2(&self) -> u32 {
        (self.bits() & Self::CHUNK_COMPRESS_SIZE_PO2.bits()) >> CHUNK_COMPRESS_SHIFT
    }

    fn set_chunk_compress_size_po2(&mut self, chunk_compress_size_po2: u32) -> Result<()> {
        if chunk_compress_size_po2 > MAX_CHUNK_COMPRESS_SIZE_PO2 {
            bail!(
                "Chunk compress size po2 should be less than or equal to {}",
                MAX_CHUNK_COMPRESS_SIZE_PO2
            );
        }
        *self |= IndexEntryFlags::from_bits_retain(chunk_compress_size_po2 << CHUNK_COMPRESS_SHIFT);
        Ok(())
    }
}

#[repr(C)]
struct IndexEntry {
    /// Timestamp of the data entry
    timestamp: u64,
    /// Offset into the data file that this entry exists at
    offset: u64,
    /// Length of the data entry
    len: u32,
    /// Flags for this data entry
    flags: IndexEntryFlags,
    /// crc32 of the data entry
    data_crc: u32,
    /// crc32 of this entry (e.g. crc32 of all the above members)
    index_crc: u32,
}

const INDEX_ENTRY_SIZE: usize = std::mem::size_of::<IndexEntry>();
const INDEX_ENTRY_SIZE_PO2: u32 = INDEX_ENTRY_SIZE.trailing_zeros();
const_assert_eq!(INDEX_ENTRY_SIZE, 32);

#[derive(Copy, Clone, Debug)]
pub struct ChunkSizePo2(pub u32);

#[derive(Copy, Clone, Debug)]
pub enum CompressionMode {
    None,
    Zstd,
    ZstdDictionary(ChunkSizePo2),
}

/// The StoreWriter struct maintains state to put more data in the
/// store. It keeps track of the index and data file it's currently
/// working on so in the common case it can just append data. When it
/// rolls over to a new shard, it will recreate itself.
// #[derive(Debug)]
pub struct StoreWriter {
    logger: slog::Logger,
    /// Directory of the store itself
    dir: PathBuf,
    /// Currently active index file. Append only so cursor always
    /// points to end of file.
    index: File,
    /// Currently active data file. Append only so cursor always
    /// point to end of file.
    data: File,
    /// Current length of the data file (needed to record offsets in
    /// the index)
    data_len: u64,
    /// Active shard
    shard: u64,
    /// Cached compressor for memory efficiency. Compressor also stores key
    /// frame for dict compression.
    compressor: Option<Compressor>,
    /// If non-empty, individual frames are compressed with
    /// `compression_mode`.
    compression_mode: CompressionMode,
    /// Serialization format of data frames
    format: Format,
}

// Given path to the store dir, get a Vec<String> of the index file
// names, sorted
fn get_index_files(path: &Path) -> Result<Vec<String>> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory {}", path.display()))?
        .filter_map(|res_ent| {
            res_ent
                .map(|ent| {
                    ent.file_name()
                        .to_str()
                        .filter(|s| s.starts_with("index"))
                        .map(|s| s.to_string())
                })
                .transpose()
        })
        .collect::<Result<Vec<_>, std::io::Error>>()
        .with_context(|| format!("Failed to read directory entries in {}", path.display()))?;

    entries.sort_unstable();
    Ok(entries)
}

enum SerializedFrame<'a> {
    Owned(bytes::Bytes),
    Borrowed(&'a [u8]),
}

impl AsRef<[u8]> for SerializedFrame<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            SerializedFrame::Owned(b) => b.as_ref(),
            SerializedFrame::Borrowed(s) => s,
        }
    }
}

impl SerializedFrame<'_> {
    fn into_owned(self) -> bytes::Bytes {
        match self {
            SerializedFrame::Owned(b) => b,
            SerializedFrame::Borrowed(s) => bytes::Bytes::copy_from_slice(s),
        }
    }
}

/// Serialization format. Currently only Cbor is supported.
#[derive(Copy, Clone, Debug)]
pub enum Format {
    Cbor,
}

/// Serialize a single data frame with `format` format.
fn serialize_frame(data: &DataFrame, format: Format) -> Result<bytes::Bytes> {
    match format {
        Format::Cbor => {
            let bytes = serde_cbor::to_vec(data)?;
            Ok(bytes::Bytes::from(bytes))
        }
    }
}

/// Deserialize a single data frame with `format` format.
fn deserialize_frame(bytes: &[u8], format: Format) -> Result<DataFrame> {
    match format {
        Format::Cbor => {
            let data_frame = serde_cbor::from_slice(bytes)?;
            Ok(data_frame)
        }
    }
}

impl StoreWriter {
    /// Create a new `StoreWriter` that writes data to `path`
    /// directory. Data serialized with `format`.
    ///
    /// If `compression_mode` is set, dataframes are zstd compressed,
    /// as defined by `compression_mode`.
    pub fn new<P: AsRef<Path>>(
        logger: slog::Logger,
        path: P,
        compression_mode: CompressionMode,
        format: Format,
    ) -> Result<Self> {
        Self::new_with_timestamp(logger, path, SystemTime::now(), compression_mode, format)
    }

    pub fn new_with_timestamp<P: AsRef<Path>>(
        logger: slog::Logger,
        path: P,
        timestamp: SystemTime,
        compression_mode: CompressionMode,
        format: Format,
    ) -> Result<Self> {
        let shard = calculate_shard(timestamp);
        Self::new_with_shard(logger, path, shard, compression_mode, format)
    }

    fn new_with_shard<P: AsRef<Path>>(
        logger: slog::Logger,
        path: P,
        shard: u64,
        compression_mode: CompressionMode,
        format: Format,
    ) -> Result<Self> {
        if !path.as_ref().is_dir() {
            std::fs::create_dir(&path).with_context(|| {
                format!("Failed to create store path: {}", path.as_ref().display())
            })?;
        }

        let (data_path, index_path) = {
            let mut data_path = path.as_ref().to_path_buf();
            let mut index_path = data_path.clone();
            data_path.push(format!("data_{:011}", shard));
            index_path.push(format!("index_{:011}", shard));
            (data_path, index_path)
        };

        let index = OpenOptions::new()
            .append(true)
            .create(true)
            .open(index_path.as_path())
            .with_context(|| format!("Failed to open index file: {}", index_path.display()))?;
        nix::fcntl::flock(
            index.as_raw_fd(),
            nix::fcntl::FlockArg::LockExclusiveNonblock,
        )
        .with_context(|| {
            format!(
                "Failed to acquire file lock on index file: {}",
                index_path.display(),
            )
        })?;

        let data = OpenOptions::new()
            .append(true)
            .create(true)
            .open(data_path.as_path())
            .with_context(|| format!("Failed to open data file: {}", data_path.display()))?;
        nix::fcntl::flock(
            data.as_raw_fd(),
            nix::fcntl::FlockArg::LockExclusiveNonblock,
        )
        .with_context(|| {
            format!(
                "Failed to acquire file lock on data file: {}",
                data_path.display(),
            )
        })?;

        let data_len = data
            .metadata()
            .with_context(|| {
                format!(
                    "Failed to get metadata of data file: {}",
                    data_path.display()
                )
            })?
            .len();

        Ok(StoreWriter {
            logger,
            dir: path.as_ref().to_path_buf(),
            index,
            data,
            data_len,
            shard,
            // First compressed write initializes the compressor
            compressor: None,
            compression_mode,
            format,
        })
    }

    /// The index file is padded to the next (1 << alignment_po2) aligned
    /// boundary. Both the original and aligned lengths are then returned.
    /// Mostly used to align index file with INDEX_ENTRY_SIZE or chunk size if
    /// dictionary compression is used. Misalignment can happen if we partially
    /// wrote an index entry, a new chunk must be used, an external actor
    /// modified the index file, etc.
    fn pad_and_get_index_len(index: &mut File, alignment_po2: u32) -> Result<(u64, u64)> {
        let index_len = index
            .metadata()
            .context("Failed to get metadata of index file")?
            .len();
        let alignment_mask = (1 << alignment_po2) - 1;
        let aligned_len = (index_len + alignment_mask) & !alignment_mask;
        if aligned_len != index_len {
            index
                .set_len(aligned_len)
                .context("Failed to pad index file")?;
            // Since file is opened as append only, we don't need to
            // move the cursor to end of file.
        }
        Ok((index_len, aligned_len))
    }

    /// For the given `DataFrame` and an optional Compressor mut ref, returns a
    /// tuple consisting of:
    ///
    ///   1) Raw bytes to write to the data file
    ///   2) Flags to write to the index entry
    ///
    /// For compressed write, the Compressor will be initialized if None, and
    /// potentially updated. is_key_frame is used to indicate the start of a new
    /// chunk if dictionary compression is enabled.
    fn get_bytes_and_flags_for_frame(
        &self,
        data_frame: &DataFrame,
        compressor: &mut Option<Compressor>,
        is_key_frame: bool,
    ) -> Result<(bytes::Bytes, IndexEntryFlags)> {
        let mut flags = match self.format {
            Format::Cbor => IndexEntryFlags::CBOR,
        };
        // Get serialized data frame
        let frame_bytes =
            serialize_frame(data_frame, self.format).context("Failed to serialize data frame")?;
        let serialized = match self.compression_mode {
            CompressionMode::None => frame_bytes,
            CompressionMode::Zstd => {
                flags |= IndexEntryFlags::COMPRESSED;
                compressor
                    .get_or_insert_with(Compressor::new)
                    .compress_with_dict_reset(&frame_bytes)
                    .context("Failed to compress data")?
            }
            CompressionMode::ZstdDictionary(ChunkSizePo2(chunk_size_po2)) => {
                flags |= IndexEntryFlags::COMPRESSED;
                flags
                    .set_chunk_compress_size_po2(chunk_size_po2)
                    .expect("bug: invalid chunk compress size");
                let compressor = compressor.get_or_insert_with(Compressor::new);
                if is_key_frame {
                    let serialized = compressor
                        .compress_with_dict_reset(&frame_bytes)
                        .context("Failed to compress key frame")?;
                    compressor
                        .load_dict(&frame_bytes)
                        .context("Failed to set key frame as dict")?;
                    serialized
                } else {
                    compressor
                        .compress_with_loaded_dict(&frame_bytes)
                        .context("Failed to compress data frame")?
                }
            }
        };
        Ok((serialized, flags))
    }

    /// Store data with corresponding timestamp in current shard.
    /// Fails if data does not belong to current shard. Errors may be
    /// returned if file operations fail.
    fn put_in_current_shard(&mut self, timestamp: SystemTime, data: &DataFrame) -> Result<()> {
        let shard = calculate_shard(timestamp);
        if shard != self.shard {
            panic!("Can't write data to shard as it belongs to different shard")
        }

        // PO2 chunk size in bytes if dict compression is used, otherwise 0.
        let chunk_alignment_po2 =
            if let CompressionMode::ZstdDictionary(ChunkSizePo2(chunk_size_po2)) =
                self.compression_mode
            {
                // chunk_size_po2 is in number of entries. Add with entry size
                // po2 to get size in bytes po2.
                chunk_size_po2 + INDEX_ENTRY_SIZE_PO2
            } else {
                0
            };
        // If dict compression is used but Compressor uninitialized, e.g. new
        // shard, previous write failed, then pad index to start a new chunk.
        // Otherwise pad to ensure index file is aligned with INDEX_ENTRY_SIZE.
        let alignment_po2 = if chunk_alignment_po2 != 0 && self.compressor.is_none() {
            chunk_alignment_po2
        } else {
            INDEX_ENTRY_SIZE_PO2
        };
        let (index_len, aligned_len) = Self::pad_and_get_index_len(&mut self.index, alignment_po2)
            .with_context(|| {
                format!(
                    "Failed to get index length and possibly pad index file: index_{:011}",
                    shard
                )
            })?;
        if index_len != aligned_len {
            if alignment_po2 == INDEX_ENTRY_SIZE_PO2 {
                warn!(
                    self.logger,
                    "Index length not a multiple of fixed index entry size: {}. Padded to size: {}",
                    index_len,
                    aligned_len,
                );
            } else if alignment_po2 == chunk_alignment_po2 {
                // Always happen when below restarts. Thus log with info level
                info!(
                    self.logger,
                    "Padded index so that first entry of block is aligned. Previous len: {}. New len: {}",
                    index_len,
                    aligned_len,
                );
            } else {
                panic!("Unexpected alignment_po2 value");
            }
        }

        // Take the compressor from self before modifying it. In case any write
        // failure occurs, the old compressor (potentially in bad state) will be
        // discarded and a new one be created in the next write. No-op if
        // compression is not used.
        let mut compressor = self.compressor.take();
        // If dict compression is used and the index file is chunk aligned, the
        // current frame is the key frame.
        let is_key_frame =
            chunk_alignment_po2 != 0 && aligned_len.trailing_zeros() >= chunk_alignment_po2;
        let (serialized, flags) = self
            .get_bytes_and_flags_for_frame(data, &mut compressor, is_key_frame)
            .context("Failed to get serialized frame and flags")?;

        // Appends to data file are large and cannot be atomic. We
        // may have partial writes that increase file size without
        // updating the stored state. Thus always read actual data
        // file length. This is less of an issue for the index file
        // but we track it anyway.
        let data_len = self
            .data
            .metadata()
            .with_context(|| {
                format!(
                    "Failed to get metadata of data file: data_{:011}",
                    self.shard
                )
            })?
            .len();
        // Warn potential data file corruption
        if self.data_len != data_len {
            warn!(
                self.logger,
                "Data length mismatch: {} (expect {})", data_len, self.data_len
            );
            self.data_len = data_len;
        }

        let offset = self.data_len;
        // It doesn't really matter which order we write the data in,
        // most filesystems do not provide ordering guarantees for
        // appends to different files anyways. We just need to handle
        // various failure cases on the read side.
        self.data
            .write_all(&serialized)
            .context("Failed to write entry to data file")?;
        self.data_len += serialized.len() as u64;
        let data_crc = serialized.crc32();

        let mut index_entry = IndexEntry {
            timestamp: get_unix_timestamp(timestamp),
            offset,
            flags,
            len: serialized
                .len()
                .try_into()
                .with_context(|| format!("Serialized len={} overflows u32", serialized.len()))?,
            data_crc,
            index_crc: 0,
        };
        index_entry.index_crc = index_entry.crc32();
        {
            // unsafe to turn this into a slice - we need this to write it though
            let entry_slice = unsafe {
                std::slice::from_raw_parts(
                    &index_entry as *const IndexEntry as *const u8,
                    INDEX_ENTRY_SIZE,
                )
            };
            self.index
                .write_all(entry_slice)
                .context("Failed to write entry to index file")?;
        }

        // Set compressor only after successful writes. No-op if not in
        // compression mode
        self.compressor = compressor;
        Ok(())
    }

    /// Store data with corresponding timestamp. Returns true if a new shard
    /// is created and data is written successfully. Errors may be returned if
    /// file operations fail.
    pub fn put(&mut self, timestamp: SystemTime, data: &DataFrame) -> Result<bool> {
        let shard = calculate_shard(timestamp);
        if shard != self.shard {
            // We just recreate the StoreWriter since this is a new shard
            let mut writer = Self::new_with_shard(
                self.logger.clone(),
                self.dir.as_path(),
                shard,
                self.compression_mode,
                self.format,
            )?;
            // Set self to new shard only if we succeed in writing the first
            // frame. If we don't do this, we may "forget" returning a true
            // for a new shard where the first write fails.
            writer.put_in_current_shard(timestamp, data)?;
            *self = writer;
            Ok(true)
        } else {
            self.put_in_current_shard(timestamp, data)?;
            Ok(false)
        }
    }

    /// Discard shards from the oldest first until f(shard_timestamp) is true
    /// or we've reached the current shard. Returns true if f(shard_timestamp)
    /// is true for the last shard visited or false otherwise.
    fn discard_until<F>(&self, f: F) -> Result<bool>
    where
        F: Fn(u64) -> bool,
    {
        let entries = get_index_files(self.dir.as_path())?;

        // Entries are sorted with increasing timestamp
        for entry in entries {
            let v: Vec<&str> = entry.split('_').collect();
            if v.len() != 2 {
                warn!(self.logger, "Invalid index file name: {}", entry);
                continue;
            }

            let entry_shard = match v[1].parse::<u64>() {
                Ok(val) => val,
                _ => {
                    warn!(self.logger, "Cannot parse index shard: {}", entry);
                    continue;
                }
            };

            if f(entry_shard) {
                return Ok(true);
            }
            if entry_shard >= self.shard {
                return Ok(false);
            }

            // Removal order doesn't matter at all, it's the
            // responsibility of the read side to handle missing files
            let mut index_path = self.dir.clone();
            index_path.push(&entry);

            match std::fs::remove_file(&index_path) {
                Err(e) if e.kind() != ErrorKind::NotFound => {
                    return Err(e).context(format!(
                        "Failed to remove index file: {}",
                        index_path.display()
                    ));
                }
                _ => {}
            };

            let mut data_path = self.dir.clone();
            data_path.push(format!("data_{:011}", entry_shard));

            match std::fs::remove_file(&data_path) {
                Err(e) if e.kind() != ErrorKind::NotFound => {
                    return Err(e).context(format!(
                        "Failed to remove data file: {}",
                        data_path.display()
                    ));
                }
                _ => {}
            };
        }
        Ok(false)
    }

    /// Discard all data earlier than timestamp
    ///
    /// We do not modify index and data files. We just look for files
    /// which can only contain earlier data and remove them.
    pub fn discard_earlier(&self, timestamp: SystemTime) -> Result<()> {
        let shard = calculate_shard(timestamp);
        self.discard_until(|shard_timestamp| shard_timestamp >= shard)?;
        Ok(())
    }

    /// Discard data until store size is less than limit, or there is only one
    /// shard left. Oldest shards are discarded first. Returns true on success
    /// or false if the current shard size is greater than the limit.
    pub fn try_discard_until_size(&self, store_size_limit: u64) -> Result<bool> {
        let dir = self.dir.clone();
        self.discard_until(|_| {
            let size = get_dir_size(dir.clone());
            size <= store_size_limit
        })
    }
}

/// Direction to scan for next sample
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Reverse,
}

impl Direction {
    pub fn get_skip_order(&self) -> std::cmp::Ordering {
        match self {
            Direction::Forward => std::cmp::Ordering::Less,
            Direction::Reverse => std::cmp::Ordering::Greater,
        }
    }

    pub fn flip(&self) -> Self {
        match self {
            Direction::Forward => Direction::Reverse,
            Direction::Reverse => Direction::Forward,
        }
    }
}

/// Convenient function to read the first sample at timestamp or after
/// timestamp in direction. Prefer directly using StoreCursor for sequential
/// reads
pub fn read_next_sample<P: AsRef<Path>>(
    path: P,
    timestamp: SystemTime,
    direction: Direction,
    logger: slog::Logger,
) -> Result<Option<(SystemTime, DataFrame)>> {
    let mut cursor = cursor::StoreCursor::new(logger, path.as_ref().to_path_buf());
    cursor.get_next(&get_unix_timestamp(timestamp), direction)
}

pub trait Store: Send + Sync {
    // We intentionally make this trait generic which not tied to the DataFrame and Model
    // type for ease of testing.
    // For LocalStore and RemoteStore, SampleType will be DataFrame
    // For FakeStore, SampleType will be u64
    type SampleType;

    /// Return the sample time and data frame. Needs to be implemented by
    /// all stores.
    // This function should return the data sample at the provided timestamp.
    // If no sample available at the given timestamp, it will return the
    // first sample after the timestamp if the direction is forward. Otherwise
    // it will return the last sample before the timestamp. This function should
    // return None in the following situation:
    // * reverse search a target that has timestamp earlier than the first recorded
    //   sample
    // * forward search a target that has timestamp later than the last recorded
    //   sample
    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Self::SampleType)>>;
}

pub struct LocalStore {
    store_cursor: StoreCursor,
}

pub struct RemoteStore {
    store: crate::remote_store::RemoteStore,
}

impl LocalStore {
    pub fn new(logger: slog::Logger, dir: PathBuf) -> Self {
        Self {
            store_cursor: StoreCursor::new(logger, dir),
        }
    }
}

impl RemoteStore {
    pub fn new(host: String, port: Option<u16>) -> Result<Self> {
        Ok(Self {
            store: crate::remote_store::RemoteStore::new(host, port)?,
        })
    }
}

impl Store for LocalStore {
    type SampleType = DataFrame;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        self.store_cursor
            .get_next(&get_unix_timestamp(timestamp), direction)
    }
}

impl Store for RemoteStore {
    type SampleType = DataFrame;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        self.store
            .get_frame(get_unix_timestamp(timestamp), direction)
    }
}

trait Crc32 {
    fn crc32(&self) -> u32;
}

/// Lookup table for byte-by-byte crc32 computation
const CRC32_TABLE: [u32; 256] = [
    0, 0x77073096, 0xEE0E612C, 0x990951BA, 0x076DC419, 0x706AF48F, 0xE963A535, 0x9E6495A3,
    0x0EDB8832, 0x79DCB8A4, 0xE0D5E91E, 0x97D2D988, 0x09B64C2B, 0x7EB17CBD, 0xE7B82D07, 0x90BF1D91,
    0x1DB71064, 0x6AB020F2, 0xF3B97148, 0x84BE41DE, 0x1ADAD47D, 0x6DDDE4EB, 0xF4D4B551, 0x83D385C7,
    0x136C9856, 0x646BA8C0, 0xFD62F97A, 0x8A65C9EC, 0x14015C4F, 0x63066CD9, 0xFA0F3D63, 0x8D080DF5,
    0x3B6E20C8, 0x4C69105E, 0xD56041E4, 0xA2677172, 0x3C03E4D1, 0x4B04D447, 0xD20D85FD, 0xA50AB56B,
    0x35B5A8FA, 0x42B2986C, 0xDBBBC9D6, 0xACBCF940, 0x32D86CE3, 0x45DF5C75, 0xDCD60DCF, 0xABD13D59,
    0x26D930AC, 0x51DE003A, 0xC8D75180, 0xBFD06116, 0x21B4F4B5, 0x56B3C423, 0xCFBA9599, 0xB8BDA50F,
    0x2802B89E, 0x5F058808, 0xC60CD9B2, 0xB10BE924, 0x2F6F7C87, 0x58684C11, 0xC1611DAB, 0xB6662D3D,
    0x76DC4190, 0x01DB7106, 0x98D220BC, 0xEFD5102A, 0x71B18589, 0x06B6B51F, 0x9FBFE4A5, 0xE8B8D433,
    0x7807C9A2, 0x0F00F934, 0x9609A88E, 0xE10E9818, 0x7F6A0DBB, 0x086D3D2D, 0x91646C97, 0xE6635C01,
    0x6B6B51F4, 0x1C6C6162, 0x856530D8, 0xF262004E, 0x6C0695ED, 0x1B01A57B, 0x8208F4C1, 0xF50FC457,
    0x65B0D9C6, 0x12B7E950, 0x8BBEB8EA, 0xFCB9887C, 0x62DD1DDF, 0x15DA2D49, 0x8CD37CF3, 0xFBD44C65,
    0x4DB26158, 0x3AB551CE, 0xA3BC0074, 0xD4BB30E2, 0x4ADFA541, 0x3DD895D7, 0xA4D1C46D, 0xD3D6F4FB,
    0x4369E96A, 0x346ED9FC, 0xAD678846, 0xDA60B8D0, 0x44042D73, 0x33031DE5, 0xAA0A4C5F, 0xDD0D7CC9,
    0x5005713C, 0x270241AA, 0xBE0B1010, 0xC90C2086, 0x5768B525, 0x206F85B3, 0xB966D409, 0xCE61E49F,
    0x5EDEF90E, 0x29D9C998, 0xB0D09822, 0xC7D7A8B4, 0x59B33D17, 0x2EB40D81, 0xB7BD5C3B, 0xC0BA6CAD,
    0xEDB88320, 0x9ABFB3B6, 0x03B6E20C, 0x74B1D29A, 0xEAD54739, 0x9DD277AF, 0x04DB2615, 0x73DC1683,
    0xE3630B12, 0x94643B84, 0x0D6D6A3E, 0x7A6A5AA8, 0xE40ECF0B, 0x9309FF9D, 0x0A00AE27, 0x7D079EB1,
    0xF00F9344, 0x8708A3D2, 0x1E01F268, 0x6906C2FE, 0xF762575D, 0x806567CB, 0x196C3671, 0x6E6B06E7,
    0xFED41B76, 0x89D32BE0, 0x10DA7A5A, 0x67DD4ACC, 0xF9B9DF6F, 0x8EBEEFF9, 0x17B7BE43, 0x60B08ED5,
    0xD6D6A3E8, 0xA1D1937E, 0x38D8C2C4, 0x4FDFF252, 0xD1BB67F1, 0xA6BC5767, 0x3FB506DD, 0x48B2364B,
    0xD80D2BDA, 0xAF0A1B4C, 0x36034AF6, 0x41047A60, 0xDF60EFC3, 0xA867DF55, 0x316E8EEF, 0x4669BE79,
    0xCB61B38C, 0xBC66831A, 0x256FD2A0, 0x5268E236, 0xCC0C7795, 0xBB0B4703, 0x220216B9, 0x5505262F,
    0xC5BA3BBE, 0xB2BD0B28, 0x2BB45A92, 0x5CB36A04, 0xC2D7FFA7, 0xB5D0CF31, 0x2CD99E8B, 0x5BDEAE1D,
    0x9B64C2B0, 0xEC63F226, 0x756AA39C, 0x026D930A, 0x9C0906A9, 0xEB0E363F, 0x72076785, 0x05005713,
    0x95BF4A82, 0xE2B87A14, 0x7BB12BAE, 0x0CB61B38, 0x92D28E9B, 0xE5D5BE0D, 0x7CDCEFB7, 0x0BDBDF21,
    0x86D3D2D4, 0xF1D4E242, 0x68DDB3F8, 0x1FDA836E, 0x81BE16CD, 0xF6B9265B, 0x6FB077E1, 0x18B74777,
    0x88085AE6, 0xFF0F6A70, 0x66063BCA, 0x11010B5C, 0x8F659EFF, 0xF862AE69, 0x616BFFD3, 0x166CCF45,
    0xA00AE278, 0xD70DD2EE, 0x4E048354, 0x3903B3C2, 0xA7672661, 0xD06016F7, 0x4969474D, 0x3E6E77DB,
    0xAED16A4A, 0xD9D65ADC, 0x40DF0B66, 0x37D83BF0, 0xA9BCAE53, 0xDEBB9EC5, 0x47B2CF7F, 0x30B5FFE9,
    0xBDBDF21C, 0xCABAC28A, 0x53B39330, 0x24B4A3A6, 0xBAD03605, 0xCDD70693, 0x54DE5729, 0x23D967BF,
    0xB3667A2E, 0xC4614AB8, 0x5D681B02, 0x2A6F2B94, 0xB40BBE37, 0xC30C8EA1, 0x5A05DF1B, 0x2D02EF8D,
];

impl Crc32 for [u8] {
    fn crc32(&self) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for byte in self {
            crc = (crc >> 8) ^ CRC32_TABLE[((crc & 0xFF) as u8 ^ *byte) as usize];
        }
        crc
    }
}

impl Crc32 for IndexEntry {
    fn crc32(&self) -> u32 {
        let slice = unsafe {
            std::slice::from_raw_parts(
                self as *const IndexEntry as *const u8,
                // Make sure to ignore the index_crc itself for this
                INDEX_ENTRY_SIZE - std::mem::size_of::<u32>(),
            )
        };
        slice.crc32()
    }
}

// This is the timestamp rounded down to the nearest
// multiple of SHARD_TIME
fn calculate_shard(timestamp: SystemTime) -> u64 {
    let timestamp_secs = get_unix_timestamp(timestamp);
    let shard_rem = timestamp_secs % SHARD_TIME;
    timestamp_secs - shard_rem
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use paste::paste;
    use slog::Drain;
    use tempfile::TempDir;

    use super::*;

    fn get_logger() -> slog::Logger {
        let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
        slog::Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!())
    }

    // Asserts that a and b are equal, to the resolution of one second
    macro_rules! assert_ts {
        ($a:expr, $b:expr) => {
            let a_dur = $a
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Timestamp earlier than UNIX EPOCH");
            let b_dur = $b
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Timestamp earlier than UNIX EPOCH");
            assert_eq!(a_dur.as_secs(), b_dur.as_secs());
        };
    }

    macro_rules! store_test {
        ($name:ident, $func:ident) => {
            paste! {
                #[test]
                fn [<$name _uncompressed_cbor>]() {
                    $func(CompressionMode::None, Format::Cbor);
                }
            }

            paste! {
                #[test]
                fn [<$name _compressed_cbor>]() {
                    $func(CompressionMode::Zstd, Format::Cbor);
                }
            }

            paste! {
                #[test]
                fn [<$name _dict_compressed_cbor>]() {
                    $func(CompressionMode::ZstdDictionary(ChunkSizePo2(2)), Format::Cbor);
                }
            }
        };
    }

    #[test]
    fn writing_to_already_written_index_with_different_compression_format_works() {
        use itertools::Itertools;

        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);

        // States, (compression_mode, format), that we transition between when
        // writing
        let states = [
            (CompressionMode::None, Format::Cbor),
            (CompressionMode::Zstd, Format::Cbor),
            (
                CompressionMode::ZstdDictionary(ChunkSizePo2(0)),
                Format::Cbor,
            ),
            (
                CompressionMode::ZstdDictionary(ChunkSizePo2(1)),
                Format::Cbor,
            ),
            (
                CompressionMode::ZstdDictionary(ChunkSizePo2(2)),
                Format::Cbor,
            ),
            (
                CompressionMode::ZstdDictionary(ChunkSizePo2(3)),
                Format::Cbor,
            ),
        ];
        // State sequence that contains all possible transitions
        let state_sequence = states
            .iter()
            .cartesian_product(states.iter())
            .flat_map(|(a, b)| vec![a, b])
            .collect::<Vec<_>>();

        for (i, (compression_mode, format)) in state_sequence.iter().enumerate() {
            let mut writer = StoreWriter::new(get_logger(), &dir, *compression_mode, *format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(i as i64);

            writer
                .put(ts + Duration::from_secs(i as u64), &frame)
                .expect("Failed to store data");
        }

        // Test reading all the samples
        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        for (i, (_compress, _format)) in state_sequence.iter().enumerate() {
            let frame = store_cursor
                .get_next(
                    &get_unix_timestamp(ts + Duration::from_secs(i as u64)),
                    Direction::Forward,
                )
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(i as u64));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(i as i64));
        }
    }

    #[test]
    fn write_index_padding() {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        // Keep test on one shard
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        // Write 1 frame without compression. Doesn't add padding.
        {
            let mut writer =
                StoreWriter::new(get_logger(), &dir, CompressionMode::None, Format::Cbor)
                    .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 0..1 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
            assert_eq!(
                writer.index.metadata().unwrap().len(),
                INDEX_ENTRY_SIZE as u64
            );
        }

        // Write 2 frames with without compression. Doesn't add padding.
        {
            let mut writer =
                StoreWriter::new(get_logger(), &dir, CompressionMode::None, Format::Cbor)
                    .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 1..3 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
            assert_eq!(
                writer.index.metadata().unwrap().len(),
                3 * INDEX_ENTRY_SIZE as u64
            );
        }

        // Write 2 frames with compression. Doesn't add padding.
        {
            let mut writer =
                StoreWriter::new(get_logger(), &dir, CompressionMode::Zstd, Format::Cbor)
                    .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 3..5 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
            assert_eq!(
                writer.index.metadata().unwrap().len(),
                5 * INDEX_ENTRY_SIZE as u64
            );
        }

        // Dict compress with chunk size of 4. Current size of 5 so
        // need to pad by 3.
        {
            let mut writer = StoreWriter::new(
                get_logger(),
                &dir,
                CompressionMode::ZstdDictionary(ChunkSizePo2(2)),
                Format::Cbor,
            )
            .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 5..13 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
            assert_eq!(
                writer.index.metadata().unwrap().len(),
                16 * INDEX_ENTRY_SIZE as u64
            );
        }

        // Dict compress with chunk size of 8. Current size of 16 so
        // no padding needed.
        {
            let mut writer = StoreWriter::new(
                get_logger(),
                &dir,
                CompressionMode::ZstdDictionary(ChunkSizePo2(3)),
                Format::Cbor,
            )
            .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 13..16 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
            assert_eq!(
                writer.index.metadata().unwrap().len(),
                19 * INDEX_ENTRY_SIZE as u64
            );
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        for i in 0..16 {
            let frame = store_cursor
                .get_next(
                    &get_unix_timestamp(ts + Duration::from_secs(i as u64)),
                    Direction::Forward,
                )
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(i as u64));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(i));
        }
    }

    store_test!(create_writer, _create_writer);
    fn _create_writer(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        StoreWriter::new(get_logger(), &dir, compression_mode, format)
            .expect("Failed to create store");
    }

    store_test!(simple_put_read, _simple_put_read);
    fn _simple_put_read(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    store_test!(simple_put_read_10, _simple_put_read_10);
    fn _simple_put_read_10(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        // Keep test on one shard
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            for i in 0..10 {
                frame.sample.cgroup.memory_current = Some(i);
                writer
                    .put(ts + Duration::from_secs(i as u64), &frame)
                    .expect("Failed to store data");
            }
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        for i in 0..10 {
            let frame = store_cursor
                .get_next(
                    &get_unix_timestamp(ts + Duration::from_secs(i as u64)),
                    Direction::Forward,
                )
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(i as u64));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(i));
        }
    }

    store_test!(put_new_shard, _put_new_shard);
    fn _put_new_shard(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let now = SystemTime::now();
        // Ensure that the follow writes (within 60s) are to the same shard
        let ts = if calculate_shard(now) == calculate_shard(now + Duration::from_secs(60)) {
            now
        } else {
            now + Duration::from_secs(60)
        };

        {
            let mut writer =
                StoreWriter::new_with_timestamp(get_logger(), &dir, ts, compression_mode, format)
                    .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(111);

            // New StoreWriter, but we're not switching to new shard
            assert!(!writer.put(ts, &frame).expect("Failed to store data"));

            frame.sample.cgroup.memory_current = Some(222);

            // No new shard
            assert!(
                !writer
                    .put(ts + Duration::from_secs(1), &frame)
                    .expect("Failed to store data")
            );

            frame.sample.cgroup.memory_current = Some(333);

            // New shard
            assert!(
                writer
                    .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                    .expect("Failed to store data")
            );
        }

        {
            let mut writer = StoreWriter::new_with_timestamp(
                get_logger(),
                &dir,
                ts + Duration::from_secs(SHARD_TIME + 1),
                compression_mode,
                format,
            )
            .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(444);

            // New StoreWriter but writing to existing shard
            assert!(
                !writer
                    .put(ts + Duration::from_secs(SHARD_TIME + 1), &frame,)
                    .expect("Failed to store data")
            );
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(111));

        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(1)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(1));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(222));

        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(SHARD_TIME)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));

        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(SHARD_TIME + 1)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME + 1));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(444));
    }

    store_test!(put_read_corrupt_data, _put_read_corrupt_data);
    fn _put_read_corrupt_data(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        let ts_next = ts + Duration::from_secs(1);
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            // Inject an extra byte to corrupt data file
            for entry in fs::read_dir(&dir).expect("Failed to read dir") {
                let entry = entry.expect("Failed to list entry");
                if let Some(name) = entry.path().file_name() {
                    if name.to_string_lossy().starts_with("data_") {
                        OpenOptions::new()
                            .append(true)
                            .open(entry.path())
                            .expect("Failed to open data file")
                            .write_all(&[0])
                            .expect("Failed to write to data file");
                    }
                }
            }

            frame.sample.cgroup.memory_current = Some(222);

            // Write a second sample after the faulty byte
            writer.put(ts_next, &frame).expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));

        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts_next), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts_next);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(222));
    }

    store_test!(
        read_past_the_end_returns_none,
        _read_past_the_end_returns_none
    );
    fn _read_past_the_end_returns_none(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame_opt = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(1)),
                Direction::Forward,
            )
            .expect("Failed to read sample");
        assert_eq!(frame_opt, None);
    }

    store_test!(read_iterates_appropriately, _read_iterates_appropriately);
    fn _read_iterates_appropriately(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(3)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    store_test!(
        put_and_read_work_across_shards,
        _put_and_read_work_across_shards
    );
    fn _put_and_read_work_across_shards(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                .expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(1)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    store_test!(read_reverse, _read_reverse);
    fn _read_reverse(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Reverse)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    store_test!(read_reverse_across_shards, _read_reverse_across_shards);
    fn _read_reverse_across_shards(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                .expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(SHARD_TIME) - Duration::from_secs(1)),
                Direction::Reverse,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    store_test!(discard_earlier, _discard_earlier);
    fn _discard_earlier(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(1), &frame)
                .expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(777);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                .expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(888);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME + 1), &frame)
                .expect("Failed to store data");

            writer
                .discard_earlier(ts + Duration::from_secs(SHARD_TIME + 1))
                .expect("Failed to discard data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(777));
    }

    store_test!(try_discard_until_size, _try_discard_until_size);
    fn _try_discard_until_size(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let dir_path_buf = dir.path().to_path_buf();
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        let mut shard_sizes = Vec::new();
        let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
            .expect("Failed to create store");

        // Write n samples from timestamp 1 seconds apart, returning size
        // increase of the store directory.
        let mut write = |timestamp: SystemTime, n: u64| -> u64 {
            let dir_size = get_dir_size(dir_path_buf.clone());
            let mut frame = DataFrame::default();
            for i in 0..n {
                frame.sample.cgroup.memory_current = Some(n as i64 + i as i64);
                writer
                    .put(timestamp + Duration::from_secs(i), &frame)
                    .expect("Failed to store data");
            }
            let dir_size_after = get_dir_size(dir_path_buf.clone());
            assert!(
                dir_size_after > dir_size,
                "Directory size did not increase. before: {} after: {}: n_samples {}",
                dir_size,
                dir_size_after,
                n,
            );
            dir_size_after - dir_size
        };

        let num_shards = 7;
        for i in 0..num_shards {
            shard_sizes.push(write(ts + Duration::from_secs(SHARD_TIME * i), i + 1));
        }
        let total_size = shard_sizes.iter().sum::<u64>();

        // In the following tests, we use new instances of StoreCursor so that
        // it doesn't continue using the mmap of current files.
        {
            // Nothing is discarded
            let target_size = total_size;
            assert!(
                writer
                    .try_discard_until_size(target_size)
                    .expect("Failed to discard data")
            );
            let frame = StoreCursor::new(get_logger(), dir.path().to_path_buf())
                .get_next(&get_unix_timestamp(ts), Direction::Forward)
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts);
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(1));
        }

        {
            // Delete first shard
            let target_size = total_size - 1;
            assert!(
                writer
                    .try_discard_until_size(target_size)
                    .expect("Failed to discard data")
            );
            let frame = StoreCursor::new(get_logger(), dir.path().to_path_buf())
                .get_next(&get_unix_timestamp(ts), Direction::Forward)
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            // assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(2));
        }

        {
            // Delete second and third shards
            let target_size = total_size - (shard_sizes[0] + shard_sizes[1] + shard_sizes[2]);
            assert!(
                writer
                    .try_discard_until_size(target_size)
                    .expect("Failed to discard data")
            );
            let frame = StoreCursor::new(get_logger(), dir.path().to_path_buf())
                .get_next(&get_unix_timestamp(ts), Direction::Forward)
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME * 3));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(4));
        }

        {
            // Delete fourth and fifth shards, with a target directory size
            // slightly greater than the resulting size directory size
            let target_size = total_size - (shard_sizes[0] + shard_sizes[1] + shard_sizes[2] +
                shard_sizes[3] + shard_sizes[4])
            + /* smaller than a shard */ 1;
            assert!(
                writer
                    .try_discard_until_size(target_size)
                    .expect("Failed to discard data")
            );
            let frame = StoreCursor::new(get_logger(), dir.path().to_path_buf())
                .get_next(&get_unix_timestamp(ts), Direction::Forward)
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME * 5));
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(6));
        }

        {
            // Delete until size is 1. Verify that the current shard remains
            // (i.e. size > 1).
            assert!(
                !writer
                    .try_discard_until_size(1)
                    .expect("Failed to discard data"),
            );
            let frame = StoreCursor::new(get_logger(), dir.path().to_path_buf())
                .get_next(&get_unix_timestamp(ts), Direction::Forward)
                .expect("Failed to read sample")
                .expect("Did not find stored sample");
            assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME) * 6);
            assert_eq!(frame.1.sample.cgroup.memory_current, Some(7));
        }
    }

    store_test!(flock_protects, _flock_protects);
    fn _flock_protects(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = SystemTime::now();
        let shard = calculate_shard(ts);
        let mut index_path = dir.path().to_path_buf();
        index_path.push(format!("index_{:011}", shard));
        let index = OpenOptions::new()
            .append(true)
            .create(true)
            .open(index_path.as_path())
            .expect("Failed to create index file");
        nix::fcntl::flock(
            index.as_raw_fd(),
            nix::fcntl::FlockArg::LockExclusiveNonblock,
        )
        .expect("Failed to acquire flock on index file");

        assert!(
            StoreWriter::new(get_logger(), &dir, compression_mode, format).is_err(),
            "Did not conflict on index lock"
        );
    }

    store_test!(
        writing_to_already_written_index_works,
        _writing_to_already_written_index_works
    );
    fn _writing_to_already_written_index_works(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));

        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(1)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    store_test!(
        read_skips_over_corrupt_index_entry,
        _read_skips_over_corrupt_index_entry
    );
    fn _read_skips_over_corrupt_index_entry(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }
        // Append garbage to the index entry
        {
            let shard = calculate_shard(ts);
            let mut index_path = dir.path().to_path_buf();
            index_path.push(format!("index_{:011}", shard));
            let mut index = OpenOptions::new()
                .append(true)
                .create(true)
                .open(index_path.as_path())
                .expect("Failed to create index file");
            index
                .write_all(b"This is complete garbage data that is longer than an entry")
                .expect("Failed to append to index");
        }
        {
            let mut writer = StoreWriter::new(get_logger(), &dir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let frame = store_cursor
            .get_next(
                &get_unix_timestamp(ts + Duration::from_secs(1)),
                Direction::Forward,
            )
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    store_test!(writer_creates_directory, _writer_creates_directory);
    fn _writer_creates_directory(compression_mode: CompressionMode, format: Format) {
        let dir = TempDir::with_prefix("below_store_test.").expect("tempdir failed");
        let mut subdir = dir.path().to_path_buf();
        subdir.push("foo");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(get_logger(), &subdir, compression_mode, format)
                .expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let mut store_cursor = StoreCursor::new(get_logger(), subdir);
        let frame = store_cursor
            .get_next(&get_unix_timestamp(ts), Direction::Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }
}
