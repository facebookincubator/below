FROM ubuntu:groovy AS build

RUN apt-get update
RUN apt-get install -y \
  build-essential \
  ca-certificates \
  clang \
  curl \
  git \
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

ADD . /below
# Build below
WORKDIR below
RUN /root/.cargo/bin/cargo build --release --tests

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

COPY --from=build /below/target/release/below /below

ENTRYPOINT ["/below"]
