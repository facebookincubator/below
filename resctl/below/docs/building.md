# Dependencies

Several dependencies need to be installed before we can build.

* libbpf-dev (for headers)
* libz (dynamically linked)
* libelf (dynamically linked)
* libncursesw (dynamically linked)

# Building

Below's UI is quite laggy in debug builds. We recommend always building in
release mode.

In the root of the repository:

```shell
$ cargo install libbpf-cargo
$ cargo libbpf make -- --release
$ cargo test
```

`cargo libbpf make` is a convenience wrapper for the BPF components. Alternatively,
you could do:

```shell
$ cargo install libbpf-cargo
$ cargo libbpf build
$ cargo libbpf gen
$ cargo build --release
$ cargo test
```
