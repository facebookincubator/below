## What is `below dump`?

`below dump` is a below subcommand that helps a user to dump the historical data below collected to stdout or files in desired format. It is designed to be scripting friendly.

## How to use `below dump`?

### Basic usage

* Dump the system stats from 10:00AM to 10:10AM in JSON format.

```bash
$ below dump system -b "10:00" -e "10:10" --output-format json
```
* Dump the system stats from 10 minutes and 20 second ago to 10minutes ago in JSON format.

```bash
$ below dump system -b 10m20s -e 10m -O json
```
* Dump the remote host’s stats from 10:00AM to 10:10AM in CSV format to a file.

```bash
$ below dump --host HOSTNAME system -b "10:00" -e "10:10" -O csv -o output.csv
```

### Dump only the data you interested in with `-f` or `--fields` option:

* Dump the system `cpu_usage` and `io_read` stats from 10:00AM to 10:10AM in JSON format. Available fields can be found with `below dump SUBCOMMAND --help`. They are listed in the  `Available Fields` section.

```bash
$ below dump system -b "10:00" -e "10:10" -O json -f cpu_usage io_read
```
* Dump all of `cpu` and `io` related stats from 10:00AM to 10:10AM in JSON format. The `cpu` and `io` here are called aggregated fields which indicates a group of `available fields`. You can find aggregated fields in the `Aggregated fields` section inside `below dump SUBCOMMAND --help`.

```bash
$ below dump system -b "10:00" -e "10:10" -O json -f cpu io
```

### Control your dump output with selector `-s` or `--select`:

* Dump the process “below” stats from 10:00 AM to 10:10 AM in JSON format. Here we use `-s`to select a field and use `--filter` or `-f` to apply a filter on this field.

```bash
$ below dump process -b "10:00" -e "10:10" -O json -s comm --filter below*
```
* Dump the process stats from 10:00 AM to 10:10 AM in JSON format and sort the output by the CPU usage from hight to low.

```bash
$ below dump process -b "10:00" -e "10:10" -O json -s cpu_total --rsort
```
* Dump the process stats from 10:00 AM to 10:10 AM in JSON format, for each timestamp, output the top 5 CPU intense processes.

```bash
$ below dump process -b "10:00" -e "10:10" -O json -s cpu_total --rsort --top 5
```

## Use `belowrc` file for saving customized dump pattern

See `belowrc.md`.
