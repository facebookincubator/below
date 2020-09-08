FROM ubuntu:latest

# Default locale is "POSIX" which doesn't seem to play well with UTF-8. Cursive
# uses UTF-8 to draw lines so we need to set this locale otherwise our lines
# will be garbled. See https://github.com/gyscos/cursive/issues/13 .
ENV LANG C.UTF-8

RUN apt-get update
RUN apt-get install -y \
  build-essential \
  ca-certificates \
  curl \
  git \
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

# Build below
WORKDIR resctl
RUN /root/.cargo/bin/cargo build --release --package below

ENTRYPOINT ["/resctl/target/release/below"]
