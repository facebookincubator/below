#!/bin/bash
#
# This script spits out an ubuntu .deb package for below.
#
# For example, to build a .deb for ubuntu 18.04:
#
#     ./build_deb.sh 18.04

set -eu

# cd to project root
cd "$(dirname "$(realpath "$0")")"/..

if [[ $# != 1 ]]; then
  echo "usage: ./build_deb.sh RELEASE" 1>&2
  exit 1
fi

docker build -f Dockerfile.debian --build-arg RELEASE="$1" -t below-deb .
docker run -v $(pwd):/output below-deb /bin/bash -c "cp /below/target/debian/below_*.deb /output"

echo Debian package copied to $(pwd)
