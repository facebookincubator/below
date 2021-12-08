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

use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{Context, Result};
use memmap::{Mmap, MmapOptions};
use slog::{warn, Logger};
use zstd::stream::decode_all;

use crate::{
    deserialize_frame, get_index_files, Crc32, DataFrame, Direction, Format, IndexEntry,
    IndexEntryFlags, SerializedFrame, INDEX_ENTRY_SIZE, SHARD_TIME,
};

/// A read-only Iterator that can move back and forth.
pub trait Cursor {
    type Offset;
    type Item;

    /// Get offset of the cursor.
    fn get_offset(&self) -> Self::Offset;
    /// Set offset of the cursor.
    fn set_offset(&mut self, offset: Self::Offset);
    /// Get the item pointed to by the cursor. Could be None if item is invalid.
    fn get(&self) -> Option<Self::Item>;
    /// Move the cursor one step in the given direction if it's within range.
    /// Return if the cursor is moved.
    fn advance(&mut self, direction: Direction) -> Result<bool>;
    /// Move the cursor in the given direction until a valid item is obtained.
    /// If no more valid item available, return None and offset unchanged.
    fn next(&mut self, direction: Direction) -> Result<Option<Self::Item>> {
        let offset = self.get_offset();
        while self.advance(direction)? {
            if let Some(item) = self.get() {
                return Ok(Some(item));
            }
        }
        self.set_offset(offset);
        Ok(None)
    }
}

/// A cursor that has keys bound to each position in ascending order.
/// Making this generic so that each Cursor may define multiple Keys.
pub trait KeyedCursor<Key: std::cmp::Ord>: Cursor {
    /// Get the key of the current position.
    fn get_key(&self) -> Option<Key>;

    /// Used by jump_to_keys with same parameters. Should be overridden to make
    /// jump_to_key more efficient by moving cursor near the key. Notice that
    /// jump_to_key should work as long as keys are monotonic in same order as
    /// offsets, even if this function does nothing.
    fn jump_near_key(&mut self, _key: &Key, _direction: Direction) {}

    /// Move the cursor to the closest valid pos to the given key in the given
    /// direction. Return if the key at the final pos satisfies the ordering
    /// requirement. For example, for a list of int keys: [3, 7, 8, 12, 19],
    /// jumping to key 9 forward moves cursor to index 3 because 12 is the
    /// smallest int greater than 9 in the list. Jumping to key 0 reverse moves
    /// cursor to index 0 and returns false because no key is smaller than 3.
    fn jump_to_key(&mut self, key: &Key, direction: Direction) -> Result<bool> {
        self.jump_near_key(key, direction);
        // Move cursor backward to get a position with lower key order, and then
        // move forward to get the first position with higher key order.
        let mut curr_key = self.get_key();
        for curr_dir in &[direction.flip(), direction] {
            let skip_order = curr_dir.get_skip_order();
            while curr_key.as_ref().map_or(true, |k| k.cmp(key) == skip_order) {
                if !self.advance(*curr_dir)? {
                    break;
                }
                curr_key = self.get_key();
            }
        }
        // Check if the last key satisfies the direction order
        Ok(curr_key.map_or(false, |k| k.cmp(key) != direction.get_skip_order()))
    }

    /// Convenient function to jump to a key and get the closest valid item to
    /// the key. Preference is given to the specified direction. Returns None
    /// only if there are no keys at all.
    ///
    /// For example, for a list of int keys: [3, 7, 8, 12, 19],
    /// `cursor.get_near(0, Direction::Reverse)` will jump
    /// cursor to 3 and return 3.
    fn get_near(
        &mut self,
        key: &Key,
        preferred_direction: Direction,
    ) -> Result<Option<Self::Item>> {
        self.jump_to_key(key, preferred_direction)?;
        match self.get() {
            Some(item) => Ok(Some(item)),
            None => self.next(preferred_direction),
        }
    }

    /// Convenient function to jump to a key and get the closest valid item
    /// that is at key or in the given direction of key. Returns None if
    /// no such key exists.
    ///
    /// For example, for a list of int keys: [3, 7, 8, 12, 19],
    /// `cursor.get_next(0, Direction::Reverse)` will return
    /// `Ok(None)`.
    fn get_next(&mut self, key: &Key, direction: Direction) -> Result<Option<Self::Item>> {
        if self.jump_to_key(key, direction)? {
            match self.get() {
                Some(item) => Ok(Some(item)),
                None => self.next(direction),
            }
        } else {
            Ok(None)
        }
    }
}

/// For read-only access to a store. Similar to an iterator, but support moving
/// back and forth.
pub struct StoreCursor {
    logger: Logger,
    // Path to the store directory that contains index and data files.
    path: PathBuf,
    // Current shard this cursor points to.
    shard: Option<u64>,
    // Mmap of the index and data files of the current shard. Could be None if
    // the current shard does not exist.
    index_mmap: Option<Mmap>,
    data_mmap: Option<Mmap>,
    // Current offset into the index mmap. The combination of shard and offset
    // locates the exact sample of this store. Offset could be None if shard
    // does not exist or just moved to a newly initialized shard.
    index_offset: Option<usize>,
}

enum StoreFile {
    Index,
    Data,
}

impl StoreCursor {
    /// Create a new cursor with uninitialized shard.
    pub fn new(logger: Logger, path: PathBuf) -> Self {
        Self {
            logger,
            path,
            shard: None,
            index_mmap: None,
            data_mmap: None,
            index_offset: None,
        }
    }

    /// Get the mmap of a related store file based on the given shard. If the
    /// file is not found or empty, None will be returned.
    fn get_mmap(&self, file_type: StoreFile, shard: u64) -> Result<Option<Mmap>> {
        let prefix = match file_type {
            StoreFile::Index => "index",
            StoreFile::Data => "data",
        };
        let path = self.path.join(format!("{}_{:011}", prefix, shard));
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                warn!(
                    self.logger,
                    "Expected file does not exist: {}",
                    path.display()
                );
                return Ok(None);
            }
            Err(e) => {
                return Err(e).context(format!("Failed while opening file: {}", path.display()));
            }
        };

        let mut len = file
            .metadata()
            .with_context(|| format!("Failed to get metadata of file: {}", path.display()))?
            .len() as usize;
        if let StoreFile::Index = file_type {
            len = len - len % INDEX_ENTRY_SIZE;
        }
        if len == 0 {
            warn!(self.logger, "0 length file found: {}", path.display());
            return Ok(None);
        }

        // Mmap is unsafe because it allows unrestricted concurrent access. In
        // our case, we only have one background process (below record) doing
        // append-only writes to both index and data files. We also use CRC to
        // verify file content. As long as we do read-only operations here, this
        // should be Ok.
        unsafe {
            Some(
                MmapOptions::new()
                    .len(len)
                    .map(&file)
                    .with_context(|| format!("Failed to mmap file {}", path.display())),
            )
            .transpose()
        }
    }

    /// Update the cursor to use the given shard and initialize the mmaps. If
    /// the current shard's index mmap has grown in length, update to the new
    /// index mmap so data appended since last update will show up.
    /// Return if the cursor updated. Could return false if either index or data
    /// file is empty or does not exist, or if the given shard is the same as
    /// the current shard and the index file does not grow. Index offset is also
    /// reset to None if moved to a different shard.
    fn update_shard(&mut self, shard: u64) -> Result<bool> {
        // This mmap is always aligned to INDEX_ENTRY_SIZE
        let new_index_mmap = match self.get_mmap(StoreFile::Index, shard)? {
            Some(index_mmap) => index_mmap,
            None => return Ok(false),
        };
        let new_data_mmap = match self.get_mmap(StoreFile::Data, shard)? {
            Some(data_mmap) => data_mmap,
            None => return Ok(false),
        };
        if self.shard == Some(shard) {
            let index_mmap_len = self.index_mmap.as_ref().map_or(0, |m| m.len());
            if new_index_mmap.len() <= index_mmap_len {
                // Nothing is updated if index file does not change.
                return Ok(false);
            }
        } else {
            self.shard = Some(shard);
            self.index_offset = None;
        }
        self.index_mmap = Some(new_index_mmap);
        self.data_mmap = Some(new_data_mmap);
        Ok(true)
    }

    /// Update current shard or move the cursor to a neighbor valid shard.
    /// Return if the cursor is updated (current shard is updated with new mmap
    /// or cursor has moved to next shard). Returning false means there is no
    /// more shard in the given direction. Retrying may succeed as the store
    /// directory is scanned on every call.
    fn update_or_advance_shard(&mut self, direction: Direction) -> Result<bool> {
        let entries = get_index_files(&self.path)?;

        let entries_iter: Box<dyn Iterator<Item = &String>> = match direction {
            Direction::Forward => Box::new(entries.iter()),
            Direction::Reverse => Box::new(entries.iter().rev()),
        };
        for entry in entries_iter {
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

            if let Some(shard) = self.shard.as_ref() {
                if entry_shard.cmp(shard) == direction.get_skip_order() {
                    continue;
                }
            }

            // Try to refresh the current shard (any new entries appended?) or
            // move to a different shard.
            if self.update_shard(entry_shard)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Move index offset to next position in given direction if it's valid.
    /// Return if index offset is updated. False means no more valid position.
    fn advance_index(&mut self, direction: Direction) -> bool {
        if let Some(index_mmap) = self.index_mmap.as_ref() {
            // get_mmap ensures that index_mmap.len() >= INDEX_ENTRY_SIZE, and
            // thus 0 is always a valid index.
            debug_assert!(index_mmap.len() > 0);
            // index offset may be None if overflows
            let offset = match self.index_offset {
                Some(offset) => match direction {
                    Direction::Forward => offset
                        .checked_add(INDEX_ENTRY_SIZE)
                        .filter(|o| o < &index_mmap.len()),
                    Direction::Reverse => offset.checked_sub(INDEX_ENTRY_SIZE),
                },
                // Default offsets
                None => match direction {
                    Direction::Forward => Some(0),
                    Direction::Reverse => index_mmap.len().checked_sub(INDEX_ENTRY_SIZE),
                },
            };
            if offset.is_some() {
                self.index_offset = offset;
                return true;
            }
        }
        false
    }

    /// Get the index entry the cursor currently pointing at.
    fn get_index_entry(&self) -> Option<&IndexEntry> {
        let index_mmap = self.index_mmap.as_ref()?;
        let index_offset = self.index_offset?;

        let index_entry_slice =
            index_mmap.get(index_offset..(index_offset.checked_add(INDEX_ENTRY_SIZE)?))?;
        // Safe because IndexEntry is always initialized and validated with crc.
        let (_, body, _) = unsafe { index_entry_slice.align_to::<IndexEntry>() };
        if body.len() != 1 {
            warn!(
                self.logger,
                "Mis-aligned index entry found: shard={} offset={}",
                self.shard.unwrap(),
                index_offset,
            );
            return None;
        }
        let index_entry = &body[0];

        if index_entry.crc32() != index_entry.index_crc {
            warn!(
                self.logger,
                "Corrupted index entry found: shard={} offset={:#x}",
                self.shard.unwrap(),
                index_offset,
            );
            None
        } else {
            Some(index_entry)
        }
    }
}

/// Offset of a StoreCursor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoreOffset {
    shard: Option<u64>,
    index_offset: Option<usize>,
}

impl StoreOffset {
    /// Shard and index offset are trimmed to multiples of SHARD_TIME and
    /// INDEX_ENTRY_SIZE respectively. Index offset is ignored if shard is None.
    pub fn new(shard: Option<u64>, index_offset: Option<usize>) -> Self {
        StoreOffset {
            shard: shard.as_ref().map(|s| s - s % SHARD_TIME),
            index_offset: shard.and(index_offset.map(|o| o - o % INDEX_ENTRY_SIZE)),
        }
    }

    pub fn get_shard(&self) -> Option<u64> {
        self.shard
    }

    pub fn get_index_offset(&self) -> Option<usize> {
        self.index_offset
    }
}

impl Cursor for StoreCursor {
    type Offset = StoreOffset;
    type Item = (SystemTime, DataFrame);

    fn get_offset(&self) -> StoreOffset {
        StoreOffset::new(self.shard, self.index_offset)
    }

    fn set_offset(&mut self, offset: StoreOffset) {
        if let Some(shard) = offset.get_shard() {
            if self.shard == Some(shard) || self.update_shard(shard).unwrap_or(false) {
                self.index_offset = offset.get_index_offset();
                return;
            }
        }
        // Set the shard even if it does not exist so that advance still finds
        // the closest valid shard. Clear index_mmap as it no longer corresponds
        // to this invalid or virtual shard. The index_offset does not matter
        // much in this case as it will be overwritten once advance is called.
        // Keep it so get_offset may get the same value back.
        self.shard = offset.get_shard();
        self.index_mmap = None;
        self.index_offset = offset.get_index_offset();
    }

    /// Move the cursor to a neighbor position. Return if the cursor is updated.
    /// Returning false means there is no more valid position in the given
    /// direction, although retrying may succeed. Notice that true means the
    /// current position is valid, but underlying sample may still be invalid.
    fn advance(&mut self, direction: Direction) -> Result<bool> {
        while !self.advance_index(direction) {
            if !self.update_or_advance_shard(direction)? {
                // No more shard available
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get the sample the cursor is currently pointing at. Notice that a store
    /// may contain holes due to data corruption etc, and thus returns None.
    /// This does not mean samples are depleted. More could be retrieved by
    /// advancing further to skip the holes.
    fn get(&self) -> Option<(SystemTime, DataFrame)> {
        let data_mmap = self.data_mmap.as_ref()?;
        let index_entry = self.get_index_entry()?;

        let data_offset = index_entry.offset as usize;
        let data_len = index_entry.len as usize;
        let data_slice = data_mmap.get(data_offset..(data_offset.checked_add(data_len)?))?;

        if data_slice.crc32() != index_entry.data_crc {
            warn!(
                self.logger,
                "Corrupted data entry found: ts={} offset={:#x}",
                index_entry.timestamp,
                index_entry.offset,
            );
            return None;
        }

        let data_decompressed = if index_entry.flags.contains(IndexEntryFlags::COMPRESSED) {
            SerializedFrame::Copy(match decode_all(data_slice) {
                Ok(decoded) => decoded,
                Err(e) => {
                    warn!(self.logger, "Failed to decompress data frame: {}", e);
                    return None;
                }
            })
        } else {
            SerializedFrame::Slice(data_slice)
        };

        let format = if index_entry.flags.contains(IndexEntryFlags::CBOR) {
            Format::Cbor
        } else {
            Format::Thrift
        };

        let ts = std::time::UNIX_EPOCH + std::time::Duration::from_secs(index_entry.timestamp);
        match deserialize_frame(data_decompressed.data(), format) {
            Ok(df) => Some((ts, df)),
            Err(e) => {
                warn!(self.logger, "Failed to deserialize data frame: {}", e);
                None
            }
        }
    }
}

/// StoreCursor has each cursor position bound to a SystemTime in monotonic
/// order. This allows moving the cursor relative to SystemTime instances.
impl KeyedCursor<u64> for StoreCursor {
    /// Get timestamp of the current pos.
    fn get_key(&self) -> Option<u64> {
        Some(self.get_index_entry()?.timestamp)
    }

    /// Set the cursor offset near the given timestamp by inferring shard and
    /// index offset.
    fn jump_near_key(&mut self, key: &u64, _direction: Direction) {
        let time_offset = key % SHARD_TIME;
        let shard = key - time_offset;
        self.set_offset(StoreOffset::new(Some(shard), None));
        // Move to the end of the shard.
        if self.advance_index(Direction::Reverse) {
            if let Some(last_entry) = self.get_index_entry() {
                let last_entry_index_offset = self
                    .get_offset()
                    .get_index_offset()
                    .expect("get_index_offset should return Some if get_index_entry returns Some");
                let last_entry_time_offset = last_entry.timestamp % SHARD_TIME;
                if last_entry_time_offset != 0 {
                    // Assume samples are recorded in constant interval and
                    // scale index offset by time offset
                    let index_offset_hint = (last_entry_index_offset as f64
                        / last_entry_time_offset as f64
                        * time_offset as f64) as usize;
                    self.set_offset(StoreOffset::new(Some(shard), Some(index_offset_hint)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{serialize_frame, StoreWriter};
    use common::util::get_unix_timestamp;
    use slog::Drain;
    use std::fs::OpenOptions;
    use std::io::Write;
    use tempdir::TempDir;
    use Direction::{Forward, Reverse};

    /// Simple cursor to illustrate implementation and test default methods.
    struct TestCursor<'a> {
        data: &'a Vec<Option<i32>>,
        offset: Option<usize>,
    }
    impl Cursor for TestCursor<'_> {
        type Offset = Option<usize>;
        type Item = i32;
        fn get_offset(&self) -> Self::Offset {
            self.offset
        }
        fn set_offset(&mut self, offset: Self::Offset) {
            self.offset = offset;
        }
        fn get(&self) -> Option<Self::Item> {
            self.offset
                .as_ref()
                .and_then(|o| self.data.get(*o).cloned().flatten())
        }
        fn advance(&mut self, direction: Direction) -> Result<bool> {
            let offset = match self.offset {
                Some(offset) => match direction {
                    Direction::Forward => offset.checked_add(1).filter(|o| o < &self.data.len()),
                    Direction::Reverse => offset.checked_sub(1),
                },
                // Default offsets
                None => match direction {
                    Direction::Forward => Some(0).filter(|o| o < &self.data.len()),
                    Direction::Reverse => self.data.len().checked_sub(1),
                },
            };
            if offset.is_some() {
                self.offset = offset;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
    impl KeyedCursor<i32> for TestCursor<'_> {
        fn get_key(&self) -> Option<i32> {
            self.get()
        }
    }

    /// Test default implementation of next(). It should skip invalid items.
    #[test]
    fn default_next() {
        let data = vec![None, Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        assert_eq!(cursor.next(Forward).unwrap(), Some(3));
        assert_eq!(cursor.next(Forward).unwrap(), Some(5));
        assert_eq!(cursor.next(Forward).unwrap(), Some(9));
        assert_eq!(cursor.next(Forward).unwrap(), None);
        assert_eq!(cursor.next(Reverse).unwrap(), Some(5));
        assert_eq!(cursor.next(Reverse).unwrap(), Some(3));
        assert_eq!(cursor.next(Reverse).unwrap(), None);
        // Offset unchanged after reaching boundry
        assert_eq!(cursor.get(), Some(3));
    }

    /// Test default implementation of jump_to_key().
    #[test]
    fn default_jump_to_key() {
        let data = vec![None, Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        // Exact key
        assert!(cursor.jump_to_key(&3, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(3));
        assert!(cursor.jump_to_key(&5, Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(5));
        // Closest key
        assert!(cursor.jump_to_key(&7, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(9));
        assert!(cursor.jump_to_key(&4, Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(3));
        // No key satisfies direction constrain.
        assert!(!cursor.jump_to_key(&10, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(9));
        assert!(!cursor.jump_to_key(&0, Reverse).unwrap());
        assert_eq!(cursor.get_key(), None);
    }

    /// Test default implementation of get_near().
    #[test]
    fn default_get_near() {
        let data = vec![Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        // Exact key
        assert_eq!(cursor.get_near(&5, Forward).unwrap(), Some(5));
        // Key in direction
        assert_eq!(cursor.get_near(&4, Forward).unwrap(), Some(5));
        assert_eq!(cursor.get_near(&4, Reverse).unwrap(), Some(3));
        // Key in direction but no key there
        assert_eq!(cursor.get_near(&2, Reverse).unwrap(), Some(3));
        assert_eq!(cursor.get_near(&10, Forward).unwrap(), Some(9));
    }

    /// Test default implementation of get_next().
    #[test]
    fn default_get_next() {
        let data = vec![Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        // Exact key
        assert_eq!(cursor.get_next(&5, Forward).unwrap(), Some(5));
        // Key in direction
        assert_eq!(cursor.get_next(&4, Forward).unwrap(), Some(5));
        assert_eq!(cursor.get_next(&4, Reverse).unwrap(), Some(3));
        // Key in direction but no key there
        assert_eq!(cursor.get_next(&2, Reverse).unwrap(), None);
        assert_eq!(cursor.get_next(&10, Forward).unwrap(), None);
    }

    fn get_logger() -> Logger {
        let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
        Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!())
    }

    /// Write a single sample in different ways and read it back.
    fn simple_put_read(compress: bool, format: Format) {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let now = std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts);
        let mut writer = StoreWriter::new(&dir, compress, format).expect("Failed to create store");
        let mut frame = DataFrame::default();
        frame.sample.cgroup.memory_current = Some(42);
        writer
            .put(now, &frame, get_logger())
            .expect("Failed to store data");

        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());
        let sample = cursor
            .next(Forward)
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_eq!(sample, (now, frame));
    }

    #[test]
    fn read_cbor() {
        simple_put_read(false, Format::Cbor);
    }
    #[test]
    fn read_compressed_cbor() {
        simple_put_read(true, Format::Cbor);
    }
    #[cfg(fbcode_build)]
    #[test]
    fn read_thrift() {
        simple_put_read(false, Format::Thrift);
    }
    #[cfg(fbcode_build)]
    #[test]
    fn read_compressed_thrift() {
        simple_put_read(true, Format::Thrift);
    }

    /// For writing samples readable by the cursor and injecting corruptions.
    /// Read correctness is tested above. Following tests only care about
    /// whether the Cursor trait is implemented correctly, therefore using this
    /// simplistic writer that only writes default data frame.
    struct TestWriter {
        path: PathBuf,
    }

    impl TestWriter {
        pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
            }
        }

        pub fn put(&self, timestamp: u64) -> Result<()> {
            self.put_helper(timestamp, false, false)
        }
        pub fn put_corrupt_index(&self, timestamp: u64) -> Result<()> {
            self.put_helper(timestamp, true, false)
        }
        pub fn put_corrupt_data(&self, timestamp: u64) -> Result<()> {
            self.put_helper(timestamp, false, true)
        }

        /// Similar to the StoreWriter but only writes default data frame. May
        /// invalidate index or data entry by zeroing crc if requested.
        fn put_helper(
            &self,
            timestamp: u64,
            corrupt_index: bool,
            corrupt_data: bool,
        ) -> Result<()> {
            let shard = timestamp - timestamp % SHARD_TIME;
            let open_options = OpenOptions::new().create(true).append(true).clone();

            let data_bytes = serialize_frame(&DataFrame::default(), Format::Cbor)
                .context("Failed to serialize data frame")?;
            let data_crc = if corrupt_data { 0 } else { data_bytes.crc32() };
            let mut data_file = open_options
                .open(self.path.join(format!("data_{:011}", shard)))
                .context("Failed to open data file")?;
            let offset = data_file
                .metadata()
                .context("Failed to get metadata of data file")?
                .len();
            data_file
                .write_all(&data_bytes)
                .context("Failed to write to data file")?;

            let mut index_entry = IndexEntry {
                timestamp,
                offset,
                len: data_bytes.len() as u32,
                flags: IndexEntryFlags::CBOR,
                data_crc,
                index_crc: 0,
            };
            if !corrupt_index {
                index_entry.index_crc = index_entry.crc32();
            }
            let entry_slice = unsafe {
                std::slice::from_raw_parts(
                    &index_entry as *const IndexEntry as *const u8,
                    INDEX_ENTRY_SIZE,
                )
            };
            open_options
                .open(self.path.join(format!("index_{:011}", shard)))
                .context("Failed to open index file")?
                .write_all(entry_slice)
                .context("Failed to write entry to index file")?;
            Ok(())
        }
    }

    /// Calling advance with nothing in the store and cursor uninitialized.
    #[test]
    fn advance_when_empty() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        assert!(!cursor.advance(Forward).unwrap());
        assert!(cursor.get_key().is_none());
        assert!(!cursor.advance(Reverse).unwrap());
        assert!(cursor.get_key().is_none());
    }

    /// Calling advance in both directions when cursor is at the last pos.
    #[test]
    fn advance_at_boundries() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts).unwrap();

        // First sample
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
        // No more sample forward. Still at first sample
        assert!(!cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
        // No more sample reverse. Still at first sample.
        assert!(!cursor.advance(Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
    }

    /// Cursor moves back and forth.
    #[test]
    fn advance_simple() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts).unwrap();
        writer.put(ts + 5).unwrap();
        writer.put(ts + SHARD_TIME).unwrap();

        // First sample
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
        // Second sample in same shard
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + 5));
        // Third sample across shard
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME));
        // Back to second sample
        assert!(cursor.advance(Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + 5));
        // Back to first sample
        assert!(cursor.advance(Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
    }

    /// Retry advance succeeds after updates.
    #[test]
    fn advance_retry() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts).unwrap();
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
        assert!(!cursor.advance(Forward).unwrap());

        // Advance succeeds after same shard update
        writer.put(ts + 5).unwrap();
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + 5));
        assert!(!cursor.advance(Forward).unwrap());

        // Advance succeeds after new shard update
        writer.put(ts + SHARD_TIME).unwrap();
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME));
        assert!(!cursor.advance(Forward).unwrap());
    }

    /// Get corrupt index/data should return None.
    #[test]
    fn get_corrupt() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        // Both index and data are None if index alone is corrupted.
        writer.put_corrupt_index(ts).unwrap();
        assert!(cursor.advance(Forward).unwrap());
        assert!(cursor.get_key().is_none());
        assert!(cursor.get().is_none());

        // Only data is None if data is corrupted but not index.
        writer.put_corrupt_data(ts + 5).unwrap();
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + 5));
        assert!(cursor.get().is_none());
    }

    /// Calling next skips corrupted index/data entries.
    #[test]
    fn skip_corrupt() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        // Only 2 valid samples, separated by invalid ones across shards
        writer.put_corrupt_data(ts).unwrap();
        writer.put(ts + 5).unwrap(); // First valid sample
        writer.put_corrupt_data(ts + 5 * 2).unwrap();
        writer.put_corrupt_index(ts + 5 * 3).unwrap();
        writer.put_corrupt_data(ts + SHARD_TIME).unwrap();
        writer.put_corrupt_index(ts + SHARD_TIME * 2).unwrap();
        writer.put(ts + SHARD_TIME * 2 + 5).unwrap(); // Second one
        writer.put_corrupt_data(ts + SHARD_TIME * 3).unwrap();

        // Calling next to move and get valid samples in both directions
        assert_eq!(
            get_unix_timestamp(cursor.next(Forward).unwrap().unwrap().0),
            ts + 5
        );
        assert_eq!(
            get_unix_timestamp(cursor.next(Forward).unwrap().unwrap().0),
            ts + SHARD_TIME * 2 + 5
        );
        // Offset unchanged
        assert!(cursor.next(Forward).unwrap().is_none());
        assert_eq!(
            get_unix_timestamp(cursor.next(Reverse).unwrap().unwrap().0),
            ts + 5
        );
    }

    /// Ensure get and set cursor offset work as expected.
    #[test]
    fn manipulate_offset() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts).unwrap();
        writer.put(ts + 5).unwrap();
        writer.put(ts + SHARD_TIME * 2 + 5).unwrap();

        let expected_offsets = &[
            StoreOffset::new(Some(ts), Some(0)),
            StoreOffset::new(Some(ts), Some(INDEX_ENTRY_SIZE)),
            StoreOffset::new(Some(ts + SHARD_TIME * 2), Some(0)),
        ];

        // Verify offset values.
        assert_eq!(cursor.get_offset(), StoreOffset::default());
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_offset(), expected_offsets[0]);
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_offset(), expected_offsets[1]);
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_offset(), expected_offsets[2]);

        // Get None from default offset.
        cursor.set_offset(StoreOffset::default());
        assert!(cursor.get_key().is_none());
        // Get None from invalid offsets.
        cursor.set_offset(StoreOffset::new(
            Some(ts + SHARD_TIME),
            Some(INDEX_ENTRY_SIZE),
        ));
        assert!(cursor.get_key().is_none());
        cursor.set_offset(StoreOffset::new(Some(ts + SHARD_TIME * 2), None));
        assert!(cursor.get_key().is_none());
        // Get values from expected offsets.
        cursor.set_offset(expected_offsets[1].clone());
        assert_eq!(cursor.get_key(), Some(ts + 5));
        cursor.set_offset(expected_offsets[0].clone());
        assert_eq!(cursor.get_key(), Some(ts));
        cursor.set_offset(expected_offsets[2].clone());
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME * 2 + 5));
    }

    /// Calling advance after set_offset with invalid offsets.
    #[test]
    fn advance_from_invalid_offset() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts).unwrap();
        writer.put(ts + SHARD_TIME * 2).unwrap();

        // To same shard
        cursor.set_offset(StoreOffset::new(Some(ts), Some(INDEX_ENTRY_SIZE)));
        assert!(cursor.advance(Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(ts));
        // To different shard
        cursor.set_offset(StoreOffset::new(Some(ts + SHARD_TIME), Some(0)));
        assert!(cursor.advance(Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME * 2));
        // Over boundry (offset shouldn't change)
        cursor.set_offset(StoreOffset::new(Some(ts + SHARD_TIME * 4), Some(0)));
        assert!(!cursor.advance(Forward).unwrap());
        assert_eq!(
            cursor.get_offset(),
            StoreOffset::new(Some(ts + SHARD_TIME * 4), Some(0))
        );
    }

    /// Ensure jump_to_key works as expected.
    #[test]
    fn jump_to_key() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = get_unix_timestamp(SystemTime::now());
        let writer = TestWriter::new(&dir);
        let mut cursor = StoreCursor::new(get_logger(), dir.path().to_path_buf());

        writer.put(ts + 5).unwrap();
        writer.put(ts + 5 * 20).unwrap();
        writer.put(ts + 5 * 21).unwrap();
        writer.put(ts + SHARD_TIME * 2).unwrap();

        // Jump to exact key
        cursor.jump_to_key(&(ts + 5), Forward).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + 5));
        cursor.jump_to_key(&(ts + SHARD_TIME * 2), Reverse).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME * 2));

        // Jump over the boundry gets the last valid samples.
        cursor.jump_to_key(&(ts), Reverse).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + 5));
        cursor.jump_to_key(&(ts + SHARD_TIME * 3), Forward).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME * 2));

        // Jump to cloest sample
        cursor.jump_to_key(&(ts + 5 * 100), Forward).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + SHARD_TIME * 2));
        cursor.jump_to_key(&(ts + 5 * 100), Reverse).unwrap();
        assert_eq!(cursor.get_key(), Some(ts + 5 * 21));
    }
}
