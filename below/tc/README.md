# tc

`tc` is a rust library that parses the queueing discipline ([qdisc](https://tldp.org/HOWTO/Traffic-Control-HOWTO/components.html#c-qdisc)) component of the Traffic Control ([`tc`](http://man7.org/linux/man-pages/man8/tc.8.html)) Linux subsystem.

The library depends on another upstream library https://github.com/rust-netlink/netlink-packet-route for parsing the [netlink](https://www.kernel.org/doc/html/latest/userspace-api/netlink/intro.html) response which is then converted into an intermediate representation to be consumed by the `model` library.
