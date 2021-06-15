# Dependencies

Several dependencies need to be installed before we can build.

* libbpf-dev (for headers)
* libz (dynamically linked)
* libelf (dynamically linked)
* libncursesw (dynamically linked)
* clang (for building BPF program at build time)

# Building

Below's UI is quite laggy in debug builds. We recommend always building in
release mode.

In the root of the repository:

```shell
$ cargo build --release
$ cargo test
```
