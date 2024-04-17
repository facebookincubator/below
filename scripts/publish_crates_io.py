#!/bin/env python3

# Copyright (c) Facebook, Inc. and its affiliates.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.


import argparse
import pathlib
import subprocess
import sys
import time
from os import path

# Paths to crates relative to repository root
# NB: Order the list based on dependencies: least deps first, most deps last
PACKAGES = [
    "below/ethtool",
    "below/cgroupfs",
    "below/procfs",
    "below/common",
    "below/btrfs",
    "below/gpu_stats",
    "below/config",
    "below/below_derive",
    "below/resctrlfs",
    "below/model",
    "below/render",
    "below/store",
    "below/view",
    "below/dump",
    "below",
]


def is_logged_in():
    return path.exists(path.expanduser("~/.cargo/credentials"))


def get_repository_root():
    return pathlib.Path(__file__).parent.parent.absolute()


def publish_crate(crate_root, dry_run):
    args = []
    args.append("cargo")
    args.append("publish")
    if dry_run:
        args.append("--dry-run")
    args.append("--manifest-path")
    args.append(f"{crate_root}/Cargo.toml")

    subprocess.run(args, check=True)


def main(args):
    if not args.dry_run and not is_logged_in():
        print("Please run `cargo login` first and then retry")
        return 1

    print(
        """The script is a little hacky and not yet idempotent. So if any
crate upload fails you may need to delete already-uploaded packages
from `PACKAGES` and restart the script
"""
    )

    repo_root = get_repository_root()
    for package in PACKAGES:
        crate_root = f"{repo_root}/{package}"
        print(f"++ Publishing {crate_root}")
        publish_crate(crate_root, args.dry_run)

        # HACK: wait for crates.io to stabilize previously uploaded deps
        print("++ Waiting for crates.io to stabilize")
        time.sleep(30)

    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Publish to crates.io")
    parser.add_argument("--dry-run", action="store_true")
    sys.exit(main(parser.parse_args()))
