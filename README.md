# resctl

resctl is an umbrella repository for **res**ource **c**on**t**ro**l** projects
at Facebook.

## Dependencies

- Fairly modern Linux kernel (4+)
- [Rust](https://www.rust-lang.org/)
- cgroup2

## Building

### TL;DR

* use rust's nightly toolchain
* fbthrift must be installed
* `cargo build` and `cargo test` to build and test

### fbthrift

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

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) file for how to help out.

## License

See [LICENSE](LICENSE) file.
