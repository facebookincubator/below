# resctl

[![CI](https://github.com/facebookincubator/resctl/workflows/CI/badge.svg)](https://github.com/facebookincubator/resctl/actions?query=workflow%3ACI+branch%3Amaster+)
[![Matrix chat](https://img.shields.io/matrix/below:matrix.org)](https://matrix.to/#/!SrWxtbLuRUMrDbftgA:matrix.org?via=matrix.org)

resctl is an umbrella repository for **res**ource **c**on**t**ro**l** projects
at Facebook.

## Projects

### below

`below` is an interactive tool to view and record historical system data. It
has support for:

* information regarding hardware resource utilization
* viewing the cgroup hierarchy
* cgroup and process information
* pressure stall information (PSI)
* `record` mode to record system data
* `replay` mode to replay historical system data
* `live` mode to view live system data
* `dump` subcommand to report script-friendly information (eg json and csv)

below does **not** have support for cgroup1.

The name "below" stems from the fact that the below developers rejected many
of [atop](https://linux.die.net/man/1/atop)'s design and style decisions.

#### Demo

<a href="https://asciinema.org/a/355506">
<img src="https://asciinema.org/a/355506.svg" width="500">
</a>

#### Comparison with alternative tools

See [comparison.md](resctl/below/docs/comparison.md) for a feature comparison
with alternative tools.

### procfs

`procfs` is a rust library that parses
[procfs](https://www.man7.org/linux/man-pages/man5/procfs.5.html) files.

### cgroupfs

`cgroupfs` is a rust library that parses
[cgroup2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
control files.

## Building

See [building.md](resctl/below/docs/building.md).

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) file for how to help out.

## License

See [LICENSE](LICENSE) file.
