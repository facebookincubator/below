Note the instructions use `podman` instead of `docker` because at time of
writing, docker doesn't yet have support for cgroup2.

# Prebuilt image

```shell
$ podman run --privileged --cgroupns=host --pid=host -it below/below:latest
```

# Local build

```shell
$ git clone https://github.com/facebookincubator/below.git ~/dev/below
<...>

$ cd ~/dev/below

$ podman build -t below .
<...>

$ podman run --privileged --cgroupns=host --pid=host -it below
```
