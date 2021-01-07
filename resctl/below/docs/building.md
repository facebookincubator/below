# TL;DR

* use rust's nightly toolchain
* fbthrift must be installed
* `cargo build` and `cargo test` to build and test

# Dependencies

* libbpf-dev (for headers)
* libz (dynamically linked)
* libelf (dynamically linked)
* libncursesw (dynamically linked)
* fbthrift (see below)

## fbthrift

We use fbthrift for data serialization as well as (in the future) remote
viewing.

Install inside `$HOME/.thrift` do:
```
[resctl]$ mkdir $HOME/.thrift
[resctl]$ ./build/fbcode_builder/getdeps.py build fbthrift --install-prefix $HOME/.thrift
```

After that add `THRIFT=$HOME/.thrift/fbthrift/bin/thrift1` to your environment
or make sure `thrift1` is accessible by adding `$HOME/.thrift/fbthrift/bin` to
`PATH`.

# Building

Below's UI is quite laggy in debug builds. We recommend always building in
release mode.

```
[resctl]$ cargo install libbpf-cargo
[resctl]$ cargo libbpf make -- --release
[resctl]$ cargo test
```

`cargo libbpf make` is a convenience wrapper for the BPF components. Alternatively,
you could do:

```
[resctl]$ cargo install libbpf-cargo
[resctl]$ cargo libbpf build
[resctl]$ cargo libbpf gen
[resctl]$ cargo build --release
[resctl]$ cargo test
```
