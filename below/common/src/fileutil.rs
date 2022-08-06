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

use std::path::Path;

use walkdir::WalkDir;

// Get directory size recursively
pub fn get_dir_size<P: AsRef<Path>>(path: P) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .fold(0, |acc, m| acc + m.len())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::File;
    use std::io::BufWriter;
    use std::io::Write;

    use tempdir::TempDir;

    use super::*;

    fn make_file<P: AsRef<Path>>(path: P, size: u64) {
        let file = File::create(&path).expect("Failed to create file");
        let mut writer = BufWriter::new(file);
        for _ in 0..size {
            writer.write_all(&[0]).expect("Failed to write");
        }
        writer.flush().expect("Failed to flush writer");
    }

    #[test]
    fn test_get_dir_size() {
        // Empty directory
        {
            let dir = TempDir::new("below_fileutil_test").expect("tempdir failed");
            let size = get_dir_size(dir.path());
            assert_eq!(size, 0);
        }
        // Directory with files and files in nested directories
        {
            let dir = TempDir::new("below_fileutil_test").expect("tempdir failed");
            fs::create_dir(dir.path().join("dir_A")).expect("Failed to create directory");
            fs::create_dir(dir.path().join("dir_B")).expect("Failed to create directory");
            make_file(dir.path().join("A"), 1000);
            make_file(dir.path().join("B"), 100);
            make_file(dir.path().join("dir_A/A"), 10);
            make_file(dir.path().join("dir_B/A"), 1);
            make_file(dir.path().join("dir_B/B"), 0);

            let size = get_dir_size(dir.path());
            assert_eq!(size, 1111);
        }
    }
}
