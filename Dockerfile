FROM ubuntu:groovy AS build

ARG RUN_TESTS

RUN apt-get update
RUN apt-get install -y \
  build-essential \
  ca-certificates \
  clang \
  curl \
  git \
  libbpf-dev \
  libelf-dev \
  libncursesw5-dev \
  libssl-dev \
  m4 \
  python3 \
  zlib1g-dev

WORKDIR /
ADD . /resctl

# Build and install fbthrift
RUN mkdir /fbthrift

RUN /resctl/build/fbcode_builder/getdeps.py build fbthrift --install-prefix /fbthrift

# Must set THRIFT env var for build to find fbthrift installation
ENV THRIFT /fbthrift/fbthrift/bin/thrift1

# Install nightly rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /rustup.sh
RUN chmod +x /rustup.sh
RUN bash /rustup.sh -y --default-toolchain nightly

# Install libbpf-rs tooling
RUN /root/.cargo/bin/cargo install libbpf-cargo

# Build below
WORKDIR resctl
RUN /root/.cargo/bin/cargo libbpf make -- --release --package below

# Run tests if requested
RUN if [[ -n "$RUN_TESTS" ]]; then     \
    /root/.cargo/bin/cargo test --     \
    --skip test_dump                   \
    --skip advance_forward_and_reverse \
    --skip disable_disk_stat           \
    stdout                             \
    --skip disable_io_stat;            \
  fi

# Now create stage 2 image. We drop all the build dependencies and only install
# runtime dependencies. This will create a smaller image suitable for
# distribution.

FROM ubuntu:groovy

# Default locale is "POSIX" which doesn't seem to play well with UTF-8. Cursive
# uses UTF-8 to draw lines so we need to set this locale otherwise our lines
# will be garbled. See https://github.com/gyscos/cursive/issues/13 .
ENV LANG C.UTF-8

RUN apt-get update
RUN apt-get install -y \
  libelf1 \
  libncursesw5 \
  zlib1g

COPY --from=build /resctl/target/release/below /below

ENTRYPOINT ["/below"]
