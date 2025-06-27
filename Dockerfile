# syntax=docker/dockerfile:1
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

FROM fedora:42 AS builder
# NOTE: `clang` is required by `libbpf-cargo` for building `below/src/bpf/exitstat.bpf.c`
RUN dnf install -yq cargo clang elfutils-libelf-devel && dnf clean all
WORKDIR /app
# Only copy over files/dirs needed for the build:
COPY Cargo.lock Cargo.toml .
COPY below/ below/
RUN <<HEREDOC
    cargo build --release --bin below
    cp target/release/below /usr/local/bin/below
    strip /usr/local/bin/below
HEREDOC

# Provides a minimal base for the runtime image to use:
FROM fedora:42 AS root-fs
RUN <<HEREDOC
    dnf --installroot /root-fs --use-host-config --setopt=install_weak_deps=0 \
        install -yq elfutils-libelf glibc libgcc libzstd zlib-ng-compat

    # Remove DNF cache (almost 100MB):
    dnf --installroot /root-fs --use-host-config --setopt=install_weak_deps=0 \
        clean all
HEREDOC

# Compose the final minimal image to publish:
FROM scratch AS runtime
COPY --link --from=root-fs /root-fs /
COPY --link --from=builder /usr/local/bin/below /below
ENTRYPOINT ["/below"]
