# Comparison with alternative tools

Missing anything? File an
[issue](https://github.com/facebookincubator/below/issues).

## Atop

https://www.atoptool.nl/

* Terminal interface
  * Supports sorting, scrolling
* In-depth host-level stats
* Built in support for persisting/replaying historical data
* `atopsar` for scriptable access to historical data

### Drawbacks

* No cgroup awareness
* May suffer priority inversions while host is under resource contention
* On-disk data has custom delta compression and may be corrupted easily

## cAdvisor

https://github.com/google/cadvisor

* Web interface
* REST API for access to current data
* Tight integration with containers and container runtimes (eg docker)
* Reports high level stats on containers
* Plugin architecture for persisting historical data

### Drawbacks

* Requires external storage service to persist historical data
* cgroup1 only
* Limited support for host level stats
* No built in terminal interface

## htop

https://htop.dev/

* Wonderful terminal interface
  * Supports searching, filtering, sorting, scrolling
* Process tree view
* Supports system actions (killing processes, `strace`ing processes)

### Drawbacks

* No support for persisting/replaying historical data
* No cgroup awareness
* Limited host-level stats

## sysstat (sar)

https://github.com/sysstat/sysstat

* CLI-based tools like `sar`, `iostat`, `pidstat`, and others  
* Collects a wide range of system performance metrics (CPU, memory, IO, etc.)  
* Mature and widely used for historical performance tracking  
* Data can be collected periodically via `cron` or systemd timers  
* Data stored in a binary format, accessible through CLI  

### Drawbacks

* No interactive or web interface  
* Lacks container or cgroup-specific visibility  
* Requires parsing and scripting to extract structured data  
* Binary logs are not human-readable without `sar`  

## collectd

https://collectd.org/

* Daemon-based metrics collector  
* Plugin architecture for system and application metrics (disk, memory, network, etc.)  
* Can push metrics to various backends (Graphite, InfluxDB, Prometheus via exporter)  
* Lightweight and extensible with community plugins  
* Suitable for long-term storage and visualization setups  

### Drawbacks

* No built-in UI or terminal interface  
* Configuration can be complex  
* Requires external tooling for data visualization  
* No direct cgroup or container integration by default  

## below

https://github.com/facebookincubator/below

* Terminal interface
  * Supports filtering, zooming, pausing, sorting, scrolling
* Built in support for persisting/replaying historical data
* In-depth host-level stats
* cgroup awareness with cgroup tree view
* `below dump` for scriptable access to historical data
* Goes to great lengths to avoid priority inversions during host resource
  contention

### Drawbacks

* cgroup2 only
* No built in data compression (recommends btrfs transparent compression)
