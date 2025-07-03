#!/bin/bash
#
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

set -eu

# cd to project root
cd "$(dirname "$(realpath "$0")")"/..

# Build `below` and create a `.deb` package:
docker build --tag localhost/below:packaged --target package-deb .
# Copy the `.deb` package to the host filesystem which you can then install via `dpkg -i below_*.deb`:
docker run --rm -it --volume "$(pwd):/output:Z" localhost/below:packaged /bin/bash -c 'cp target/debian/below_*.deb /output'

echo "Debian package copied to $(pwd)"
