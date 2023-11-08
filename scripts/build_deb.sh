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


# This script spits out an ubuntu .deb package for below.
#
# For example, to build a .deb for ubuntu 18.04:
#
#     ./build_deb.sh 18.04
#

set -eu

# cd to project root
cd "$(dirname "$(realpath "$0")")"/..

if [[ $# != 1 ]]; then
  echo "usage: ./build_deb.sh RELEASE" 1>&2
  exit 1
fi

docker build -f Dockerfile.debian --build-arg RELEASE="$1" -t below-deb .
docker run -i -v $(pwd):/output -e RELEASE="$1" below-deb /bin/bash <<'EOF'
  # Hacky way to select a file by glob but still append a suffix
  cd /below/target/debian
  for d in ./below_*.deb; do
    cp $d /output/${d%.deb}_${RELEASE}.deb
  done
EOF

echo Debian package copied to $(pwd)
