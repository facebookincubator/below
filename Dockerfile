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

FROM ubuntu:22.04 AS build

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
  build-essential \
  ca-certificates \
  clang \
  curl \
  git \
  libelf-dev \
  libncursesw5-dev \
  libssl-dev \
  m4 \
  pkg-config \
  python3 \
  zlib1g-dev

WORKDIR /

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /rustup.sh
RUN chmod +x /rustup.sh
RUN bash /rustup.sh -y

ADD . /below
# Build below
WORKDIR below
RUN /root/.cargo/bin/cargo build --release --all-targets

# Now create stage 2 image. We drop all the build dependencies and only install
# runtime dependencies. This will create a smaller image suitable for
# distribution.
FROM ubuntu:22.04

# Default locale is "POSIX" which doesn't seem to play well with UTF-8. Cursive
# uses UTF-8 to draw lines so we need to set this locale otherwise our lines
# will be garbled. See https://github.com/gyscos/cursive/issues/13 .
ENV LANG C.UTF-8

RUN apt-get update
RUN apt-get install -y \
  libelf1 \
  libncursesw5 \
  zlib1g

COPY --from=build /below/target/release/below /below

ENTRYPOINT ["/below"]
