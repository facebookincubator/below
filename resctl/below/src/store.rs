use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use fbthrift::compact_protocol;
use slog::warn;

use crate::util::get_unix_timestamp;
use below_thrift::DataFrame;

/// This data store works as follows:
///
/// Each data item (e.g. DataFrame) is simply appended to a data file.
///
/// An IndexEntry is appended to a corresponding index file. Each
/// IndexEntry contains the timestamp (e.g. key) of the data item, its
/// offset into the data file, the length of the data entry, and a CRC
/// of the data entry as well as a CRC of itself.
///
/// The CRCs in the index entry give us an atomicity guarantee - if
/// they are not present and correct, we treat it as if the entry
/// never existed.
///
/// Data and Index files are append-only and never modified (only ever
/// removed).
///
/// Data and Index files are sharded by SHARD_TIME - e.g. any one file
/// only contains data or index entries whose timestamps are congruent
/// modulo SHARD_TIME. This allows data and index files to be cleaned
/// up by just unlinking the files.

const SHARD_TIME: u64 = 24 * 60 * 60;

#[repr(C)]
struct IndexEntry {
    /// Timestamp of the data entry
    timestamp: u64,
    /// Offset into the data file that this entry exists at
    offset: u64,
    /// Length of the data entry
    len: u64,
    /// crc32 of the data entry
    data_crc: u32,
    /// crc32 of this entry (e.g. crc32 of all the above members)
    index_crc: u32,
}

const INDEX_ENTRY_SIZE: usize = std::mem::size_of::<IndexEntry>();

/// The StoreWriter struct maintains state to put more data in the
/// store. It keeps track of the index and data file it's currently
/// working on so in the common case it can just append data. When it
/// rolls over to a new shard, it will recreate itself.
#[derive(Debug)]
pub struct StoreWriter {
    /// Directory of the store itself
    dir: PathBuf,
    /// Currently active index file
    index: File,
    /// Currently active data file
    data: File,
    /// Current length of the data file (needed to record offsets in the index)
    data_len: u64,
    /// Active shard
    shard: u64,
}

// Given path to the store dir, get a Vec<String> of the index file
// names, sorted
macro_rules! get_index_entries {
    ($path:expr) => {{
        let mut entries = fs::read_dir($path)
            .with_context(|| format!("Failed to read directory {}", $path.display()))?
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
            .with_context(|| format!("Failed to read directory entries in {}", $path.display()))?;

        entries.sort_unstable();
        entries
    }};
}

impl StoreWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new_with_timestamp(path, SystemTime::now())
    }

    pub fn new_with_timestamp<P: AsRef<Path>>(path: P, timestamp: SystemTime) -> Result<Self> {
        let shard = calculate_shard(timestamp);

        Self::new_with_shard(path, shard)
    }

    fn new_with_shard<P: AsRef<Path>>(path: P, shard: u64) -> Result<Self> {
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

        let mut index = OpenOptions::new()
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

        // Check the index len and pad if we are not on an
        // INDEX_ENTRY_SIZE boundary. This could only happen if we
        // partially wrote an index entry.
        let index_len = index
            .metadata()
            .with_context(|| {
                format!(
                    "Failed to get metadata of index file: {}",
                    index_path.display()
                )
            })?
            .len();
        if (index_len % INDEX_ENTRY_SIZE as u64) != 0 {
            let remainder = INDEX_ENTRY_SIZE as u64 - (index_len % INDEX_ENTRY_SIZE as u64);
            let zero_vec = vec![0; remainder as usize];
            index.write_all(&zero_vec).with_context(|| {
                format!(
                    "Failed to pad partially written index file: {}",
                    index_path.display()
                )
            })?;
        }

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
            dir: path.as_ref().to_path_buf(),
            index,
            data,
            data_len,
            shard,
        })
    }

    /// Store data with corresponding timestamp. Errors may be
    /// returned if file operations fail.
    pub fn put(&mut self, timestamp: SystemTime, data: &DataFrame) -> Result<()> {
        let shard = calculate_shard(timestamp);
        if shard != self.shard {
            // We just recreate the StoreWriter since this is a new shard
            std::mem::replace(self, Self::new_with_shard(self.dir.as_path(), shard)?);
        }

        // It doesn't really matter which order we write the data in,
        // most filesystems do not provide ordering guarantees for
        // appends to different files anyways. We just need to handle
        // various failure cases on the read side.
        let serialized = compact_protocol::serialize(data);
        let offset = self.data_len;
        self.data
            .write_all(&serialized)
            .context("Failed to write entry to data file")?;
        self.data_len += serialized.len() as u64;
        let data_crc = serialized.as_ref().crc32();
        let mut index_entry = IndexEntry {
            timestamp: get_unix_timestamp(timestamp),
            offset,
            len: serialized.len() as u64,
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
        Ok(())
    }

    /// Discard all data earlier than timestamp
    ///
    /// We do not modify index and data files. We just look for files
    /// which can only contain earlier data and remove them.
    pub fn discard_earlier(&mut self, timestamp: SystemTime, logger: slog::Logger) -> Result<()> {
        let shard = calculate_shard(timestamp);
        let entries = get_index_entries!(self.dir.as_path());

        for entry in entries {
            let v: Vec<&str> = entry.split('_').collect();
            if v.len() != 2 {
                warn!(logger, "Invalid index file name: {}", entry);
                continue;
            }

            let entry_shard = match v[1].parse::<u64>() {
                Ok(val) => val,
                _ => {
                    warn!(logger, "Cannot parse index shard: {}", entry);
                    continue;
                }
            };

            if entry_shard >= shard {
                break;
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
        Ok(())
    }
}

/// Direction to scan for next sample
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Reverse,
}

/// Reads the next sample recorded at time >= timestamp (if
/// Direction::Forward) or time <= timestamp (if Direction::Reverse)
///
/// Reading is "stateless" - maintaining state would be an
/// optimization, but under the expectation that most reads will be
/// happening over some connection, we rather not need to maintain
/// state of ongoing reads and instead just make stateless reads fast.
pub fn read_next_sample<P: AsRef<Path>>(
    path: P,
    timestamp: SystemTime,
    direction: Direction,
    // useful for logging warnings when data corruption is detected
    logger: slog::Logger,
) -> Result<Option<(SystemTime, DataFrame)>> {
    let shard = calculate_shard(timestamp);

    let entries = get_index_entries!(path.as_ref());

    // Here we find the first index with shard >= the timestamp's
    // shard. Then perform a linear scan on that index until we find
    // the first stored timestamp we're looking for.
    //
    // We handle corrupt or incomplete data by simply skipping over
    // it which could result in us scanning additional index files.
    //
    // The linear scan may seem quite dumb given that the index
    // entries should be in sorted order, but consider that sharding
    // by day and recording a sample every five seconds gives us 17280
    // samples in an index (maximum) and each index entry is 32 bytes
    // meaning an index file is at most 552960 bytes (~500kb).
    let entries_iter: Box<dyn Iterator<Item = &String>> = match direction {
        Direction::Forward => Box::new(entries.iter()),
        Direction::Reverse => Box::new(entries.iter().rev()),
    };
    for entry in entries_iter {
        let v: Vec<&str> = entry.split('_').collect();
        if v.len() != 2 {
            warn!(logger, "Invalid index file name: {}", entry);
            continue;
        }

        let entry_shard = match v[1].parse::<u64>() {
            Ok(val) => val,
            _ => {
                warn!(logger, "Cannot parse index shard: {}", entry);
                continue;
            }
        };

        let skip_order = match direction {
            Direction::Forward => std::cmp::Ordering::Less,
            Direction::Reverse => std::cmp::Ordering::Greater,
        };
        if entry_shard.cmp(&shard) == skip_order {
            continue;
        }

        let mut data_path = path.as_ref().to_path_buf();
        data_path.push(format!("data_{:011}", entry_shard));
        let data = match File::open(&data_path) {
            Ok(f) => f,
            // Data file not found could happen if we crashed in the
            // middle of creating an index file and a data file or
            // raced on one of them being deleted treat it as if the
            // index doesn't exist and look for the next one.
            Err(e) if e.kind() == ErrorKind::NotFound => {
                warn!(
                    logger,
                    "Expected data file does not exist: {}",
                    data_path.display()
                );
                continue;
            }
            // Other errors should be returned
            Err(e) => {
                return Err(e).context(format!(
                    "Failed while opening data file {}",
                    data_path.display()
                ))
            }
        };

        let data_len = data
            .metadata()
            .with_context(|| {
                format!(
                    "Failed to get metadata of data file: {}",
                    data_path.display()
                )
            })?
            .len();
        if data_len == 0 {
            warn!(logger, "0 length data file found: {}", data_path.display());
            continue;
        }
        // This is safe because we only append to our files, so an
        // mmap will be immutable
        let data_mmap = unsafe {
            memmap::Mmap::map(&data)
                .with_context(|| format!("Failed to mmap data file {}", data_path.display()))?
        };

        let mut index_path = path.as_ref().to_path_buf();
        index_path.push(&entry);
        let index = File::open(index_path)
            .with_context(|| format!("Failed to open index file {}", entry))?;

        // This is safe because we only append to our files, so an
        // mmap will be immutable
        let index_mmap = unsafe {
            memmap::Mmap::map(&index)
                .with_context(|| format!("Failed to mmap index file {}", entry))?
        };

        // Unsafe because we are materializing this type out of
        // nowhere - this is "safe" because we validate the crc which
        // tells us that this is indeed an IndexEntry
        let index_entries = unsafe {
            std::slice::from_raw_parts(
                index_mmap.as_ptr() as *const IndexEntry,
                index_mmap.len() / INDEX_ENTRY_SIZE,
            )
        };

        // Begin linear scan of index
        //
        // Note: Writer always pads out the index file if there was
        // ever a partial write, so we're fine to iterate over this
        // file as a slice of IndexEntries.
        let index_entries_iter: Box<dyn Iterator<Item = &IndexEntry>> = match direction {
            Direction::Forward => Box::new(index_entries.iter()),
            Direction::Reverse => Box::new(index_entries.iter().rev()),
        };
        for index_entry in index_entries_iter {
            if index_entry.crc32() != index_entry.index_crc {
                warn!(
                    logger,
                    "Corrupted index entry (ts={}) found in {}", index_entry.timestamp, entry
                );
                continue;
            }

            if index_entry.timestamp.cmp(&get_unix_timestamp(timestamp)) == skip_order {
                continue;
            }

            let data_slice = match data_mmap
                .get(index_entry.offset as usize..(index_entry.offset + index_entry.len) as usize)
            {
                Some(data_slice) => data_slice,
                // Hit EOF on data file, go to next index
                None => break,
            };

            if data_slice.crc32() != index_entry.data_crc {
                warn!(
                    logger,
                    "Corrupted data entry (ts={}) found in {}",
                    index_entry.timestamp,
                    data_path.display()
                );
                continue;
            }

            let ts = std::time::UNIX_EPOCH + std::time::Duration::from_secs(index_entry.timestamp);
            return Some(compact_protocol::deserialize(data_slice).map(|df| (ts, df))).transpose();
        }
    }
    Ok(None)
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
    use super::*;
    use slog::Drain;
    use std::time::Duration;
    use tempdir::TempDir;

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

    #[test]
    fn create_writer() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        StoreWriter::new(&dir).expect("Failed to create store");
    }

    #[test]
    fn simple_put_read() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let frame = read_next_sample(&dir, ts, Direction::Forward, get_logger())
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    #[test]
    fn read_past_the_end_returns_none() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let frame_opt = read_next_sample(
            &dir,
            ts + Duration::from_secs(1),
            Direction::Forward,
            get_logger(),
        )
        .expect("Failed to read sample");
        assert_eq!(frame_opt, None);
    }

    #[test]
    fn read_iterates_appropriately() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let frame = read_next_sample(
            &dir,
            ts + Duration::from_secs(3),
            Direction::Forward,
            get_logger(),
        )
        .expect("Failed to read sample")
        .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    #[test]
    fn put_and_read_work_across_shards() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                .expect("Failed to store data");
        }

        let frame = read_next_sample(
            &dir,
            ts + Duration::from_secs(1),
            Direction::Forward,
            get_logger(),
        )
        .expect("Failed to read sample")
        .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    #[test]
    fn read_reverse() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let frame = read_next_sample(&dir, ts, Direction::Reverse, get_logger())
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    #[test]
    fn read_reverse_across_shards() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");

            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(SHARD_TIME), &frame)
                .expect("Failed to store data");
        }

        let frame = read_next_sample(
            &dir,
            ts + Duration::from_secs(SHARD_TIME) - Duration::from_secs(1),
            Direction::Reverse,
            get_logger(),
        )
        .expect("Failed to read sample")
        .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }

    #[test]
    fn discard() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
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
                .discard_earlier(ts + Duration::from_secs(SHARD_TIME + 1), get_logger())
                .expect("Failed to discard data");
        }

        let frame = read_next_sample(&dir, ts, Direction::Forward, get_logger())
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(SHARD_TIME));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(777));
    }

    #[test]
    fn flock_protects() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
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

        StoreWriter::new(&dir).expect_err("Did not conflict on index lock");
    }

    #[test]
    fn writing_to_already_written_index_works() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let frame = read_next_sample(&dir, ts, Direction::Forward, get_logger())
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));

        let frame = read_next_sample(
            &dir,
            ts + Duration::from_secs(1),
            Direction::Forward,
            get_logger(),
        )
        .expect("Failed to read sample")
        .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    #[test]
    fn read_skips_over_corrupt_index_entry() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let ts = std::time::UNIX_EPOCH + Duration::from_secs(SHARD_TIME);
        {
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
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
            let mut writer = StoreWriter::new(&dir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(666);
            writer
                .put(ts + Duration::from_secs(5), &frame)
                .expect("Failed to store data");
        }

        let frame = read_next_sample(
            &dir,
            ts + Duration::from_secs(1),
            Direction::Forward,
            get_logger(),
        )
        .expect("Failed to read sample")
        .expect("Did not find stored sample");
        assert_ts!(frame.0, ts + Duration::from_secs(5));
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(666));
    }

    #[test]
    fn writer_creates_directory() {
        let dir = TempDir::new("below_store_test").expect("tempdir failed");
        let mut subdir = dir.path().to_path_buf();
        subdir.push("foo");
        let ts = SystemTime::now();
        {
            let mut writer = StoreWriter::new(&subdir).expect("Failed to create store");
            let mut frame = DataFrame::default();
            frame.sample.cgroup.memory_current = Some(333);

            writer.put(ts, &frame).expect("Failed to store data");
        }

        let frame = read_next_sample(&subdir, ts, Direction::Forward, get_logger())
            .expect("Failed to read sample")
            .expect("Did not find stored sample");
        assert_ts!(frame.0, ts);
        assert_eq!(frame.1.sample.cgroup.memory_current, Some(333));
    }
}
