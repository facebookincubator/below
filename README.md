# below

[![CI](https://github.com/facebookincubator/below/workflows/CI/badge.svg)](https://github.com/facebookincubator/below/actions?query=workflow%3ACI+branch%3Amaster+)
[![Matrix chat](https://img.shields.io/matrix/below:matrix.org)](https://matrix.to/#/!SrWxtbLuRUMrDbftgA:matrix.org?via=matrix.org)

`below` is an interactive tool to view and record historical system data. It
has support for:

* information regarding hardware resource utilization
* viewing the cgroup hierarchy
* cgroup and process information
* pressure stall information (PSI)
* `record` mode to record system data
* `replay` mode to replay historical system data
* `live` mode to view live system data
* `dump` subcommand to report script-friendly information (eg JSON and CSV)

below does **not** have support for cgroup1.

The name "below" stems from the fact that the below developers rejected many
of [atop](https://linux.die.net/man/1/atop)'s design and style decisions.

## Demo

<a href="https://asciinema.org/a/355506">
<img src="https://asciinema.org/a/355506.svg" width="500">
</a>

## Comparison with alternative tools

See [comparison.md](resctl/below/docs/comparison.md) for a feature comparison
with alternative tools.

## Installing

```shell
$ cargo install below
$ below --help
```

For convenience, we also provide a Dockerfile and
[pre-built images](https://hub.docker.com/r/below/below) on Docker Hub.
See [docker.md](resctl/below/docs/docker.md) for how to use them.

Alternatively, see [building.md](resctl/below/docs/building.md) for non-docker
build instructions.

Live view of system:

```shell
$ sudo below live
```

Run recording daemon:

```shell
$ sudo cp ~/.cargo/bin/below /bin/below  # if using cargo-install
$ sudo cp resctl/below/etc/below.service /etc/systemd/system
$ sudo systemctl daemon-reload
$ sudo systemctl start below
```

Replay historical data:

```shell
$ below replay -t "3m ago"
```

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) file for how to help out.

## License

See [LICENSE](LICENSE) file.
