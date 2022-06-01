# Dependencies

Several dependencies need to be installed before we can build.

* libz (dynamically linked)
* libelf (dynamically linked)
* libncursesw (dynamically linked)
* clang (for building BPF program at build time)

## Install build dependencies

### Ubuntu

```
# For Focal (20.04 LTS) or Jammy (22.04 LTS)
sudo apt install -y build-essential ca-certificates clang curl git \
  libelf-dev libncursesw5-dev libssl-dev m4 pkg-config python3 zlib1g-dev
```

# Building

Below's UI is quite laggy in debug builds. We recommend always building in
release mode.

In the root of the repository:

```shell
$ cargo build --release
$ cargo test
```
