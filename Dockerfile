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
ENV PATH="/root/.cargo/bin:${PATH}"
RUN <<HEREDOC
    # `clang` is required by `libbpf-cargo` for building `below/src/bpf/exitstat.bpf.c`
    # `zig` + `cargo-zigbuild` allows for easier cross-compilation + building with broader glibc support
    dnf install -yq clang elfutils-libelf-devel rustup zig
    dnf clean all

    # `libbpf-cargo` requires the `rustfmt` component to generate `exitstat.skel.rs`
    rustup-init -y --profile minimal --default-toolchain stable --component rustfmt
    cargo install cargo-zigbuild
HEREDOC
WORKDIR /app
# Only copy over files/dirs needed for the build:
COPY Cargo.lock Cargo.toml .
COPY below/ below/
RUN <<HEREDOC
    # Building with `cargo zigbuild` excludes the standard system paths, add them back:
    # - CFLAGS is needed as a includes fallback for discovering headers from installed packages.
    # - RUSTFLAGS is needed for crates with `build.rs` scripts to include the search path for linking libs.
    export CFLAGS='-isystem /usr/include'
    export RUSTFLAGS='-L /usr/lib64'
    cargo zigbuild --bin below --release --target "$(uname -m)-unknown-linux-gnu.2.34"

    cp "target/$(uname -m)-unknown-linux-gnu/release/below" /usr/local/bin/below
    strip /usr/local/bin/below
HEREDOC

# Support for `scripts/build_deb.sh`:
FROM builder AS package-deb
RUN cargo install cargo-deb
COPY README.md .
COPY etc/ .
RUN cargo deb --package below --no-build --target "$(uname -m)-unknown-linux-gnu"

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
