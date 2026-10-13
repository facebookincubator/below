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

use std::cmp::Reverse;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use common::util::get_unix_timestamp;
use tar::Archive;
use tempfile::TempDir;

use crate::DataFrame;
use crate::Direction;
use crate::LocalStore;
use crate::Store;
use crate::cursor::Cursor;
use crate::cursor::StoreCursor;

/// The stores in snapshot tarballs made by `below snapshot`, read as one, so
/// snapshots of adjacent or overlapping time ranges of a host read like its
/// store. Each tarball is unpacked to a temporary directory that is removed
/// when this is dropped.
pub struct SnapshotStore {
    snapshots: Vec<Snapshot>,
}

struct Snapshot {
    /// Epoch seconds of the first and last sample.
    first: u64,
    last: u64,
    store: LocalStore,
    _dir: TempDir,
}

impl SnapshotStore {
    pub fn open(logger: slog::Logger, tarballs: &[impl AsRef<Path>]) -> Result<Self> {
        let snapshots = tarballs
            .iter()
            .map(|tarball| Snapshot::open(logger.clone(), tarball.as_ref()))
            .collect::<Result<_>>()?;
        Ok(Self { snapshots })
    }
}

impl Snapshot {
    fn open(logger: slog::Logger, tarball: &Path) -> Result<Self> {
        let dir = TempDir::with_prefix("snapshot_replay.")?;
        Archive::new(File::open(tarball).context("Failed to open snapshot file")?)
            .unpack(dir.path())
            .context("Failed to unpack snapshot file")?;
        let entries = fs::read_dir(dir.path())?.collect::<io::Result<Vec<_>>>()?;
        let path = match entries.as_slice() {
            [entry] if entry.file_type()?.is_dir() => entry.path(),
            _ => bail!(
                "Snapshot file {} must hold exactly one directory (the store)",
                tarball.display()
            ),
        };
        // A snapshot written without uploading ends with the empty shard the
        // writer creates for the day it was taken, which the cursor would warn
        // about.
        let quiet = slog::Logger::root(slog::Discard, slog::o!());
        let bound = |direction| {
            StoreCursor::new(quiet.clone(), path.clone())
                .next(direction)
                .map(|sample| sample.map(|(timestamp, _)| get_unix_timestamp(timestamp)))
        };
        let (Some(first), Some(last)) = (bound(Direction::Forward)?, bound(Direction::Reverse)?)
        else {
            bail!("Snapshot file {} has no samples", tarball.display());
        };
        Ok(Self {
            first,
            last,
            store: LocalStore::new(logger, path),
            _dir: dir,
        })
    }
}

impl Store for SnapshotStore {
    type SampleType = DataFrame;

    /// Reads only the snapshot whose samples continue soonest in `direction`
    /// from `timestamp`. Where snapshots of a host overlap they hold the same
    /// samples, being copies of its store, so the nearest sample is in that
    /// one. Ties go to the snapshot listed first.
    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        let t = get_unix_timestamp(timestamp);
        let snapshots = self.snapshots.iter_mut();
        let nearest = match direction {
            Direction::Forward => snapshots
                .filter(|s| s.last >= t)
                .min_by_key(|s| s.first.max(t)),
            Direction::Reverse => snapshots
                .filter(|s| s.first <= t)
                .min_by_key(|s| Reverse(s.last.min(t))),
        };
        let Some(snapshot) = nearest else {
            return Ok(None);
        };
        // Reading within the snapshot's range gives the same sample, and keeps
        // its cursor from warning about a day it lacks.
        let t = t.max(snapshot.first).min(snapshot.last);
        snapshot
            .store
            .get_sample_at_timestamp(SystemTime::UNIX_EPOCH + Duration::from_secs(t), direction)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::CompressionMode;
    use crate::Format;
    use crate::StoreWriter;

    fn logger() -> slog::Logger {
        slog::Logger::root(slog::Discard, slog::o!())
    }

    fn time(timestamp: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp)
    }

    /// A tarball laid out like `below snapshot` writes one, holding a store
    /// with samples at `timestamps`, each tagged with `name` as its hostname.
    fn tarball(dir: &Path, name: &str, timestamps: &[u64]) -> PathBuf {
        let store = TempDir::with_prefix("below_snapshot_store_test.").expect("tempdir failed");
        let mut writer =
            StoreWriter::new(logger(), store.path(), CompressionMode::None, Format::Cbor)
                .expect("failed to create store");
        for timestamp in timestamps {
            let mut frame = DataFrame::default();
            frame.sample.system.hostname = name.to_owned();
            writer
                .put(time(*timestamp), &frame)
                .expect("failed to write sample");
        }
        let path = dir.join(format!("{name}.tar"));
        let mut builder = tar::Builder::new(File::create(&path).expect("failed to create tarball"));
        builder
            .append_dir_all("store", store.path())
            .expect("failed to add store");
        builder.finish().expect("failed to write tarball");
        path
    }

    /// Every sample from `start` in `direction`, as (epoch seconds, tag).
    fn read_all(store: &mut SnapshotStore, start: u64, direction: Direction) -> Vec<(u64, String)> {
        let mut samples = Vec::new();
        let mut next = time(start);
        while let Some((timestamp, frame)) = store
            .get_sample_at_timestamp(next, direction)
            .expect("failed to read")
        {
            samples.push((get_unix_timestamp(timestamp), frame.sample.system.hostname));
            next = match direction {
                Direction::Forward => timestamp + Duration::from_secs(1),
                Direction::Reverse => timestamp - Duration::from_secs(1),
            };
        }
        samples
    }

    #[test]
    fn overlapping_and_disjoint_snapshots_read_as_one() {
        let dir = TempDir::with_prefix("below_snapshot_test.").expect("tempdir failed");
        // `late` is listed before `early` to show the order of the list only
        // breaks ties.
        let mut store = SnapshotStore::open(
            logger(),
            &[
                tarball(dir.path(), "late", &[200, 205]),
                tarball(dir.path(), "early", &[100, 105, 110, 115]),
                tarball(dir.path(), "overlap", &[110, 115, 120, 125]),
            ],
        )
        .expect("failed to open snapshots");

        let s = |timestamp: u64, name: &str| (timestamp, name.to_owned());
        let forward = vec![
            s(100, "early"),
            s(105, "early"),
            s(110, "early"),
            // In both, and `early` is listed first.
            s(115, "early"),
            s(120, "overlap"),
            s(125, "overlap"),
            // Across the gap.
            s(200, "late"),
            s(205, "late"),
        ];
        assert_eq!(read_all(&mut store, 0, Direction::Forward), forward);
        let reverse = vec![
            s(205, "late"),
            s(200, "late"),
            s(125, "overlap"),
            s(120, "overlap"),
            // Read from `overlap`, the one reaching closer to 119.
            s(115, "overlap"),
            s(110, "early"),
            s(105, "early"),
            s(100, "early"),
        ];
        assert_eq!(read_all(&mut store, 1000, Direction::Reverse), reverse);

        assert_eq!(
            read_all(&mut store, 150, Direction::Forward)[0],
            s(200, "late")
        );
        assert_eq!(
            read_all(&mut store, 150, Direction::Reverse)[0],
            s(125, "overlap")
        );
        assert!(read_all(&mut store, 206, Direction::Forward).is_empty());
        assert!(read_all(&mut store, 99, Direction::Reverse).is_empty());

        assert!(
            SnapshotStore::open(logger(), &[tarball(dir.path(), "empty", &[])]).is_err(),
            "a snapshot without samples should be rejected"
        );
    }

    #[test]
    fn unpacked_stores_are_removed_when_dropped() {
        let dir = TempDir::with_prefix("below_snapshot_test.").expect("tempdir failed");
        let store = SnapshotStore::open(
            logger(),
            &[
                tarball(dir.path(), "a", &[100, 105]),
                tarball(dir.path(), "b", &[110]),
            ],
        )
        .expect("failed to open snapshots");
        let unpacked = store
            .snapshots
            .iter()
            .map(|s| s._dir.path().to_path_buf())
            .collect::<Vec<_>>();
        assert_eq!(unpacked.len(), 2);
        assert!(unpacked.iter().all(|dir| dir.exists()));
        drop(store);
        assert!(unpacked.iter().all(|dir| !dir.exists()), "{unpacked:?}");
    }

    #[test]
    fn tarball_must_hold_one_directory() {
        let dir = TempDir::with_prefix("below_snapshot_test.").expect("tempdir failed");
        let path = dir.path().join("two.tar");
        let mut builder = tar::Builder::new(File::create(&path).expect("failed to create tarball"));
        for name in ["a", "b"] {
            builder
                .append_dir(name, dir.path())
                .expect("failed to add directory");
        }
        builder.finish().expect("failed to write tarball");
        let file = dir.path().join("file.tar");
        let mut builder = tar::Builder::new(File::create(&file).expect("failed to create tarball"));
        builder
            .append_path_with_name(&path, "store")
            .expect("failed to add file");
        builder.finish().expect("failed to write tarball");

        for tarball in [path, file] {
            let err = SnapshotStore::open(logger(), &[tarball])
                .err()
                .expect("a tarball without exactly one directory should be rejected");
            assert!(
                err.to_string().contains("must hold exactly one directory"),
                "{err}"
            );
        }
    }
}
