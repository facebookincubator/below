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

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /rustup.sh
RUN chmod +x /rustup.sh
RUN bash /rustup.sh -y

ADD . /resctl
# Build below
WORKDIR resctl
RUN /root/.cargo/bin/cargo build --release --package below

# Run tests if requested
RUN if [ -n "$RUN_TESTS" ]; then                 \
    /root/.cargo/bin/cargo test --               \
    --skip test_dump                             \
    --skip advance_forward_and_reverse           \
    --skip disable_disk_stat                     \
    --skip disable_io_stat                       \
    --skip record_replay_integration             \
    --skip test_viewrc_collapse_cgroups          \
    --skip test_viewrc_default_view              \
    --skip test_belowrc_to_event                 \
    --skip test_event_controller_override_failed \
    --skip test_event_controller_override        \
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
