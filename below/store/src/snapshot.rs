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

use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use tar::Archive;
use tempfile::TempDir;

use crate::DataFrame;
use crate::Direction;
use crate::LocalStore;
use crate::Store;

/// The store in a snapshot tarball made by `below snapshot`, unpacked to a
/// temporary directory that is removed when this is dropped.
pub struct SnapshotStore {
    store: LocalStore,
    _dir: TempDir,
}

impl SnapshotStore {
    pub fn open(logger: slog::Logger, tarball: &Path) -> Result<Self> {
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
        Ok(Self {
            store: LocalStore::new(logger, path),
            _dir: dir,
        })
    }
}

impl Store for SnapshotStore {
    type SampleType = DataFrame;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        self.store.get_sample_at_timestamp(timestamp, direction)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use common::util::get_unix_timestamp;

    use super::*;
    use crate::CompressionMode;
    use crate::Format;
    use crate::StoreWriter;

    fn logger() -> slog::Logger {
        slog::Logger::root(slog::Discard, slog::o!())
    }

    /// A snapshot tarball, laid out like `below snapshot` writes one, of a
    /// store with samples at `timestamps`.
    fn tarball(dir: &Path, timestamps: &[u64]) -> std::path::PathBuf {
        let store = TempDir::with_prefix("below_snapshot_store_test.").expect("tempdir failed");
        let mut writer =
            StoreWriter::new(logger(), store.path(), CompressionMode::None, Format::Cbor)
                .expect("failed to create store");
        for timestamp in timestamps {
            writer
                .put(
                    SystemTime::UNIX_EPOCH + Duration::from_secs(*timestamp),
                    &DataFrame::default(),
                )
                .expect("failed to write sample");
        }
        let path = dir.join(format!("snapshot_{}.tar", timestamps[0]));
        let mut builder = tar::Builder::new(File::create(&path).expect("failed to create tarball"));
        builder
            .append_dir_all("store", store.path())
            .expect("failed to add store");
        builder.finish().expect("failed to write tarball");
        path
    }

    #[test]
    fn unpacked_store_is_read_and_removed_when_dropped() {
        let dir = TempDir::with_prefix("below_snapshot_test.").expect("tempdir failed");
        let mut snapshot = SnapshotStore::open(logger(), &tarball(dir.path(), &[100, 105]))
            .expect("failed to open snapshot");
        let (timestamp, _) = snapshot
            .get_sample_at_timestamp(SystemTime::UNIX_EPOCH, Direction::Forward)
            .expect("failed to read")
            .expect("no sample");
        assert_eq!(get_unix_timestamp(timestamp), 100);

        let unpacked = snapshot._dir.path().to_path_buf();
        assert!(unpacked.exists());
        drop(snapshot);
        assert!(!unpacked.exists(), "{} was not removed", unpacked.display());
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
            let err = SnapshotStore::open(logger(), &tarball)
                .err()
                .expect("a tarball without exactly one directory should be rejected");
            assert!(
                err.to_string().contains("must hold exactly one directory"),
                "{err}"
            );
        }
    }
}
