# TL;DR

* use rust's nightly toolchain
* fbthrift must be installed
* `cargo build` and `cargo test` to build and test

# Dependencies

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

```
[resctl]$ cargo build
[resctl]$ cargo test
```
