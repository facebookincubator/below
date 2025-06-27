# Dependencies

Several dependencies need to be installed before we can build.

* libz (dynamically linked)
* libelf (dynamically linked)
* clang-15+ (for building BPF program at build time)
* rustfmt (used by libbpf-cargo for bpf skeleton code generation)

## Install build dependencies

### Ubuntu

```shell
# For Focal (20.04 LTS) or Jammy (22.04 LTS)
sudo apt install -y build-essential ca-certificates clang curl git \
  libelf-dev libssl-dev m4 pkg-config python3 zlib1g-dev
```

Also check that `rustfmt` is installed. If you are using rustup, it should be
installed by default, but if not, install with:

```shell
rustup component add rustfmt
```

# Building

Make sure clang-15 is installed and exported.

```shell
export CLANG=clang-15
```

Below's UI is quite laggy in debug builds. We recommend always building in
release mode.

In the root of the repository:

```shell
cargo build --release
cargo test
```
